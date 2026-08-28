use std::any::TypeId;

use fxhash::FxBuildHasher;
use indexmap::IndexSet;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

use crate::{
    entity::Entity,
    extensions::{AccessVec, ParamAccess, SystemParam},
    query::filter::{EmptyQueryFilter, QueryFilter},
    registry::REGISTRY,
    system::validation::{AccessHashSet, FunctionData},
    world::{archetypes::Archetype, storage::World},
};

pub trait QueryData {
    type Item<'w>;
    type ReadOnlyItem<'w>;
    type Fetch: Copy;

    fn matches(types: &IndexSet<TypeId, FxBuildHasher>) -> bool;
    unsafe fn init_fetch(archetype: &Archetype, systems_data: &mut FunctionData) -> Self::Fetch;
    fn collect_access(
        reads: &mut AccessVec<std::any::TypeId>,
        writes: &mut AccessVec<std::any::TypeId>,
    );
    unsafe fn fetch_mut<'w>(fetch: Self::Fetch, index: usize) -> Self::Item<'w>;
    unsafe fn fetch_read_only<'w>(fetch: Self::Fetch, index: usize) -> Self::ReadOnlyItem<'w>;
}

pub struct Mutable;
pub struct ReadOnly;

pub struct QuerySubChunk<'w, Q: QueryData, I> {
    safe_fetch: ThreadSafeFetch<Q>,
    sub_indices: &'w [usize],
    _marker: std::marker::PhantomData<I>,
}

impl<'w, Q: QueryData, T> QuerySubChunk<'w, Q, T> {
    #[inline(always)]
    pub fn iter<'b>(&'b self) -> impl Iterator<Item = Q::ReadOnlyItem<'b>> {
        let safe_fetch = self.safe_fetch.clone();
        self.sub_indices
            .iter()
            .map(move |&idx| unsafe { safe_fetch.fetch_read_only(idx) })
    }
}

impl<'w, Q: QueryData> QuerySubChunk<'w, Q, Mutable> {
    #[inline(always)]
    pub fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = Q::Item<'b>> {
        let safe_fetch = self.safe_fetch.clone();
        self.sub_indices
            .iter()
            .map(move |&idx| unsafe { safe_fetch.fetch_mut(idx) })
    }
}

#[derive(Copy)]
struct ThreadSafeFetch<Q: QueryData>(Q::Fetch);
impl<Q: QueryData> Clone for ThreadSafeFetch<Q> {
    #[inline(always)]
    fn clone(&self) -> Self {
        ThreadSafeFetch(self.0)
    }
}

unsafe impl<Q: QueryData> Send for ThreadSafeFetch<Q> {}
unsafe impl<Q: QueryData> Sync for ThreadSafeFetch<Q> {}

impl<Q: QueryData> ThreadSafeFetch<Q> {
    #[inline(always)]
    unsafe fn fetch_read_only<'w>(&self, index: usize) -> Q::ReadOnlyItem<'w> {
        unsafe { Q::fetch_read_only(self.0, index) }
    }
    #[inline(always)]
    unsafe fn fetch_mut<'w>(&self, index: usize) -> Q::Item<'w> {
        unsafe { Q::fetch_mut(self.0, index) }
    }
}

pub struct QueryArchetypeView<'a, Q: QueryData, I> {
    pub(crate) indices: &'a [usize],
    pub(super) fetch: Q::Fetch,
    _marker: std::marker::PhantomData<I>,
}

