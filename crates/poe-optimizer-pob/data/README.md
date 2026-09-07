# Pinned evaluator source manifest

[pob-source-manifest.json](pob-source-manifest.json) records the SHA-256 and
normalized byte length of every tracked `.lua` file under the upstream
`src/` and `runtime/lua/` directories, plus its root `manifest.xml`.
The initial pin is `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`:
1,082 files, 40,737,626 normalized bytes.

[src/source.rs](../src/source.rs) verifies these files using Rust before Lua
starts. Git is needed to regenerate this manifest, but is not needed to evaluate
a packaged runtime. The verifier also rejects additional Lua modules, symbolic
links in the inspected source trees, and startup override files
`src/first.run`, `src/installed.cfg`, and `src/manifest.xml`.

Only UTF-8 CRLF line endings become LF before hashing; a BOM, lone CR, and all
other bytes remain significant. The manifest fingerprint applies the same
normalization to the JSON itself. Source hashes therefore match both ordinary
Windows and Linux checkouts.

All active tree, gem, item, stat-description, modifier-cache, and query-modifier
data is Lua and covered. Graphics, development/export inputs, and tree JSON
fallbacks are excluded: the verifier requires each `tree.lua` before startup,
so a missing tree cannot silently trigger JSON conversion. Imported build XML
has its own source hash and is not part of this manifest.

## Regeneration and pin upgrades

From the repository root, with PowerShell 7.2 or newer and Git available:

```powershell
pwsh -File scripts/update-pob-manifest.ps1
git diff -- crates/poe-optimizer-pob/data/pob-source-manifest.json
```

The [generator](../../../scripts/update-pob-manifest.ps1) reads the declared
revision from both evaluator `UPSTREAM_REVISION` constants, requires a matching
clean upstream checkout, enumerates tracked paths in ordinal order, validates
UTF-8, computes normalized hashes, and checks the pin/clean state again before
writing this JSON file. It also rejects ignored Lua modules that could shadow
a tracked module and the three startup overrides. It does not fetch or update
the submodule. Regenerating an unchanged pin reproduces the canonical LF JSON byte for byte.

To upgrade deliberately:

1. Review the intended upstream commit and update the submodule checkout to that
   commit. Keep its source tree clean.
2. Update `UPSTREAM_REVISION` in both
   [source.rs](../src/source.rs) and [runtime.rs](../src/runtime.rs).
   Review changed loading behavior for new data dependencies or startup overrides.
3. Run the generator with `-ExpectedRevision <reviewed-40-character-commit>`.
   A pin mismatch fails before the output is written.
4. Review the manifest diff, run source-verifier tests and the evaluator
   import/export/fresh-calculation checks on Windows and Linux, then update the
   [implementation checkpoint](../../../docs/implementation.md).
   Record changed calculation behavior before adopting new golden outputs.

The verifier bounds each file by its manifest length, caps normalized source
files at 16 MiB, and caps the inventory at 20,000 entries. Changes to these limits
or manifest scope require a corresponding verifier/generator review.
