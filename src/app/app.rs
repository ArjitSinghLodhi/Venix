use std::{
    any::TypeId,
    collections::{HashMap, HashSet, VecDeque},
};

use crate::{
    app::plugin::PluginsBuildAll,
    commands::commands::ComponentTuple,
    schedule::{
        schedule::{IntoScheduleId, Schedule, ScheduleId, ScheduleLabel, SchedulePlace},
        schedules_list::Startup,
    },
    system::validation::{IntoSystemConfigs, System},
    world::storage::World,
};

struct ConfigurationContext {
    plugins_processed: bool,
    schedules_added: bool,
    systems_added: bool,
    built: bool,
    ran_startup: bool,
}
impl ConfigurationContext {
    fn new() -> Self {
        Self {
            plugins_processed: false,
            schedules_added: false,
            systems_added: false,
            built: false,
            ran_startup: false,
        }
    }

    fn plugins_processed(&self) {
        if !self.plugins_processed {
            panic!("plugins Not processed when expected");
        }
    }

    fn schedules_added(&self) {
        if !self.schedules_added {
            panic!("Schedules not added and processed when expected");
        }
    }

    fn systems_added(&self) {
        if !self.systems_added {
            panic!("Systems not Added and Configured when expected");
        }
    }

    fn built(&self) {
        self.plugins_processed();
        self.schedules_added();
        self.systems_added();
        if !self.built {
            panic!("Configuration Somehow Not Built even After All Check: Engine problem Likely!");
        }
    }

    fn ran_startup(&self) {
        if !self.ran_startup {
            panic!("Startup not processed already when expected");
        }
    }
}

struct SystemsBlock {
    schedule_id: ScheduleId,
    systems: Vec<Box<dyn System>>,
}

pub struct App {
    pub(crate) world: World,
    schedules: Vec<Schedule>,
    plugins: Vec<Box<dyn PluginsBuildAll>>,
    total_systems_blocks: Vec<SystemsBlock>,
    runner_fn: fn(&mut App),
    configuration: ConfigurationContext,
}

impl App {
    pub fn new() -> Self {
        let mut schedules = Vec::new();
        schedules.push(Schedule::new(Startup));
        let function = |app: &mut App| {
            app.build();
            app.run_startup();
            loop {
                app.update();
            }
        };
        Self {
            world: World::new(),
            schedules: schedules,
            plugins: Vec::new(),
            total_systems_blocks: Vec::new(),
            runner_fn: function,
            configuration: ConfigurationContext::new(),
        }
    }

    pub fn add_schedule<T: ScheduleLabel + 'static>(&mut self, schedule: T) -> &mut Self {
        self.schedules.push(Schedule::new(schedule));
        self
    }

    pub fn add_systems<M>(
        &mut self,
        schedule: ScheduleId,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        let mut block = SystemsBlock {
            schedule_id: schedule,
            systems: Vec::new(),
        };
        systems.add_to_schedule(&mut block.systems);
        self.total_systems_blocks.push(block);
        self
    }

    pub fn insert_resource<T: 'static>(&mut self, resource: T) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    pub fn add_plugins<T: PluginsBuildAll + 'static>(&mut self, plugins: T) -> &mut Self {
        self.plugins.push(Box::new(plugins));
        self
    }

    pub fn run_schedule(&mut self, schedule_id: ScheduleId) {
        self.configuration.built();
        self.configuration.ran_startup();
        if schedule_id == Startup::id() {
            panic!("Please use run_startup() instead");
        }
        let schedule = self
            .schedules
            .iter_mut()
            .find(|s| s.schedule.id_from_self() == schedule_id)
            .expect("Schedule Not Registered when Running manual schedule through ScheduleId");
        schedule.run(&mut self.world);
    }

    pub fn build(&mut self) {
        if self.configuration.built {
            panic!("App Already Built");
        }
        self.build_everything();
        self.configuration.built = true;
    }

    pub fn run_startup(&mut self) {
        self.configuration.built();
        if self.configuration.ran_startup {
            panic!("Startup Already ran");
        }
        let mut startup = self.schedules.remove(0);
        startup.run(&mut self.world);
        self.configuration.ran_startup = true;
    }

    pub fn update(&mut self) {
        self.configuration.built();
        self.configuration.ran_startup();
        for schedule in self.schedules.iter_mut() {
            schedule.run(&mut self.world);
        }
        self.clear_changed_tracker();
    }

    pub fn run(&mut self) {
        let function = self.runner_fn;
        function(self);
    }

    pub fn set_runner(&mut self, function: fn(&mut App)) {
        self.runner_fn = function;
    }
}

impl App {
    pub fn get_resource<T: 'static>(&self) -> &T {
        self.world.get_resource::<T>()
    }

    pub fn get_resource_mut<T: 'static>(&mut self) -> &mut T {
        self.world.get_resource_mut::<T>()
    }

    pub fn pre_allocate_archetype<T: ComponentTuple>(&mut self) {
        self.world.pre_allocate_archetype::<T>();
    }

    pub fn apply_commands(&mut self) {
        self.world.apply_commands();
    }

    pub fn clear_changed_tracker(&mut self) {
        self.world.clear_change_trackers();
    }
}

impl App {
    fn build_everything(&mut self) {
        self.configure_plugins();
        self.configure_schedules();
        self.configure_systems();
    }

