# Magnified Area support delivery

This packet adds ordinary Action-context support programs for Magnified Area I
(Gem `082a`) and II (Gem `082b`). It uses the existing support preparation and
receiving contracts. Source identities, constants and receiving paths are data;
there is no new evaluator branch or public API.

| Source support | Area contribution | Support resource-cost factor | Damage contribution |
| --- | --- | --- | --- |
| Magnified Area I | Increase 35 percentage points | Multiply 1.3 | None: its zero damage key has no modifier mapping |
| Magnified Area II | Increase 45 percentage points | Multiply 1.3 | Multiply 1.0 only when the explicit Area eligibility input is true |

New Action Stat `32f9` carries Area Increase with percentage-point unit `0002`.
New Action Stat `32fa` carries dimensionless support resource-cost factors with
unit `0001`. The source calls this second modifier `SupportManaMultiplier`, but
its cost consumer applies it to multiple resource costs; this is not a final mana
cost or a mana-only penalty. The source's later four-decimal rounding and final
cost formulas are outside this packet.

New Action Boolean Stat `32fb` is a required computed Area eligibility input for
II's separate damage program. It has no producer in this packet. A supported
skill's mutable prepared flags are not replaced by a constant inferred from its
name or catalogue types. The guarded neutral factor uses existing generic Action
damage-factor Stat `32f8`. Missing eligibility stays unresolved; false eligibility
produces no damage contribution. I's unmapped zero is retained in source evidence
and has no invented numerical producer.

The source's `Tabulate` helper suppresses zero-valued non-override records. II's
zero therefore need not appear in that diagnostic list even when Area matches.
Its proof must retain the raw Area-flagged modifier, the actual query Area bit and
the original `More` behavior. Diagnostic suppression does not remove a native
program or justify omitting the condition.

The receiving fragment includes eleven existing Action outputs: the two player
Djinn Commands, eight Sand/Water Djinn child actions, and Ice Nova. Their exact
grant paths expand fourteen declared stat sets. Each receiver uses independent
prepared admission; a child's summoner is the assigned root. The source rejects
both tiers on both player Commands, which therefore receive no contributions.
The eight child actions and both Ice Nova sets admit both tiers in the reviewed
contexts. A shared support list does not turn local skill merges into global
player contributions. The Djinn
primary Summons have no owned Action output here; their source merge observations
do not fabricate one or certify full primary delivery.

`migration.json` is a V4 migration retaining schema V6 and operations V19. It
allocates only `32f9`–`32fb` and appends five programs. The previous II
prepared-input program and its Partial closure survive exactly. I receives a new
Partial owner. All existing descriptors, programs, routing, imports and queries
are preserved apart from checked release dependency rebinding.

`receiving.json` and `source-vectors.json`, including Lua declaration snippets,
are authoring-only reference artifacts. The eighteen-file runtime/import
publication contains their provenance commitment, not these payloads or Lua
strings. The publication test checks that boundary explicitly.
All outer receiving inventories and both rule owners remain Partial. The native
fixture supplies explicit finite preparation, activation and eligibility inputs;
those test boundaries do not complete production inventories. No final radius,
resource cost, damage, complete original build or retired input issue is claimed.

## Evidence and reproduction

The exact predecessor is `runs/owned-direct-support-targets-02/package`, input
`7f365df6c162b5d1c9cf45aca32531ee5a647f0ace4e7b1b53e411e6b98d53bc`.
Static declarations come from the pinned source and the existing authenticated
Djinn preparation evidence. Source04 passed all 36 cases in both JIT modes in
574.85 seconds. Each report is 74,626,853 bytes with SHA256
`f77f0506c32020c028d1ce007e2600dceef06ff083c2b7ad782177373cba0d6d`:
`runs/owned-magnified-area-source-04/source-jit-off.json` and
`runs/owned-magnified-area-source-04/source-jit-on.json` are byte-identical.
The witness preserves all five unchanged originals and their independent
repeats, fresh plus two requested rebuild observations, and exact original
method/object joins. It does not select a canonical parity lifecycle.

