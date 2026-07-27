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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    pub fn build(self) -> Result<super::CpcSketch, SketchError> {
        super::CpcSketch::from_lg_k(self.lg_k)
    }
}
