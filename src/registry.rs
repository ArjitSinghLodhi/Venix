use crate::world::archetypes::ArchetypeId;

pub(crate) struct RegistryCell {
    pub(crate) archetype_id: ArchetypeId,
    pub(crate) idx: u32,
    pub(crate) handle_count: std::sync::atomic::AtomicU32,
}

pub(crate) struct UnsafeGlobalRegistry(pub Vec<RegistryCell>);
unsafe impl Send for UnsafeGlobalRegistry {}
unsafe impl Sync for UnsafeGlobalRegistry {}

pub(crate) static mut REGISTRY: UnsafeGlobalRegistry = UnsafeGlobalRegistry(Vec::new());
