use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

use crate::{
    entity::Entity,
    query::{
        filter::{EmptyFilter, Filter},
        params::WorldQuery,
    },
    registry::REGISTRY,
    system::validation::{AccessHashSet, FunctionData},
    world::{archetypes::Archetype, storage::World},
};

pub struct Mutable;
pub struct ReadOnly;

pub struct QuerySubChunk<'a, 'w, Q: WorldQuery, I> {
    safe_fetch: ThreadSafeFetch<Q>,
    sub_indices: &'a [usize],
    _marker: std::marker::PhantomData<&'w I>,
}

impl<'a, 'w, Q: WorldQuery, T> QuerySubChunk<'a, 'w, Q, T> {
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = Q::ReadOnlyItem<'w>> + '_ {
        let safe_fetch = self.safe_fetch.clone();
        self.sub_indices
            .iter()
            .map(move |&idx| unsafe { safe_fetch.fetch_read_only(idx) })
    }
}

impl<'a, 'w, Q: WorldQuery> QuerySubChunk<'a, 'w, Q, Mutable> {
    #[inline(always)]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = Q::Item<'w>> + '_ {
        let safe_fetch = self.safe_fetch.clone();
        self.sub_indices
            .iter()
            .map(move |&idx| unsafe { safe_fetch.fetch_mut(idx) })
    }
}

#[derive(Copy)]
struct ThreadSafeFetch<Q: WorldQuery>(Q::Fetch);
impl<Q: WorldQuery> Clone for ThreadSafeFetch<Q> {
    #[inline(always)]
    fn clone(&self) -> Self {
        ThreadSafeFetch(self.0)
    }
}

unsafe impl<Q: WorldQuery> Send for ThreadSafeFetch<Q> {}
unsafe impl<Q: WorldQuery> Sync for ThreadSafeFetch<Q> {}

impl<Q: WorldQuery> ThreadSafeFetch<Q> {
    #[inline(always)]
    unsafe fn fetch_read_only<'w>(&self, index: usize) -> Q::ReadOnlyItem<'w> {
        unsafe { Q::fetch_read_only(self.0, index) }
    }
    #[inline(always)]
    unsafe fn fetch_mut<'w>(&self, index: usize) -> Q::Item<'w> {
        unsafe { Q::fetch_mut(self.0, index) }
    }
}

pub struct QueryArchetypeView<'a, 'w, Q: WorldQuery, I> {
    pub(crate) indices: &'a [usize],
    pub(super) fetch: Q::Fetch,
    _marker: std::marker::PhantomData<&'w I>,
}

impl<'a, 'w, Q: WorldQuery, T> QueryArchetypeView<'a, 'w, Q, T> {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.indices.len()
    }
}

impl<'a, 'w, Q: WorldQuery, T: Sync> QueryArchetypeView<'a, 'w, Q, T> {
    pub fn iter(&self) -> impl Iterator<Item = Q::ReadOnlyItem<'w>> + '_ {
        self.indices
            .iter()
            .map(move |idx| unsafe { Q::fetch_read_only(self.fetch, *idx) })
    }
    pub fn par_iter(&self) -> impl IndexedParallelIterator<Item = Q::ReadOnlyItem<'w>> + '_
    where
        Q::ReadOnlyItem<'w>: Send,
    {
        let safe_fetch = ThreadSafeFetch::<Q>(self.fetch);

        self.indices
            .into_par_iter()
            .map(move |i| unsafe { safe_fetch.fetch_read_only(*i) })
    }
    pub fn par_chunks(
        &self,
        chunk_size: usize,
    ) -> impl IndexedParallelIterator<Item = QuerySubChunk<'_, 'w, Q, ReadOnly>>
    where
        Q::ReadOnlyItem<'w>: Send,
    {
        assert!(chunk_size > 0, "Chunk size must be greater than zero");
        let indices_slice = self.indices;
        let len = self.indices.len();
        let safe_fetch = ThreadSafeFetch::<Q>(self.fetch);

        (0..len)
            .into_par_iter()
            .step_by(chunk_size)
            .map(move |start_pos| {
                let end_pos = std::cmp::min(start_pos + chunk_size, len);
                QuerySubChunk {
                    safe_fetch: safe_fetch.clone(),
                    sub_indices: &indices_slice[start_pos..end_pos],
                    _marker: std::marker::PhantomData,
                }
            })
    }
}

