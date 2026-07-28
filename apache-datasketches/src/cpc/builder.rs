use crate::error::SketchError;

/// Builder for [`crate::cpc::CpcSketch`], mirroring upstream's
/// `cpc_sketch_alloc` constructor. `lg_k` defaults to `11`
/// (`cpc_constants::DEFAULT_LG_K`). The seed is never exposed — every
/// sketch built by this crate always uses upstream's `DEFAULT_SEED`.
#[derive(Debug, Clone, Copy)]
pub struct CpcSketchBuilder {
    lg_k: u8,
}

impl Default for CpcSketchBuilder {
    fn default() -> Self {
        Self { lg_k: 11 }
    }
}

impl CpcSketchBuilder {
    /// Creates a new builder with the default `lg_k` (`11`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base-2 log of the number of bins in the sketch (`4..=26`).
    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    /// Builds the sketch. Returns [`SketchError::InvalidConfig`] if `lg_k`
    /// is out of range.
    pub fn build(self) -> Result<super::CpcSketch, SketchError> {
        super::CpcSketch::from_lg_k(self.lg_k)
    }
}

/// Builder for [`crate::cpc::CpcUnion`], mirroring upstream's
/// `cpc_union_alloc` constructor. `lg_k` defaults to `11`
/// (`cpc_constants::DEFAULT_LG_K`). The seed is never exposed, same as
/// [`CpcSketchBuilder`].
#[derive(Debug, Clone, Copy)]
pub struct CpcUnionBuilder {
    lg_k: u8,
}

impl Default for CpcUnionBuilder {
    fn default() -> Self {
        Self { lg_k: 11 }
    }
}

impl CpcUnionBuilder {
    /// Creates a new builder with the default `lg_k` (`11`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base-2 log of the number of bins in the union (`4..=26`).
    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    /// Builds the union. Returns [`SketchError::InvalidConfig`] if `lg_k`
    /// is out of range.
    pub fn build(self) -> Result<super::CpcUnion, SketchError> {
        super::CpcUnion::from_lg_k(self.lg_k)
    }
}