The bounded receipt projects exact source fields and a fourteen-row action/set
matrix. Knife Throw set 1 lacks the Area query bit while set 2 has it; both Ice
Nova sets have it. Water's recharge, conversion and passive Mana Wave actions
lack it despite admitting Magnified. The native fixture uses these observed
facts as an explicit finite input boundary; no production Area classifier is
authored. Source `Flag` returning nil remains absent evidence, not a fabricated
false value. Magnified's own cost record is MORE 30 (factor 1.3); Ice's aggregate
1.43 also includes Encroaching Ground's separate MORE 10 record.

Source01 failed diagnostic serialization; source02 exceeded the former 64 MiB
report bound; source03 covered only thirteen sets because scalar Ice selectors
were ignored by the original loader. The repaired controls use the original
per-effect stat-set child maps. These failed runs remain failed evidence. The
source04 gate and authenticator use a bounded 128 MiB report limit.

To reproduce the optional source gate, keep its child selector unset and use a
fresh evidence directory:

```powershell
Remove-Item Env:POE_MAGNIFIED_AREA_SOURCE_CHILD -ErrorAction SilentlyContinue
$env:POE_MAGNIFIED_AREA_SOURCE_OUT = 'runs/owned-magnified-area-source-new'
cargo test -p poe-optimizer-pob --test owned_djinn_support_preparation_source magnified_area_support::complete_magnified_area_delivery_uses_original_recipients -- --ignored --exact --test-threads=1
```

The ordinary authoring check uses tracked data and the source manifest only:

```powershell
cargo test -p poe-optimizer-cli --test owned_magnified_area_support_delivery authored_magnified_area_preserves_costs_conditions_and_partial_coverage
```

Publish into a new, nonexistent output directory:

```powershell
$env:POE_OPTIMIZER_TEST_MAGNIFIED_PRIOR = 'runs/owned-direct-support-targets-02/package'
$env:POE_OPTIMIZER_TEST_MAGNIFIED_OUTPUT = 'runs/owned-magnified-area-support-delivery-new'
cargo test -p poe-optimizer-cli --test owned_magnified_area_support_delivery publish_magnified_area_preserving_all_five_originals -- --ignored --exact
```

The checked publication compares both freshly normalized endpoints for all five
unchanged originals, preserves all 110 query rows and selected unresolved counts
`106/117/109/122/5`, and rebuilds all eighteen package files byte-for-byte. It
authenticates actual source files and reports only in this optional gate.

Publication03 passed in 27.45 seconds. Its checked package is
`runs/owned-magnified-area-support-delivery-03/package`, input
`f835888002f4212f9aa663cd86aa6a7abff6e0b5b2a971863642017d3d689881`.
The eighteen files total 60,804,367 bytes with 97 provenance rows. Earlier
publication attempts exposed stale shared-test assumptions about the Direct
dependency digest and V20 sidecars; those failures remain recorded. The repaired
helpers retain exact policy, source-link and version comparisons.

Native tests use the resulting checked release through
`POE_OPTIMIZER_TEST_MAGNIFIED_RELEASE=<new output>/package`. Their finite boundary
is separate from release-wide coverage. All six passed in 4.46 seconds against
publication03 (`runs/owned-magnified-area-native-03.log`):

```powershell
$env:POE_OPTIMIZER_TEST_MAGNIFIED_RELEASE = 'runs/owned-magnified-area-support-delivery-03/package'
cargo test -p poe-optimizer-cli --test owned_magnified_area_support_delivery native:: --locked -- --ignored --nocapture
```

They cover tier values, exact admission, the fourteen-row Area matrix, removal,
disabled supports, repeated occurrences, duplicate-family/quality selection,
missing eligibility, refusal of Partial coverage, scratch reuse and four Rayon
workers. Ice retains its physical Gem-to-generated-Skill path and explicit finite
final-input projection. Two earlier runs stopped on fixture readiness/receiver
facts; the fixes change only test setup. No runtime default or alternate physical
representation was introduced.
