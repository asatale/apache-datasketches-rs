use apache_datasketches_sys::array_of_doubles_sketch::ffi as sys;

/// Controls how aggressively an ArrayOfDoubles sketch's internal hash table
/// grows. Mirrors upstream's `datasketches::resize_factor`. Default is `X8`,
/// matching `theta_constants::DEFAULT_RESIZE_FACTOR` (the tuple family
/// inherits Theta's builder defaults).
///
/// This is a distinct type from the theta module's `ResizeFactor` with the
/// same shape — the two sketch families are independently feature-gated and
/// do not share types. (Deliberately not an intra-doc link: `theta` may not
/// be compiled in.)
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

impl From<ResizeFactor> for sys::TupleResizeFactor {
    fn from(rf: ResizeFactor) -> Self {
        match rf {
            ResizeFactor::X1 => sys::TupleResizeFactor::X1,
            ResizeFactor::X2 => sys::TupleResizeFactor::X2,
            ResizeFactor::X4 => sys::TupleResizeFactor::X4,
            ResizeFactor::X8 => sys::TupleResizeFactor::X8,
        }
    }
}

/// Builder for [`crate::tuple::ArrayOfDoublesSketch`], mirroring upstream's
/// `update_array_of_doubles_sketch::builder`. `lg_k` defaults to `12`,
/// `resize_factor` to [`ResizeFactor::X8`], `p` to `1.0` (no sampling), and
/// `num_values` to `1` (matching upstream's
/// `default_array_tuple_update_policy` default). The seed is never exposed —
/// every sketch built by this crate uses upstream's `DEFAULT_SEED`.
#[derive(Debug, Clone, Copy)]
pub struct ArrayOfDoublesSketchBuilder {
    lg_k: u8,
    resize_factor: ResizeFactor,
    p: f32,
    num_values: u8,
}

impl Default for ArrayOfDoublesSketchBuilder {
    fn default() -> Self {
        Self {
            lg_k: 12,
            resize_factor: ResizeFactor::default(),
            p: 1.0,
            num_values: 1,
        }
    }
}

impl ArrayOfDoublesSketchBuilder {
    /// Creates a new builder with default settings (`lg_k = 12`,
    /// `resize_factor = X8`, `p = 1.0`, `num_values = 1`).
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

    /// Sets the sampling probability. `1.0` (the default) disables sampling;
    /// values below `1.0` put the sketch into estimation mode from the start.
    pub fn p(mut self, p: f32) -> Self {
        self.p = p;
        self
    }

    /// Sets the fixed number of `f64` values each retained entry carries.
    /// Must be at least `1`. Every sketch that will later be unioned,
    /// intersected, or differenced with this one must use the same value.
    pub fn num_values(mut self, num_values: u8) -> Self {
        self.num_values = num_values;
        self
    }

    /// Builds the sketch. Returns
    /// [`SketchError::InvalidConfig`](crate::SketchError::InvalidConfig) if
    /// `lg_k` is out of range, `p` is outside `(0, 1]`, or `num_values` is
    /// `0`.
    pub fn build(self) -> Result<super::ArrayOfDoublesSketch, crate::error::SketchError> {
        super::ArrayOfDoublesSketch::from_parts(
            self.lg_k,
            self.resize_factor,
            self.p,
            self.num_values,
        )
    }
}
