// ONNX Runtime's static library (CoreML glue) calls clang's
// `__isPlatformVersionAtLeast`, which lives in clang's runtime, not in what
// rustc links: point the linker at it on macOS.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let out = std::process::Command::new("clang")
        .arg("--print-resource-dir")
        .output()
        .expect("clang is needed to build on macOS");
    let dir = String::from_utf8(out.stdout).expect("utf-8 path");
    println!("cargo:rustc-link-search=native={}/lib/darwin", dir.trim());
    println!("cargo:rustc-link-lib=static=clang_rt.osx");
}
