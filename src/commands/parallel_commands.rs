use fxhash::FxBuildHasher;
use papaya::HashSet;

use crate::commands::DespawnCommand;
use crate::commands::command_queue::CommandQueue;
use crate::commands::commands::Commands;
use crate::extensions::{ParamAccess, SystemParam};
use crate::system::validation::FunctionData;
use crate::world::storage::World;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct ParallelCommands {
    pub(crate) queue: Arc<RwLock<CommandQueue>>,
    pub(crate) despawns: Arc<HashSet<DespawnCommand, FxBuildHasher>>,
}

unsafe impl Send for ParallelCommands {}
unsafe impl Sync for ParallelCommands {}

impl SystemParam for ParallelCommands {
    fn get_access() -> ParamAccess {
        ParamAccess::default()
    }

    fn extract(world: &mut World, _data: &mut FunctionData) -> Self {
        Self {
            queue: world.commands.queue.clone(),
            despawns: world.commands.despawns.clone(),
        }
    }
}

impl ParallelCommands {
    pub fn scope<F, R>(&self, f: F) -> R
    where
        F: for<'b> FnOnce(Commands<'b>) -> R,
    {
        let commands = Commands {
            queue: self.queue.read().unwrap(),
            despawns: self.despawns.pin(),
        };
        f(commands)
    }
}
