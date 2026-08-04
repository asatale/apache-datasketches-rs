#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("theta_sketch_shim.h");
        include!("theta_compact_shim.h");
        include!("theta_wrapped_shim.h");
        include!("theta_union_shim.h");

        type ThetaSketchShim = crate::theta_sketch::ffi::ThetaSketchShim;
        type CompactThetaSketchShim = crate::theta_compact::ffi::CompactThetaSketchShim;
        type WrappedCompactThetaSketchShim =
            crate::theta_wrapped::ffi::WrappedCompactThetaSketchShim;
        type ResizeFactor = crate::theta_sketch::ffi::ResizeFactor;

        type ThetaUnionShim;

        fn new_theta_union(lg_k: u8, rf: ResizeFactor, p: f32)
            -> Result<UniquePtr<ThetaUnionShim>>;

        fn update_with_sketch(self: Pin<&mut ThetaUnionShim>, sketch: &ThetaSketchShim);
        fn update_with_compact(self: Pin<&mut ThetaUnionShim>, sketch: &CompactThetaSketchShim);
        fn update_with_wrapped(
            self: Pin<&mut ThetaUnionShim>,
            sketch: &WrappedCompactThetaSketchShim,
        );

        fn get_result(self: &ThetaUnionShim, ordered: bool) -> UniquePtr<CompactThetaSketchShim>;
        fn reset(self: Pin<&mut ThetaUnionShim>);
    }
}
