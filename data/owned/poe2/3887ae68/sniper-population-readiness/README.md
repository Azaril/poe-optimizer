# Sniper population readiness partition

This offline packet partitions the existing owned Skill `0012` program
`ordinary-population-inputs`. Its three unchanged facts project actor level,
project the quality factor, and activate the exact population grant. The new
`ordinary-population-requirements` program retains the original character-level
requirement. The facts can participate in preparation; the requirement remains
an execution obligation. Population existence is not inferred from satisfying
that requirement.

Each program contains only the original reads and nodes reachable from its own
effects, in the original vector order. The level read and node are shared;
quality dependencies belong to facts and character-level eligibility dependencies
belong to the requirement. `partition.json` records the exact original program,
both subsequences, and original-index maps. No original row is unreferenced or
discarded. The Rust authoring checker reconstructs every original row and rejects
changed values, missing dependencies, unused reads, reordered indices, duplicated
effects and conflicting shared dependencies.

The predecessor is `runs/owned-command-damage-01/package`, authenticated by its
complete release receipt in `dependencies.json`. Only one owner's program list,
the independent rule-release identity, and a final authoring provenance entry
change. The owner's existing Partial coverage and other programs are preserved.
Definitions, registry, schema, tables, receivers, effect applications, routing,
import policies and all 110 queries stay exact. No IDs, game defaults, operations,
formulas, field dispositions or coverage claims are introduced. No new PoB run is
required to prove this syntax-preserving owned-data partition.

The helper compiles the successor through the existing full release assembler.
It then inverses the partition and reassembles the entire predecessor, requiring
its exact full receipt. The publication test replays all five unchanged originals,
authenticates complete drafts/sidecars before comparing them with only the
existing fresh-import lineage canonicalization, retains selected issue counts, and
rebuilds all 18 artifacts byte-identically. Only `rules.json`, `manifest.json` and
`release.json` may differ from the predecessor. The package carries no evaluation
bundle; phase recommendations in `authoring.json` are authoring guidance, not
proof of a complete build or a published stage contract.

Three ordinary Rust authoring/tampering tests pass. Publication01 passes in
24.96 seconds at `runs/owned-sniper-population-readiness-01`, this checkpoint's
checked endpoint. All eight Command native tests pass in 20.44 seconds
against the two actual published packages, including the preserved character-level
requirement on independent roots and four-worker Rayon replay. Targeted strict
Clippy passes. These are component checks; complete native originals remain 0/5.
The ignored publication test uses `POE_OPTIMIZER_TEST_SNIPER_POPULATION_PRIOR` and
`POE_OPTIMIZER_TEST_SNIPER_POPULATION_OUTPUT`. Command Damage native tests consume
the authenticated successor through `POE_OPTIMIZER_TEST_COMMAND_DAMAGE_READINESS_RELEASE`.
Exact identities and receipts are recorded in the
[implementation checkpoint](../../../../../docs/implementation.md).
