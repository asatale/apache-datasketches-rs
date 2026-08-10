#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    struct JaccardBoundsFfi {
        lower_bound: f64,
        estimate: f64,
        upper_bound: f64,
    }

    unsafe extern "C++" {
        include!("theta_sketch_shim.h");
        include!("theta_compact_shim.h");
        include!("theta_wrapped_shim.h");
        include!("theta_jaccard_shim.h");

        type ThetaSketchShim = crate::theta_sketch::ffi::ThetaSketchShim;
        type CompactThetaSketchShim = crate::theta_compact::ffi::CompactThetaSketchShim;
        type WrappedCompactThetaSketchShim =
            crate::theta_wrapped::ffi::WrappedCompactThetaSketchShim;

        fn jaccard_sketch_sketch(a: &ThetaSketchShim, b: &ThetaSketchShim) -> JaccardBoundsFfi;
        fn jaccard_sketch_compact(
            a: &ThetaSketchShim,
            b: &CompactThetaSketchShim,
        ) -> JaccardBoundsFfi;
        fn jaccard_sketch_wrapped(
            a: &ThetaSketchShim,
            b: &WrappedCompactThetaSketchShim,
        ) -> JaccardBoundsFfi;
        fn jaccard_compact_sketch(
            a: &CompactThetaSketchShim,
            b: &ThetaSketchShim,
        ) -> JaccardBoundsFfi;
        fn jaccard_compact_compact(
            a: &CompactThetaSketchShim,
            b: &CompactThetaSketchShim,
        ) -> JaccardBoundsFfi;
        fn jaccard_compact_wrapped(
            a: &CompactThetaSketchShim,
            b: &WrappedCompactThetaSketchShim,
        ) -> JaccardBoundsFfi;
        fn jaccard_wrapped_sketch(
            a: &WrappedCompactThetaSketchShim,
            b: &ThetaSketchShim,
        ) -> JaccardBoundsFfi;
        fn jaccard_wrapped_compact(
            a: &WrappedCompactThetaSketchShim,
            b: &CompactThetaSketchShim,
        ) -> JaccardBoundsFfi;
        fn jaccard_wrapped_wrapped(
            a: &WrappedCompactThetaSketchShim,
            b: &WrappedCompactThetaSketchShim,
        ) -> JaccardBoundsFfi;
    }
}
