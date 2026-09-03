use std::sync::Arc;

use fxhash::FxBuildHasher;
use papaya::HashSet;
use parking_lot::RwLock;

use crate::commands::command_queue::CommandQueue;
use crate::commands::commands::Commands;
use crate::entity::Entity;
use crate::extensions::{ParamAccess, SystemParam};
use crate::system::validation::FunctionData;
use crate::world::storage::World;

/// A thread-safe, thread-clonable handle that acts as a detached remote input into the ECS engine.
///
/// `ParallelCommands` can be passed to external or background worker threads, allowing them to
/// concurrently queue up structural modifications and despawn targets outside the main execution
/// path. It can be obtained directly from the application layer via [`.get_par_commands()`].
///
/// # Deferred Actions & Invariants
///
/// Any commands or despawn intents pushed through this handle remain subject to the engine's
/// core framework invariants:
///
/// * **Execution Timing:** Just like main-thread commands, actions queued here are entirely deferred
///   and will only execute during the next command execution phase.
/// * **Handle Violation Rule:** Any entity targets scheduled for destruction through this handle are
///   bound by the strict *Entity Despawn Invariant*. All active cloned handles for those targets must
///   be dropped across the frame layout before execution occurs.
///
/// [`.get_par_commands()`]: crate::app::App::get_par_commands
#[derive(Clone)]
pub struct ParallelCommands {
    pub(crate) queue: Arc<RwLock<CommandQueue>>,
    pub(crate) despawns: Arc<HashSet<Entity, FxBuildHasher>>,
}

impl ParallelCommands {
    /// Creates a temporary [`Commands`] context bound to the current scope block.
    ///
    /// This allows external or parallel tasks to safely issue commands using the engine's
    /// standard command syntax via an inner closure.
    pub fn scope<F, R>(&self, f: F) -> R
    where
        F: for<'b> FnOnce(Commands<'b>) -> R,
    {
        let commands = Commands {
            queue: self.queue.read(),
            despawns: self.despawns.pin(),
        };
        f(commands)
    }
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
