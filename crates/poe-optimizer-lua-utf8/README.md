# Native UTF-8 module for the PoB host

`register(&mlua::Lua)` preloads `lua-utf8` from statically compiled C source.
Call it before loading PoB's `Modules/Common.lua`. The Lua state needs the
package standard library; a normal `mlua::Lua::new()` is enough for this module.

The C API calls link to the same vendored LuaJIT that `mlua` uses. No native
library is loaded through `package.cpath`, and this crate does not use the
upstream Windows `lua51.dll` or `lua-utf8.dll`.

## Source and build provenance

- UTF-8 implementation: [starwing/luautf8 0.1.6](https://github.com/starwing/luautf8/tree/1bb70d45208d4033dcc75efee968be70534dd3ab),
  commit `1bb70d45208d4033dcc75efee968be70534dd3ab`. This is the upstream
  release corresponding to the `luautf8 0.1.6-1` dependency in the pinned
  [PoB Dockerfile](../../vendor/path-of-building-poe2/Dockerfile).
  `lutf8lib.c`, `unidata.h`, and its MIT `LICENSE` are copied unchanged.
- Public LuaJIT headers and copyright:
  [luajit-src 210.7.3+1ee778a](https://crates.io/crates/luajit-src/210.7.3+1ee778a),
  published crate SHA-256
  `869665372263eb337b14f480cfb864b89f12eade4eb42cb415f71517b4a67572`.
  Its packaging commit is `6a4e63186d4c84d726737b85ab2882b7213c9717`;
  the embedded [LuaJIT source](https://github.com/LuaJIT/LuaJIT/tree/1ee778a4e37122d8ca7d5733c590a47dafd6b15c)
  is `1ee778a4e37122d8ca7d5733c590a47dafd6b15c`.
  The four public headers and `COPYRIGHT` are copied unchanged from the archive.
- Exact file checksums and lengths are in [vendor/provenance.json](vendor/provenance.json).

`mlua 0.12.1` uses `mlua-sys 0.12.0`. Its vendored LuaJIT build exports
link metadata but does not export a Cargo include directory. We therefore
vendor the small public header set and constrain `luajit-src` to its matching
version in this crate's build dependencies. The build dependency only constrains
Cargo's resolver; this crate never invokes `luajit_src::Build::build`.
`mlua-sys` builds the single Lua runtime; `cc` builds only the UTF-8 module.

Keep the header source pin and `luajit-src` version aligned when updating LuaJIT.
Do not enable `mlua`'s alternate Lua backends or define `LUA_BUILD_AS_DLL` in
this workspace. Use `cargo tree -i luajit-src` to verify the single runtime
source selection after a dependency update.

The unit tests exercise multibyte indexing, reversal, substitution, casing,
invalid UTF-8 detection, module caching, and a native error crossing the Lua
protected-call boundary. They deliberately empty `package.cpath`.
