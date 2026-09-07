# Observed passive coverage

`BuildCoverage.passives` carries an owned, backend-neutral observation of the evaluated passive tree. It supports oracle comparisons while the primary calculation implementation moves into portable Rust. It does not import an extracted catalog into the live evaluator or certify that requested allocations survived loading.

## Schema and compatibility

The surrounding coverage schema remains version 1. Historical saved reports without `passives` deserialize as `None`, and reserialization omits the absent field. A present observation has its own `schema_version: 1`, rejects unknown fields, and is validated structurally. `EvaluationResult::validate_recorded` also checks that its class, primary ascendancy, tree version, and physical allocation set agree with the existing `BuildSummary`. Readers cannot silently accept malformed present evidence as if it were absent. A mutation consumer that needs passive realization proof must explicitly require this field; legacy compatibility is not permission to skip verification.

All fields are owned Rust values with serde support. They contain no Lua references, operating-system handles, source-file paths, or process identifiers. Native backends can implement the same observation semantics without the PoB runtime.

## Actual runtime evidence

The PoB adapter executes the read-only `passive_coverage.lua` after MAIN evaluation, against `build.spec` and its live selected descriptors and nodes. It reads:

- Selected class index, internal integer ID, name, and implicit start node.
- Selected primary/secondary ascendancy index, internal ID, catalog ID, name, and start node where available. Missing identity is retained if the backend has fallen back from an invalid selection.
- Every physical node in `spec.allocNodes`, sorted by ID, including implicit roots, and its allocation mode (0 common, 1 weapon set 1, 2 weapon set 2).
- The live node type, inherited `node.name`, actual display name `node.dn`, and actual `node.sd` stat lines.
- Implicit-root roles, `isFreeAllocate` with nil/false/true preserved, ascendancy membership name, multiple-choice/granted/attribute flags, conquest state, and hash-override presence.
- The six return values of the actual `spec:CountAllocNodes()` call: ordinary, primary ascendancy, secondary ascendancy, sockets, weapon set 1, and weapon set 2.

These buckets overlap. A weapon-set allocation is also counted in its ordinary or ascendancy category; sockets are a subset. `CountAllocNodes` excludes class/ascendancy roots and nodes whose `isFreeAllocate` is non-nil, then treats ascendancy multiple-choice options specially. The adapter does not replace this calculation with a guessed point-cost sum. The observation contains the complete allocated spec, including both weapon sets; it is not a claim that every allocation contributed to the currently selected weapon set's calculation.

The adapter additionally observes `spec.switchableNodes`, the reverse map populated by the live `BuildAllDependsAndPaths` replacement branch. It joins map values to allocated nodes by Lua table identity, reporting the actual selected source ID. A class or ascendancy selector is classified only when that ID agrees with the corresponding live option; otherwise an unclassified observation is preserved. This uses the loaded evaluator's data and actual selection map, not the Rust tree snapshot.

`ReplaceNode` changes `dn` and `sd` but preserves the inherited `name`. Those fields are intentionally separate. For example, the Abyssal Lich root can legitimately display `Lich` through inherited upstream data. The switch record also states whether the live stat table is still the selected source's table and whether its display name still matches. A subsequent jewel, attribute choice, or other modifier can break these matches. Reporting that difference avoids pretending an automatic source selection proves the final effective payload.

The source manifest stays unchanged. The adapter fingerprint includes this Lua observer and the shared passive types/validation, as well as the separated import crate implementation and manifest. Existing fixtures and independent numerical goldens are not regenerated.

## Verification

The core contract tests cover historical schema-1 reports, optional-field round trips, distinct live names, nil/false allocation flags, malformed version/mode/root/order/count data, switch selectors, unknown fields, and disagreement with the existing build summary.

The live integration matrix derives expectations from the separately extracted, pinned source snapshot, then evaluates independent fresh build documents. It covers all 31 class/ascendancy identities and 16 ordinary class entrances, including reverse-listed graph entrances and class/ascendancy switches. The matrix is initialized once and runs at most two PoB child processes at a time. Comparisons check internal ownership, implicit roots, exact physical allocation sets, direct count buckets, live display names/stat lines, selected switch-source IDs, base attributes, and finite positive resources. Further cases inspect the supplied build and a weapon-set allocation, demonstrating that mode and ordinary counts overlap.

This is differential evidence between a data projection and the real pinned evaluator, not a replacement for the independent C-host numerical goldens. It detects runtime pruning, alias/inheritance differences, and source-projection mistakes. It does not prove complete game topology, acquisition legality, combat execution, or full native calculation parity.