impl<'w, Q: QueryData, T> QueryArchetypeView<'w, Q, T> {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.indices.len()
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a, Q: QueryData, T: Sync> QueryArchetypeView<'a, Q, T> {
    pub fn iter<'b>(&'b self) -> impl Iterator<Item = Q::ReadOnlyItem<'b>> {
        self.indices
            .iter()
            .map(move |idx| unsafe { Q::fetch_read_only(self.fetch, *idx) })
    }

    pub fn par_iter<'b>(&'b self) -> impl IndexedParallelIterator<Item = Q::ReadOnlyItem<'b>>
    where
        Q::ReadOnlyItem<'b>: Send,
    {
        let safe_fetch = ThreadSafeFetch::<Q>(self.fetch);

        self.indices
            .into_par_iter()
            .map(move |i| unsafe { safe_fetch.fetch_read_only(*i) })
    }

    pub fn par_chunks<'b>(
        &'b self,
        chunk_size: usize,
    ) -> impl IndexedParallelIterator<Item = QuerySubChunk<'b, Q, ReadOnly>>
    where
        Q::ReadOnlyItem<'b>: Send,
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

impl<'a, Q: QueryData> QueryArchetypeView<'a, Q, Mutable> {
    pub fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = Q::Item<'b>> {
        self.indices
            .iter()
            .map(move |idx| unsafe { Q::fetch_mut(self.fetch, *idx) })
    }

    pub fn par_iter_mut<'b>(&'b mut self) -> impl IndexedParallelIterator<Item = Q::Item<'b>>
    where
        Q::Item<'b>: Send,
    {
        let safe_fetch = ThreadSafeFetch::<Q>(self.fetch);
        self.indices
            .into_par_iter()
            .map(move |i| unsafe { safe_fetch.fetch_mut(*i) })
    }

    pub fn par_chunks_mut<'b>(
        &'b mut self,
        chunk_size: usize,
    ) -> impl IndexedParallelIterator<Item = QuerySubChunk<'b, Q, Mutable>>
    where
        Q::Item<'b>: Send,
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

pub struct Query<'q, Q: QueryData, F: QueryFilter = EmptyQueryFilter> {
    pub(crate) matching_archetypes: Vec<Option<*const Archetype>>,
    pub(crate) cached_fetches: Vec<Option<Q::Fetch>>,
    pub(crate) cached_indices: Vec<Vec<usize>>,
    pub(crate) _marker: std::marker::PhantomData<(&'q (), Q, F)>,
}
unsafe impl<'q, Q: QueryData, F: QueryFilter> Send for Query<'q, Q, F> {}
unsafe impl<'q, Q: QueryData, F: QueryFilter> Sync for Query<'q, Q, F> {}

impl<'q, Q: QueryData, F: QueryFilter> Query<'q, Q, F> {
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

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = QueryArchetypeView<'a, Q, ReadOnly>> {
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

    pub fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = QueryArchetypeView<'a, Q, Mutable>> {
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
    pub fn get(&self, entity: &Entity) -> Option<Q::ReadOnlyItem<'q>> {
        unsafe {
            let cell_ptr = REGISTRY.get_ptr(entity.registry_index as usize);

            let arch_id = (*cell_ptr).archetype_id;
            let row_idx = (*cell_ptr).idx;

            let fetch = self.cached_fetches[arch_id.id() as usize]?;
            Some(Q::fetch_read_only(fetch, row_idx as usize))
        }
    }

    pub fn get_mut(&mut self, entity: &Entity) -> Option<Q::Item<'q>> {
        unsafe {
            let cell_ptr = REGISTRY.get_ptr(entity.registry_index as usize);

            let arch_id = (*cell_ptr).archetype_id;
            let row_idx = (*cell_ptr).idx;

            let fetch = self.cached_fetches[arch_id.id() as usize]?;
            Some(Q::fetch_mut(fetch, row_idx as usize))
        }
    }
}

impl<'q, Q: QueryData + 'static, F: QueryFilter + 'static> SystemParam for Query<'q, Q, F> {
    fn get_access() -> ParamAccess {
        let mut access = ParamAccess::default();
        Q::collect_access(&mut access.reads, &mut access.writes);
        F::collect_filter(&mut access.with_filters, &mut access.without_filters);
        access
    }

    fn extract(world: &mut World, system_data: &mut FunctionData) -> Self {
        Query::<Q, F>::new(world, system_data)
    }
}
