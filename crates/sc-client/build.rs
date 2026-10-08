//! The browser build's link (engine-stack design 10a): only for `wasm32-unknown-emscripten`, nothing
//! elsewhere. WebGL 2 (OpenGL ES 3.0) and nothing older, a heap that grows, the two small engine data
//! files embedded, and `web/pre.js`, which fetches the compressed deck and unpacks it into the
//! page's file system before `main` runs, so the client reads `compiled/tern.deck` as it does on a Pi.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("emscripten") {
        return;
    }
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let root = root.canonicalize().expect("the repository root");
    let pre = root.join("web/pre.js");
    println!("cargo:rerun-if-changed={}", pre.display());
    for arg in [
        "-sMIN_WEBGL_VERSION=2".to_owned(),
        "-sMAX_WEBGL_VERSION=2".to_owned(),
        "-sALLOW_MEMORY_GROWTH=1".to_owned(),
        "-sINITIAL_MEMORY=268435456".to_owned(),
        "-sSTACK_SIZE=5242880".to_owned(),
        "-sFORCE_FILESYSTEM=1".to_owned(),
        "-sEXPORTED_RUNTIME_METHODS=FS,addRunDependency,removeRunDependency".to_owned(),
        format!("--pre-js={}", pre.display()),
    ] {
        println!("cargo:rustc-link-arg-bins={arg}");
    }
    for rel in ["data/engine/render.json", "data/crew/walk.json", "data/space/exterior.json"] {
        let f = root.join(rel);
        println!("cargo:rerun-if-changed={}", f.display());
        println!("cargo:rustc-link-arg-bins=--embed-file={}@/{rel}", f.display());
    }
}
