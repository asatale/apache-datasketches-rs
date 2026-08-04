use apache_datasketches_sys::theta_sketch::ffi as sys;

/// Controls how aggressively a theta sketch's internal hash table grows.
/// Mirrors upstream's `datasketches::resize_factor`. Default is `X8`,
/// matching `theta_constants::DEFAULT_RESIZE_FACTOR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResizeFactor {
    /// Grow by 1x (i.e. never resize past the initial allocation).
    X1,
    /// Grow by 2x each time the hash table fills.
    X2,
    /// Grow by 4x each time the hash table fills.
    X4,
    /// Grow by 8x each time the hash table fills. The default.
    #[default]
    X8,
}

impl From<ResizeFactor> for sys::ResizeFactor {
    fn from(rf: ResizeFactor) -> Self {
        match rf {
            ResizeFactor::X1 => sys::ResizeFactor::X1,
            ResizeFactor::X2 => sys::ResizeFactor::X2,
            ResizeFactor::X4 => sys::ResizeFactor::X4,
            ResizeFactor::X8 => sys::ResizeFactor::X8,
        }
    }
}

/// Builder for [`crate::theta::ThetaSketch`], mirroring upstream's
/// `update_theta_sketch::builder`. `lg_k` defaults to `12`
/// (`theta_constants::DEFAULT_LG_K`), `resize_factor` to [`ResizeFactor::X8`],
/// `p` to `1.0` (no sampling). The seed is never exposed — every sketch built
/// by this crate always uses upstream's `DEFAULT_SEED`.
#[derive(Debug, Clone, Copy)]
pub struct ThetaSketchBuilder {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
}

impl Default for ThetaSketchBuilder {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::default(),
            p: 1.0,
        }
    }
}

impl ThetaSketchBuilder {
    /// Creates a new builder with default settings (`lg_k = 12`,
    /// `resize_factor = X8`, `p = 1.0`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base-2 log of the target number of retained entries.
    pub fn lg_k(mut self, lg_k: u8) -> Self {
        self.lg_k = lg_k;
        self
    }

    /// Sets the hash table's growth [`ResizeFactor`].
    pub fn resize_factor(mut self, resize_factor: ResizeFactor) -> Self {
        self.resize_factor = resize_factor;
        self
    }

    /// Sets the sampling probability. `1.0` (the default) disables
    /// sampling; values below `1.0` put the sketch into estimation mode
    /// from the start.
    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    /// Builds the sketch. Returns
    /// [`SketchError::InvalidConfig`](crate::SketchError::InvalidConfig) if
    /// `lg_k` is out of range.
    pub fn build(self) -> Result<super::ThetaSketch, crate::error::SketchError> {
        super::ThetaSketch::from_parts(self.lg_k, self.resize_factor, self.p)
    }
}
