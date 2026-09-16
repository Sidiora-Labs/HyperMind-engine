#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use std::collections::BTreeSet;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

pub const HNSW_THRESHOLD: usize = 50_000;
const CONNECTIVITY: usize = 32;
const EXPANSION: usize = 512;

pub struct HnswIndex {
    index: Index,
    dimensions: usize,
}

impl std::fmt::Debug for HnswIndex {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HnswIndex")
            .field("dimensions", &self.dimensions)
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

impl HnswIndex {
    pub fn new(dimensions: usize, capacity: usize) -> Result<Self, Error> {
        if dimensions == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let index = Index::new(&IndexOptions {
            dimensions,
            metric: MetricKind::IP,
            quantization: ScalarKind::I8,
            connectivity: CONNECTIVITY,
            expansion_add: EXPANSION,
            expansion_search: EXPANSION,
            multi: false,
        })
        .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))?;
        index
            .reserve(capacity.max(1))
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        Ok(Self { index, dimensions })
    }

    pub fn add(&self, key: u64, vector: &[i8]) -> Result<(), Error> {
        if key == 0 || vector.len() != self.dimensions || self.index.contains(key) {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        if self.index.size() == self.index.capacity() {
            self.index
                .reserve(self.index.capacity().saturating_mul(2).max(1))
                .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        }
        self.index
            .add(key, vector)
            .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))
    }

    pub fn search(
        &self,
        query: &[i8],
        admitted: &BTreeSet<u64>,
        limit: usize,
    ) -> Result<Vec<u64>, Error> {
        if query.len() != self.dimensions || limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        if admitted.is_empty() {
            return Ok(Vec::new());
        }
        self.index
            .filtered_search(query, limit.min(admitted.len()), |key| {
                admitted.contains(&key)
            })
            .map(|matches| matches.keys)
            .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))
    }

    pub fn checkpoint(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = vec![0; self.index.serialized_length()];
        self.index
            .save_to_buffer(&mut bytes)
            .map_err(|_| Error::new(ErrorCode::WriteFailed))?;
        Ok(bytes)
    }

    pub fn restore(dimensions: usize, bytes: &[u8]) -> Result<Self, Error> {
        let result = Self::new(dimensions, 1)?;
        result
            .index
            .load_from_buffer(bytes)
            .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))?;
        if result.index.dimensions() != dimensions
            || result.index.metric_kind() != MetricKind::IP
            || result.index.scalar_kind() != ScalarKind::I8
        {
            return Err(Error::new(ErrorCode::VectorIndexCorrupt));
        }
        result.index.change_expansion_search(EXPANSION);
        result.index.change_expansion_add(EXPANSION);
        Ok(result)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.index.size()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
