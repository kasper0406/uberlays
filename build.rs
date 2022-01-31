extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=irsdk/wrapper.hpp");

    let bindgens = bindgen::Builder::default()
        .header("irsdk/wrapper.hpp")
        .rustfmt_bindings(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindgens");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindgens
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
    
    prost_build::compile_protos(&["src/track.proto"], &["src/"])
        .expect("Failed to generate protobuf code");
}
