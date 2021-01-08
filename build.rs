use std::env;
use std::path::PathBuf;

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:link-search=/usr/lib");
    println!("cargo:rustc-link-lib=X11");

    // Invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=src/bindgen.h");
    println!("cargo:rerun-if-changed=src/utils.c");

    // Invalidate the built crate whenever any of the included header files changed.
    let bindings = bindgen::Builder::default()
        .header("src/bindgen.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    cc::Build::new()
        .file("src/utils.c")
        .compile("utils");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
