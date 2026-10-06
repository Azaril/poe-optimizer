fn main() {
    // This script reads no package files. Target configuration is already part
    // of Cargo's build unit; unrelated authoring/docs edits need no script rerun.
    println!("cargo:rerun-if-changed=build.rs");
    // Windows/MSVC defaults the process main stack to 1 MiB. Debug builds of
    // the typed data loader exhaust it before native backend initialization
    // finishes (including the metrics command). Reserve 8 MiB for this executable;
    // pages are committed on demand. Worker stack sizes and other targets retain
    // their existing configuration.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg-bin=poe-optimizer=/STACK:8388608");
    }
}
