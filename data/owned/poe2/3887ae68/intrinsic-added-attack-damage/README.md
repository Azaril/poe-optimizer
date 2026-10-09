# Intrinsic added attack damage

This data-only packet supplies the intrinsic added-damage contribution for the exact admitted Skeletal Sniper Actor provider. It uses existing typed expressions, Actor/Action scopes and checked action routing. It adds three Stats and two programs, one route, no query/receiver/table, and no coverage closure.

- `336b`: Actor raw percentage, produced on Skill 0012 / ActorSlot 001f (exact provider Actor 3091) from injected profile scale 2537 as `(scale - 1) * 100`.
- `336c`: the same percentage projected through the exact Basic Attack 0021 / output 0022 selector (part 0007, mode 0008, stat set 0009).
- `336d`: one current-Action Multiply contribution, `1 + percentage / 100`, present only when percentage is nonzero. It is an individual unrounded factor, not the final combined AddedDamage factor.

The retained source scale 1.15 produces percentage 14.999999999999991. This is not rounded to 15 or reconstructed from a combined factor. Both Actor and Action carrier values remain available for audit and later contributor grouping. Generic Product is not introduced as proof of source group rounding. Source emission is absent at scale 1; a missing profile, producer or route is unavailable rather than an identity default.

Admission is structural: the existing ActorSlot declares exact provider 3091 and belongs to the reviewed creating Skill 0012. The Basic output alone does not authorize this producer for other Actors, and its Gas Arrow sibling receives no new route. No runtime skill-name check, profile DTO, synthetic Actor, default final input or unconditional factor for other profiles is added. The complete authenticated Sniper profile has no damageFixup field. A present numeric zero or Boolean false is not treated as evidence of absence. Source Spectre/Companion filtering and Attack flags are retained in the offline evidence; broader domains remain unsupported.

`source-vectors.json` projects the unchanged physical-damage source02 reports (37 cases, 38 complete loads per JIT) into 24 exact recipient/mode rows from 10 controls. Thirteen rows have actual Basic Attack observations; others remain explicitly unobserved. The rows retain source occurrence, profile, query configuration, original filtered raw record and the original physical-base consumer observation, including the flat 3–7 controls. The complete loaded profile and absence proof come from the existing intrinsic-Life source02 reports, rather than inferring absence from a scalar-only offence projection. Historical observer provenance remains historical; no current observer is substituted. No fresh source VM is needed.

Publication starts at `runs/owned-ascendancy-owner-publication-01/package`, uses the existing checked three-Stat migration, appends exactly two programs/one route, and proves the exact inverse to that migration. Both real owners, route inventories, global query closure, existing producers and all other data retain their prior coverage. All five fresh imports, local identities, saved queries and Pending responsibilities must remain unchanged, with selected counts 106/117/109/123/4 and 18 byte-identical rebuilt files. The root joined Sniper fixture separately checks actual source preparation, missing profile/authorized producer/route, sibling route exclusion, identity absence, scratch reuse and parallel execution.

```powershell
cargo test --locked --all-features --test owned_intrinsic_added_attack_damage
$env:POE_OPTIMIZER_TEST_INTRINSIC_ADDED_ATTACK_PRIOR = 'runs/owned-ascendancy-owner-publication-01/package'
$env:POE_OPTIMIZER_TEST_INTRINSIC_ADDED_ATTACK_OUTPUT = 'runs/owned-intrinsic-added-attack-publication-02'
cargo test --locked --all-features --test owned_intrinsic_added_attack_damage -- --include-ignored --nocapture
```

Validated publication: `runs/owned-intrinsic-added-attack-publication-02/package`,
input `c4c38e18383f708bc36d4a571bb260d0afadf0e69c1e7a6881ef80b8a0098ba3`.
All five packet/source/publication tests pass. The joined numerical check and
replay export pass, all ten ordinary native replay tests pass, and all 38 tests
in its six existing consumers pass. Strict Engine and all-feature CLI Clippy
pass. The current implementation document records logs and the resume point.
Complete native builds remain 0/5; no owner or input obligation was retired.
