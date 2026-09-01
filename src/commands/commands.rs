use std::sync::Arc;

use fxhash::FxBuildHasher;
use papaya::{HashSet, HashSetRef, LocalGuard};
use parking_lot::{RawRwLock, RwLock, lock_api::RwLockReadGuard};

use crate::{
    commands::{
        DespawnCommand,
        bundle::ComponentBundle,
        command_queue::{CommandQueue, WorldCommand},
        command_types::{
            AddComponentsCommand, BatchSpawnCommand, InsertComponentsCommand,
            RemoveComponentsCommand, SpawnCommand,
        },
    },
    entity::Entity,
    system::validation::{FunctionData, ParamAccess, SystemParam},
    world::storage::World,
};

pub(crate) struct CommandBuffer {
    pub(crate) queue: Arc<RwLock<CommandQueue>>,
    pub(crate) despawns: Arc<HashSet<DespawnCommand, FxBuildHasher>>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(CommandQueue::new())),
            despawns: Arc::new(HashSet::with_hasher(FxBuildHasher::new())),
        }
    }
}

pub struct Commands<'a> {
    pub(crate) queue: parking_lot::RwLockReadGuard<'a, CommandQueue>,
    pub(crate) despawns: HashSetRef<'a, DespawnCommand, FxBuildHasher, LocalGuard<'a>>,
}

impl Commands<'_> {
    fn push<C: WorldCommand + 'static>(&mut self, command: C) {
        self.queue.push(command);
    }

    pub fn spawn<C: ComponentBundle + Send>(&mut self, components: C) {
        self.push(SpawnCommand { components });
    }

    pub fn spawn_batch<C, I>(&mut self, components_iter: I)
    where
        C: ComponentBundle + Send,
        I: IntoIterator<Item = C>,
        I::IntoIter: Send + 'static,
    {
        self.push(BatchSpawnCommand {
            components_iter: Box::new(components_iter.into_iter()),
        });
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.despawns.insert(DespawnCommand { entity });
    }

    pub fn add_components<C: ComponentBundle + Send>(&mut self, entity: Entity, components: C) {
        self.push(AddComponentsCommand { entity, components });
    }

    pub fn remove_components<C: ComponentBundle + Send>(&mut self, entity: Entity) {
        self.push(RemoveComponentsCommand::<C> {
            entity,
            _marker: std::marker::PhantomData,
        });
    }

    pub fn insert_components<C: ComponentBundle + Send>(&mut self, entity: Entity, components: C) {
        self.push(InsertComponentsCommand { entity, components });
    }

    pub fn despawn_iter(&self) -> impl Iterator<Item = &DespawnCommand> {
        self.despawns.iter()
    }

    pub(crate) fn push_fn<F>(&mut self, f: F)
    where
        F: FnOnce(&mut World) + Send + 'static,
    {
        self.queue.push_fn(f);
    }

    pub fn insert_resource<T: 'static + Send>(&mut self, resource: T) {
        self.push_fn(|world| world.insert_resource(resource));
    }

    pub fn remove_resource<T: 'static + Send>(&mut self) {
        self.push_fn(|world| {
            world.remove_resource::<T>();
        });
    }
}

impl<'a> SystemParam for Commands<'a> {
    fn get_access() -> ParamAccess {
        ParamAccess::default()
    }

    fn extract(world: &mut World, _data: &mut FunctionData) -> Self {
        let queue_local = world.commands.queue.read();
        let despawns_local = world.commands.despawns.pin();

        unsafe {
            let queue = std::mem::transmute::<
                RwLockReadGuard<'_, RawRwLock, CommandQueue>,
                RwLockReadGuard<'_, RawRwLock, CommandQueue>,
            >(queue_local);
            let despawns = std::mem::transmute::<
                HashSetRef<'_, DespawnCommand, FxBuildHasher, LocalGuard<'_>>,
                HashSetRef<'_, DespawnCommand, FxBuildHasher, LocalGuard<'_>>,
            >(despawns_local);

            Self { queue, despawns }
        }
    }
}
