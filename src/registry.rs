use crate::world::archetypes::ArchetypeId;

pub struct RegistryCell {
    pub archetype_id: ArchetypeId,
    pub idx: u32,
    pub handle_count: std::sync::atomic::AtomicU32,
}

pub struct UnsafeGlobalRegistry(pub Vec<RegistryCell>);
unsafe impl Send for UnsafeGlobalRegistry {}
unsafe impl Sync for UnsafeGlobalRegistry {}

pub static mut REGISTRY: UnsafeGlobalRegistry = UnsafeGlobalRegistry(Vec::new());
