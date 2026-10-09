# Historical root witness source

These two files retain the exact observer and Rust test-driver bytes that produced
the original `class-start-root` packet's authenticated reports. They are source
evidence only; Cargo does not compile this directory, and no application path
loads its Lua file.

The original packet hashes and provenance remain unchanged. Its evidence checker
in `tests/support/owned_class_start_root.rs` resolves the original observer and
driver pins to these files explicitly, with no fallback to the current witness.
The original report metadata pins the source contents, not their disk locations.

Current root investigations use the generalized observer at
`crates/poe-optimizer-pob/tests/support/implicit_class_start_source.lua` and driver
at `crates/poe-optimizer-pob/tests/owned_implicit_class_start_source.rs`. New reports
authenticate those current files separately. The generalized witness handles
both the shared Class root and the selected Ascendancy root; the archived files
are not a second executable test framework or a supported historical API.