impl<'a, 'w, Q: WorldQuery> QueryArchetypeView<'a, 'w, Q, Mutable> {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = Q::Item<'w>> + '_ {
        self.indices
            .iter()
            .map(move |idx| unsafe { Q::fetch_mut(self.fetch, *idx) })
    }
    pub fn par_iter_mut(&mut self) -> impl IndexedParallelIterator<Item = Q::Item<'w>> + '_
    where
        Q::Item<'w>: Send,
    {
        let safe_fetch = ThreadSafeFetch::<Q>(self.fetch);
        self.indices
            .into_par_iter()
            .map(move |i| unsafe { safe_fetch.fetch_mut(*i) })
    }
    pub fn par_chunks_mut(
        &mut self,
        chunk_size: usize,
    ) -> impl IndexedParallelIterator<Item = QuerySubChunk<'_, 'w, Q, Mutable>>
    where
        Q::Item<'w>: Send,
    {
        assert!(chunk_size > 0, "Chunk size must be greater than zero");

        let indices_slice = self.indices;
        let len = indices_slice.len();
        let safe_fetch = ThreadSafeFetch::<Q>(self.fetch);

        (0..len)
            .into_par_iter()
            .step_by(chunk_size)
            .map(move |start_pos| {
                let local_fetch = safe_fetch.clone();
                let end_pos = std::cmp::min(start_pos + chunk_size, len);

                QuerySubChunk {
                    safe_fetch: local_fetch,
                    sub_indices: &indices_slice[start_pos..end_pos],
                    _marker: std::marker::PhantomData,
                }
            })
    }
}

pub struct Query<Q: WorldQuery, F: Filter = EmptyFilter> {
    pub(crate) matching_archetypes: Vec<Option<*const Archetype>>,
    pub(crate) cached_fetches: Vec<Option<Q::Fetch>>,
    pub(crate) cached_indices: Vec<Vec<usize>>,
    pub(crate) _marker: std::marker::PhantomData<(Q, F)>,
}
unsafe impl<Q: WorldQuery, F: Filter> Send for Query<Q, F> {}
unsafe impl<Q: WorldQuery, F: Filter> Sync for Query<Q, F> {}

impl<'w, Q: WorldQuery + 'w, F: Filter> Query<Q, F> {
    pub(crate) fn new(world: &mut World, system_data: &mut FunctionData) -> Self {
        let mut matching_archetypes = vec![None; world.archetypes_manager.archetypes.len()];
        let mut cached_fetches = vec![None; world.archetypes_manager.archetypes.len()];
        let mut cached_indices = vec![Vec::new(); world.archetypes_manager.archetypes.len()];

        for arch in world.archetypes_manager.archetypes.values() {
            let arch_id = arch.id();
            if Q::matches(&arch.types)
                && F::matches(&AccessHashSet {
                    set: arch.types.clone(),
                })
            {
                matching_archetypes[arch_id as usize] = Some(arch as *const Archetype);
                let fetch = unsafe { Q::init_fetch(arch, system_data) };
                cached_fetches[arch_id as usize] = Some(fetch);
                let mut indices = (0..arch.entities.len()).collect::<Vec<usize>>();
                F::filter_indices(arch, &mut indices, system_data);
                cached_indices[arch_id as usize] = indices;
            }
        }

        Self {
            matching_archetypes,
            cached_fetches,
            cached_indices,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn iter<'a>(
        &'a self,
    ) -> impl Iterator<Item = QueryArchetypeView<'a, 'w, Q, ReadOnly>> + 'a {
        self.matching_archetypes
            .iter()
            .flatten()
            .map(move |&arch_ptr| unsafe {
                let arch = &*arch_ptr;
                let arch_idx = arch.id() as usize;

                QueryArchetypeView {
                    indices: &self.cached_indices[arch_idx],
                    fetch: self.cached_fetches[arch_idx].unwrap(),
                    _marker: std::marker::PhantomData,
                }
            })
    }

    pub fn iter_mut<'a>(
        &'a mut self,
    ) -> impl Iterator<Item = QueryArchetypeView<'a, 'w, Q, Mutable>> + 'a {
        let cached_fetches = &self.cached_fetches;
        let cached_indices = &self.cached_indices;

        self.matching_archetypes
            .iter()
            .flatten()
            .map(move |&arch_ptr| unsafe {
                let arch = &*arch_ptr;
                let arch_idx = arch.id() as usize;

                QueryArchetypeView {
                    indices: &cached_indices[arch_idx],
                    fetch: cached_fetches[arch_idx].unwrap(),
                    _marker: std::marker::PhantomData,
                }
            })
    }
    pub fn get(&self, entity: &Entity) -> Option<Q::ReadOnlyItem<'_>> {
        unsafe {
            let cell_ptr = REGISTRY.get_ptr(entity.registry_index as usize);

            let arch_id = (*cell_ptr).archetype_id;
            let row_idx = (*cell_ptr).idx;

            let fetch = self.cached_fetches[arch_id.id() as usize]?;
            Some(Q::fetch_read_only(fetch, row_idx as usize))
        }
    }

    pub fn get_mut(&mut self, entity: &Entity) -> Option<Q::Item<'_>> {
        unsafe {
            let cell_ptr = REGISTRY.get_ptr(entity.registry_index as usize);

            let arch_id = (*cell_ptr).archetype_id;
            let row_idx = (*cell_ptr).idx;

            let fetch = self.cached_fetches[arch_id.id() as usize]?;
            Some(Q::fetch_mut(fetch, row_idx as usize))
        }
    }
}
