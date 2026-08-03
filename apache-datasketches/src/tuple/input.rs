use super::{ArrayOfDoublesSketch, CompactArrayOfDoublesSketch};
use apache_datasketches_sys::array_of_doubles_input::ArrayOfDoublesInputRef;

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::ArrayOfDoublesSketch {}
    impl Sealed for super::CompactArrayOfDoublesSketch {}
}

/// Either ArrayOfDoubles sketch type can be fed into any of this module's set
/// operations — union, intersection, a-not-b, and Jaccard similarity. This
/// trait is sealed — it cannot be implemented outside this crate — since the
/// set-op shims only have concrete overloads for these two exact types.
pub trait ArrayOfDoublesInput: sealed::Sealed {
    #[doc(hidden)]
    fn as_input(&self) -> ArrayOfDoublesInputRef<'_>;

    /// The fixed number of `f64` values each of this sketch's retained
    /// entries carries. Set operations require all operands to agree.
    fn get_num_values(&self) -> u8;
}

impl ArrayOfDoublesInput for ArrayOfDoublesSketch {
    fn as_input(&self) -> ArrayOfDoublesInputRef<'_> {
        ArrayOfDoublesInputRef::Sketch(&self.inner)
    }

    fn get_num_values(&self) -> u8 {
        ArrayOfDoublesSketch::get_num_values(self)
    }
}

impl ArrayOfDoublesInput for CompactArrayOfDoublesSketch {
    fn as_input(&self) -> ArrayOfDoublesInputRef<'_> {
        ArrayOfDoublesInputRef::Compact(&self.inner)
    }

    fn get_num_values(&self) -> u8 {
        CompactArrayOfDoublesSketch::get_num_values(self)
    }
}
