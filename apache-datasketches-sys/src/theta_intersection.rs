#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("theta_sketch_shim.h");
        include!("theta_compact_shim.h");
        include!("theta_wrapped_shim.h");
        include!("theta_intersection_shim.h");

        type ThetaSketchShim = crate::theta_sketch::ffi::ThetaSketchShim;
        type CompactThetaSketchShim = crate::theta_compact::ffi::CompactThetaSketchShim;
        type WrappedCompactThetaSketchShim =
            crate::theta_wrapped::ffi::WrappedCompactThetaSketchShim;

        type ThetaIntersectionShim;

        fn new_theta_intersection() -> UniquePtr<ThetaIntersectionShim>;

        fn update_with_sketch(self: Pin<&mut ThetaIntersectionShim>, sketch: &ThetaSketchShim);
        fn update_with_compact(
            self: Pin<&mut ThetaIntersectionShim>,
            sketch: &CompactThetaSketchShim,
        );
        fn update_with_wrapped(
            self: Pin<&mut ThetaIntersectionShim>,
            sketch: &WrappedCompactThetaSketchShim,
        );

        fn get_result(
            self: &ThetaIntersectionShim,
            ordered: bool,
        ) -> Result<UniquePtr<CompactThetaSketchShim>>;
        fn has_result(self: &ThetaIntersectionShim) -> bool;
    }
}
