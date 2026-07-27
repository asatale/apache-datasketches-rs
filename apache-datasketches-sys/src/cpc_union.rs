#[cxx::bridge(namespace = "apache_datasketches_rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("cpc_sketch_shim.h");
        include!("cpc_union_shim.h");

        type CpcSketchShim = crate::cpc_sketch::ffi::CpcSketchShim;

        type CpcUnionShim;

        fn new_cpc_union(lg_k: u8) -> Result<UniquePtr<CpcUnionShim>>;

        fn update_sketch(self: Pin<&mut CpcUnionShim>, sketch: &CpcSketchShim);

        fn get_result(self: &CpcUnionShim) -> UniquePtr<CpcSketchShim>;
    }
}
