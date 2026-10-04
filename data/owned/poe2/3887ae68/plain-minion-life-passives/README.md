# Four plain Minion Life producers

This packet completes the default producer behavior of four passive nodes selected in Original05. It retains each existing 6% Minion Damage program verbatim and adds the separate 6% maximum Minion Life contribution demonstrated by the pinned source. It allocates only Stat `32e5`, an Actor-scoped percentage-point channel whose producers address Player. This carrier describes modifiers intended for minions; it does not change Player maximum Life.

| Source node | Owned Passive | Added producer |
|---|---|---|
| 19006 | `0cf8` | Life Increase 6 |
| 229 | `0dfb` | Life Increase 6 |
| 39461 | `1311` | Life Increase 6 |
| 54453 | `1791` | Life Increase 6 |

Each exact source definition has only the untagged Life and Damage modifier records. The full pinned Lua node bodies and tracked tree catalog prove ordinary allocation, no alternate view, attached choice, grant, socket or unlock behavior, and no class-start edge that could inject an additional flag. Source `229` is authenticated using its actual `skill=229` node body, separately from the tree group with the same number. External transforming providers keep their own coverage obligations.

The source witness records `tree.nodes[id].modList`, the default definition modifiers. Its `effective_same_definition` field is object identity between that tree definition and `spec.nodes[id]`; the recorded value is **false**. The pinned constructor deliberately creates per-spec metatable wrappers, so distinct identity is expected and does not mean different modifier contents. The publication helper preserves that false observation and the inherited name, and authenticates the constructor/index/dispatch excerpts. This packet does not claim that effective runtime modifiers were recorded or that external transformations are absent.

`closure.json` contains four replacement Passive descriptors, the one new Stat descriptor, and the four completed owner inventories. Their seven empty declaration inventories become Complete (28 in total). `dependencies.json` holds exact prior rows and referenced percent/Damage descriptors. `bindings.json` contains the reviewed IDs. `source-vectors.json` carries bounded, exact evidence pointers; `authoring.json` commits all artifact bytes and the input release.

The existing complete source witness is reused without alteration: both JIT reports have 19,612,949 bytes and SHA256 `030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935`. Its 37 cases / 38 complete loads preserve all five originals. This packet extracts the four precise Life/Damage records from all originals, repeated/warm original05 and the existing remove95 control. That control removes a Damage-only node and is not evidence for removing one of these Life nodes.

The publication helper uses the existing full successor and passive-declaration refinement APIs. It allocates Stat `32e5` through the typed registry, authenticates the exact predecessor/source bodies/report bytes, and reverses only the reviewed changes before comparing the entire prior recipe. Import dependency rebindings are separately checked. Registry history, every prior numerical program, nonreviewed closure, mapping, route and query contract must survive.

There is no published Life reducer or receiver, and no claim about received Life totals, a full Life pool, all Minion Life contributors or a complete build. The native aggregate checks use an explicitly finite test-only reducer to measure four producers as 24 and three as 18, and must preserve whole-request incomplete-contributor checks. These values are not full-build numerical parity. The real release retains its other Partial coverage and has no evaluation bundle.

## Verification

The authored-data test, four native Life tests and full publication/original-preservation test pass. Publication reproduces all eighteen files byte-for-byte and preserves all five original drafts, sidecars, saved selections and 110 queries. The earlier Damage/owner tests and previous publication also pass. Publication reuses the pinned source02 reports. The extended shared observer was separately rerun through all 37 cases / 38 complete loads in both JIT modes; every report byte remains identical to those pinned observations except the observer-code hash. The original reports remain untouched. The normal authored-data test reads only tracked artifacts/catalog/manifest; optional publication additionally authenticates the pinned checkout and local source reports.

```powershell
cargo test --test owned_plain_minion_owner_closure_cli four_life_node_closures_preserve_damage_and_have_complete_source_evidence
cargo test -p poe-optimizer-engine --test owned_plain_minion_life

$env:POE_OPTIMIZER_TEST_MINION_LIFE_PRIOR = 'runs/owned-plain-minion-owner-closure-01/package'
$env:POE_OPTIMIZER_TEST_MINION_LIFE_OUTPUT = 'runs/owned-plain-minion-life-passives-reproduction'
cargo test --test owned_plain_minion_owner_closure_cli publish_four_life_owner_closures_preserving_all_five_originals -- --include-ignored --nocapture
```

The output directory must be a new, nonexistent path. Publication must retain all five normalized drafts/sidecars and 110 query rows, preserve unresolved/full-build gates, and reproduce the release files byte for byte on a second build.