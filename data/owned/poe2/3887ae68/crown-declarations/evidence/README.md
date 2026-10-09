# Historical Crown witness source

These exact Rust/Lua bytes match the original Crown declaration proof's local
pins. They were recovered from commit
`53a719b2519fed9df04c3e3b106a48e3f07154b7`, independently checking both recorded
SHA-256 digests before copying. The retained reports and published provenance
remain unchanged.

The live `generated_extra_stat_consumption` helper has since gained further
controls. The Crown evidence checker resolves its two historical pins explicitly
to these snapshots. It still rejects any changed bytes; it neither accepts the
live helper as historical evidence nor substitutes a newly computed digest.
Cargo does not compile these files, and no application path loads the Lua file.
Current investigations continue to use the live helper.