    fn configure_plugins(&mut self) {
        let mut seen_plugins = HashSet::new();

        while !self.plugins.is_empty() {
            let current_batch = std::mem::take(&mut self.plugins);
            for plugin_group in &current_batch {
                for name in plugin_group.get_plugin_names() {
                    if !seen_plugins.insert(name) {
                        panic!(
                            "Duplicate plugin detected! The plugin '{}' has already been registered.",
                            name
                        );
                    }
                }
            }
            for mut plugins_build_all in current_batch {
                plugins_build_all.build_all(self);
            }
        }

        self.configuration.plugins_processed = true;
    }

    fn configure_systems(&mut self) {
        for system_block in self.total_systems_blocks.drain(..) {
            let target_schedule = match self
                .schedules
                .iter_mut()
                .find(|s| s.schedule.id_from_self() == system_block.schedule_id)
            {
                Some(schedule) => schedule,
                None => {
                    let missing_name = system_block.schedule_id.name;
                    panic!(
                        "❌ CONFIGURATION ERROR: Attempted to add systems to Schedule '{}' which was never registered via add_schedule()!",
                        missing_name
                    );
                }
            };

            target_schedule
                .systems
                .extend(system_block.systems.into_iter());
        }
        self.configuration.systems_added = true;
    }

    fn configure_schedules(&mut self) {
        let unarranged = std::mem::take(&mut self.schedules);

        let mut schedule_map: HashMap<TypeId, Schedule> = HashMap::with_capacity(unarranged.len());
        for s in unarranged {
            let id = s.schedule.id_from_self();
            if schedule_map.contains_key(&id.id) {
                let name = s.schedule.name();
                panic!(
                    "❌ CONFIGURATION ERROR: Duplicate Schedule detected for '{}'!",
                    name
                );
            }
            schedule_map.insert(id.id, s);
        }

        let startup_id = TypeId::of::<Startup>();
        if !schedule_map.contains_key(&startup_id) {
            panic!(
                "❌ CONFIGURATION ERROR: Core 'Startup' Schedule is missing from the registration pool!"
            );
        }

        let mut adjacency_list: HashMap<TypeId, Vec<TypeId>> = HashMap::new();
        let mut in_degree: HashMap<TypeId, usize> = HashMap::new();
        let mut unique_edges: HashSet<(TypeId, TypeId)> = HashSet::new();

        for &id in schedule_map.keys() {
            in_degree.insert(id, 0);
            adjacency_list.entry(id).or_default();
        }

        for (id, schedule) in &schedule_map {
            let place = schedule.schedule.get_place();
            let current_schedule_name = schedule.schedule.name();

            let (u, v) = match place {
                SchedulePlace::Before(target) => {
                    let target_id = target.id;
                    if target_id == startup_id {
                        panic!(
                            "❌ CONFIGURATION ERROR: Custom Schedule '{}' cannot be placed Before(Startup). Startup must always remain the absolute structural root!",
                            current_schedule_name
                        );
                    }

                    if !schedule_map.contains_key(&target_id) {
                        panic!(
                            "❌ CONFIGURATION ERROR: Schedule '{}' attempts to run Before a target Schedule that was never registered!",
                            current_schedule_name
                        );
                    }
                    (*id, target_id)
                }
                SchedulePlace::After(target) => {
                    let target_id = target.id;
                    if !schedule_map.contains_key(&target_id) {
                        panic!(
                            "❌ CONFIGURATION ERROR: Schedule '{}' attempts to run After a target Schedule that was never registered!",
                            current_schedule_name
                        );
                    }
                    (target_id, *id)
                }
            };

            if unique_edges.insert((u, v)) {
                adjacency_list.entry(u).or_default().push(v);
                *in_degree.entry(v).or_default() += 1;
            }
        }

        let mut sorted_ids = Vec::with_capacity(schedule_map.len());
        sorted_ids.push(startup_id);

        if let Some(neighbors) = adjacency_list.get(&startup_id) {
            for &v in neighbors {
                if let Some(deg) = in_degree.get_mut(&v) {
                    if *deg > 0 {
                        *deg -= 1;
                    }
                }
            }
        }

        let mut queue: VecDeque<TypeId> = in_degree
            .iter()
            .filter(|&(&id, &deg)| deg == 0 && id != startup_id)
            .map(|(&id, _)| id)
            .collect();

        while let Some(u) = queue.pop_front() {
            sorted_ids.push(u);

            if let Some(neighbors) = adjacency_list.get(&u) {
                for &v in neighbors {
                    let deg = in_degree.get_mut(&v).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(v);
                    }
                }
            }
        }

        if sorted_ids.len() != schedule_map.len() {
            let mut trapped_schedules = Vec::new();
            for id in schedule_map.keys() {
                if !sorted_ids.contains(id) {
                    let s = &schedule_map[id];
                    trapped_schedules.push(s.schedule.name());
                }
            }
            panic!(
                "❌ CONFIGURATION ERROR: Circular dependency deadlock detected in Schedule constraints! Trapped Schedules: {:?}",
                trapped_schedules
            );
        }

        self.schedules = sorted_ids
            .into_iter()
            .map(|id| schedule_map.remove(&id).unwrap())
            .collect();
        self.configuration.schedules_added = true;
    }
}
