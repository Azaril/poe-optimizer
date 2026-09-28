# Explicit actor-owned ability data

This migration binds the existing Skeletal Sniper population slot to a reusable owned
Actor definition and explicitly supplies Basic Attack and Gas Arrow. The runtime uses
ordinary injected schemas, grants and typed rule programs. No skill-name dispatch or
source-language callback is involved.

The input commits the complete selected-action release
`751754e24a65031251c4047e97e4999a0114f87e750d662ae1a085129409a1d4`.
It requires the checked full-release migration compiler; applying these rows as a
monotonic schema refinement would be incorrect. Existing actor, skill, output and
baseline-rule identities remain unchanged. Eleven append-only registry allocations
`3091` through `309b` add the Actor definition, two skill slots, two grants and six
ability parameters. `bindings.json` names these typed addresses for authoring/tests.

Each Actor program supplies a distinct ability, activates its grant and projects:

| Input | Authored value | Evidence |
| --- | --- | --- |
| Ability level | Integer 1 | Both pinned ability effects have one level row. |
| Ability quality | 0 percentage points | The source constructs child abilities with quality 0. |
| Actor level | Exact current actor's existing level stat | Interpolation uses actor level independently of effect level. |

The optional complete-source test
[`owned_actor_ability_inputs.rs`](../../../../../crates/poe-optimizer-pob/tests/owned_actor_ability_inputs.rs)
establishes these facts with unchanged pinned functions in both JIT modes. Original05
has physical Gem level 20, effective summoning level 22, actor level 44 and ability
level 1 / quality 0. Separate component probes exercise actor levels 1/20/40/100 and
parent quality 0/20. The injected actor-level parameter range is 1–100; values outside
that reviewed range are not silently clamped. Storm Mage supplies an independent source
contrast and is not converted by this release.

Parent activation still gates the generated actor and both abilities. The literal true
ability grant does not activate a disabled or unresolved summoner. An unavailable actor
level remains an unresolved required input. The ability parameter collections and new
Actor rule coverage remain explicitly Partial; future conversion must supply their
remaining semantics. Existing rule closures and missing command-skill evidence remain.

Only original05 queries `reference-14` and `reference-16` gain the Basic Attack ability
grant at the end of their action-provider path. Their actor identity and output remain
unchanged. Every original, all 110 query IDs/order/metric selectors, and Twister's selected
action remain. The old output-only selectors are not aliases for the new supplied path.

Publish from the checked predecessor into a new directory:

```text
poe-optimizer assemble-owned-release runs/owned-selected-actions-01/package --migration data/owned/poe2/3887ae68/actor-ability-supply/migration.json --output runs/owned-actor-abilities-01/package
```

This data establishes supply and typed input projection. It does not implement ability
damage, support transfer, complete effective Gem levels or whole-build evaluation. See
the [actor contract](../../../../../docs/owned-actor-skill-supply.md) and
[implementation log](../../../../../docs/implementation.md) for validation/publication status.
