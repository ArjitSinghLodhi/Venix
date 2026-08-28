mod events;
mod parallel_events;

pub(crate) use events::{EventBuffer, TRACKED_EVENTS, register_event};

pub use events::{EventReader, EventWriter};
pub use parallel_events::{ParallelEventReader, ParallelEventWriter};
