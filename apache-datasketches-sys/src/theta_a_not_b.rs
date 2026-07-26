#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("theta_sketch_shim.h");
        include!("theta_compact_shim.h");
        include!("theta_wrapped_shim.h");
        include!("theta_a_not_b_shim.h");

        type ThetaSketchShim = crate::theta_sketch::ffi::ThetaSketchShim;
        type CompactThetaSketchShim = crate::theta_compact::ffi::CompactThetaSketchShim;
        type WrappedCompactThetaSketchShim = crate::theta_wrapped::ffi::WrappedCompactThetaSketchShim;

        type ThetaAnotBShim;

        fn new_theta_a_not_b() -> UniquePtr<ThetaAnotBShim>;

        fn compute_sketch_sketch(self: &ThetaAnotBShim, a: &ThetaSketchShim, b: &ThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_sketch_compact(self: &ThetaAnotBShim, a: &ThetaSketchShim, b: &CompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_sketch_wrapped(self: &ThetaAnotBShim, a: &ThetaSketchShim, b: &WrappedCompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_compact_sketch(self: &ThetaAnotBShim, a: &CompactThetaSketchShim, b: &ThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_compact_compact(self: &ThetaAnotBShim, a: &CompactThetaSketchShim, b: &CompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_compact_wrapped(self: &ThetaAnotBShim, a: &CompactThetaSketchShim, b: &WrappedCompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_wrapped_sketch(self: &ThetaAnotBShim, a: &WrappedCompactThetaSketchShim, b: &ThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_wrapped_compact(self: &ThetaAnotBShim, a: &WrappedCompactThetaSketchShim, b: &CompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn compute_wrapped_wrapped(self: &ThetaAnotBShim, a: &WrappedCompactThetaSketchShim, b: &WrappedCompactThetaSketchShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
    }
}
