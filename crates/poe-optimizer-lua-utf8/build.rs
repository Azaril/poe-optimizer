fn main() {
    println!("cargo:rerun-if-changed=vendor/luautf8/lutf8lib.c");
    println!("cargo:rerun-if-changed=vendor/luautf8/unidata.h");
    println!("cargo:rerun-if-changed=vendor/luajit-headers");

    // These public headers are copied from the exact luajit-src release pinned
    // in Cargo.toml. mlua-sys does not currently export include-directory metadata.
    // Do not define LUA_BUILD_AS_DLL: all C API calls resolve to mlua's static VM.
    cc::Build::new()
        .file("vendor/luautf8/lutf8lib.c")
        .include("vendor/luajit-headers")
        .define("_CRT_SECURE_NO_WARNINGS", None)
        .warnings(false)
        .compile("poe_optimizer_luautf8");
}
