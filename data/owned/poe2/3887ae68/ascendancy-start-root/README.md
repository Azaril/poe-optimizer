# Default Ascendancy root inventory

This packet closes only the intrinsic, stat-free default inventory of source
node `8305`, owned passive `1b5d`. It adds no definitions, programs or receivers.
The seven empty declaration inventories and empty default rule inventory become
Complete; all topology, mappings, query sets and other owners retain their prior
contents. The immediate predecessor is the authenticated Mana override release.

Class `0a23`, Ascendancy `0a36` and shared Player Actor `332a` remain Partial.
Previously completed Class root `1790` remains unchanged. The selected root's
geometry is retained as a dependency of external radius transformations; it is
not classified wholly as presentation. No transformed-node, neighboring-node,
allocation-budget, final-metric or whole-build closure follows from this packet.

## Evidence

The generalized original-source witness authenticates the full loaded raw node,
constructor field inventory, complete default ModList, declaration-field
presence and selected root relation. It also executes an independent original
full constructor and checks its result against the loaded root. Original source
methods and the selected build, scalar outputs and selected modifier list remain
unchanged. No copied parser or native rule supplies the reference result.

Ascendancy nodes differ from the shared Class root: `PassiveSpec.lua:1464–1465`
reprocesses their selected occurrences with original `ProcessStats`. The selected
root therefore owns a different ModList object from the prototype. Reports retain
that false object-identity result and both full, equal, empty modifier projections.
This observation adds no Lua object-copy behavior to the native evaluator.

`runs/owned-implicit-roots-source-03/root-8305/source-jit-{off,on}.json` contains
three fresh loads and two independent constructor probes per JIT mode. The
235,225-byte reports are byte-identical, SHA-256
`5202a6d68cd2514643997d13cb69a5b2ac78f46f417fb6feb82d0f2a60c07dca`.
The same run also repeats the original Class-root witness through the generalized
code. Together both roots pass twelve full loads in 23.68 seconds.

The compact committed projection is authenticated to both reports. Reviewed raw
and constructed field dispositions reject unknown fields; selected ModList,
declaration, root-identity and topology mutations also reject. The existing
passive-refinement publisher checks the exact predecessor, complete inverse and
all-five import/query preservation. New ordinary tests share the existing
`owned_class_start_root` target and root evidence validator.

## Reproduction

Six ordinary Rust tests pass in 0.01 seconds. Publication02 passes in 30.80
seconds at `runs/owned-ascendancy-start-root-publication-02/package`, input
`d22618ec692e1d76d683e92cb623aebe5939d034681eb6876caafdfd10b20f84`.
All eighteen files rebuild byte-identically. All five imported drafts, source
sidecars, saved selections and 110 queries remain unchanged; selected input
obligations remain 107/117/109/123/4, with no evaluation bundle and 0/5 complete
native builds. Publication01 correctly rejected two JSON-authoring changes from
`-0.0` to `0`; exact source number tokens are now retained, with no relaxed
comparison. See the publication02 validation report for the full preservation
checks.

Use fresh output paths, the pinned source and serial Cargo execution:

```powershell
$env:POE_OPTIMIZER_TEST_IMPLICIT_CLASS_START_SOURCE_OUT = 'runs/owned-implicit-roots-source-fresh'
cargo test --locked -p poe-optimizer-pob --test owned_implicit_class_start_source actual_implicit_class_start_preserves_default_inventory_and_shared_root -- --ignored --exact --nocapture
cargo test --locked -p poe-optimizer-cli --test owned_class_start_root
$env:POE_OPTIMIZER_TEST_ASCENDANCY_START_ROOT_PRIOR = 'runs/owned-mana-override-publication-01/package'
$env:POE_OPTIMIZER_TEST_ASCENDANCY_START_ROOT_OUTPUT = 'runs/owned-ascendancy-start-root-publication-fresh'
cargo test --locked -p poe-optimizer-cli --test owned_class_start_root publish_default_ascendancy_root_preserving_all_five_originals -- --ignored --exact --nocapture
```

Publication authenticates the exact reviewed source-report paths recorded in
`source-vectors.json`. A deliberately refreshed source run requires review and
updated report/observer/driver hashes before publication; a fresh run is not
silently substituted for the retained evidence.
