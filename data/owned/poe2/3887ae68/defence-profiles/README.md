# Raw defensive equipment profiles

The finite catalog covers every one of the 1,756 constructed item bases in the pinned
source. It preserves 1,240 present profiles, including six empty tables, and 516 absent
profiles. Present tables contain 3,031 raw numeric values across six reviewed fields.
Acquisition authenticates the optional checkout and reuses one bounded construction.
The paired item-base catalog remains byte-identical to the existing v1 artifact.

The native compiler reads only finite JSON and injected field policy. It shares validation
and literal lowering with the weapon compiler; the weapon wire format, digest domain,
program identifiers, error variants and work accounting are preserved. It does not copy
source tables, add a runtime parser, or select behavior by example-build identity.

| Source field | EquipmentUse statistic | Unit | Missing within a present table |
| --- | --- | --- | --- |
| Armour | `29fb` | Defence rating `29ee` | Explicit zero input |
| BlockChance | `29fc` | Percentage points `0002` | Omit value; false presence `2a01` |
| EnergyShield | `29fd` | Energy Shield points `29ed` | Explicit zero input |
| Evasion | `29fe` | Defence rating `29ee` | Explicit zero input |
| MovementPenalty | `29ff` | Percentage points `0002` | Omit value; false presence `2a02` |
| Ward | `2a00` | Ward points `29fa` | Explicit zero input |

Keys have prefix `def.000000000000` in `poe2/owned-mechanics-v1`. The ordered definition
append allocates Ward's resource-point unit, six EquipmentUse Quantity statistics and
two EquipmentUse capabilities, taking the registry from 10745 to 10754. Existing Actor
passive contribution channels are separate. `bindings.json` documents these meanings;
the checked policy and published schema establish the actual compiler contract.

An absent whole profile produces no program or field-presence facts. An empty profile
runs the same reviewed field-absence policy as any present table. Authored zero remains
present. The source's raw positive movement penalty, including values such as 0.05,
passes through without rescaling; later composition negates it as a conditional source
MovementSpeed BASE effect. Profile presence does not prove equipment eligibility,
activation, or selection of a weapon-versus-defence calculation branch.

Acquire through the optional Rust source adapter:

```text
poe-optimizer export-owned-defence-profiles --source-root vendor/path-of-building-poe2 --output NEW_ACQUISITION
```

Compile with a validated predecessor containing the
[passive defence channels](../passive-defence-inputs/README.md):

```text
poe-optimizer compile-owned-defence-profiles PRIOR --base-catalog data/owned/poe2/3887ae68/item-bases/catalog.json --catalog data/owned/poe2/3887ae68/defence-profiles/catalog.json --policy data/owned/poe2/3887ae68/defence-profiles/policy.json --definitions data/owned/poe2/3887ae68/defence-profiles/definitions.json --output NEW
```

Build the first command with feature `pob`; the second is available in the native-only
CLI. Both output directories must be new. Reproduction binds exact catalog bytes and
source provenance. Publication preserves compact v2 artifacts, resource bounds, prior
programs, owner closure, all five original drafts/item attributions and their 110 queries.
Acquisition evidence and this README stay outside the closed published bundle.

All profile owners remain Partial. Quality, local and hybrid modifiers, effect ordering,
per-level contributions, block flooring, movement-penalty conditions and ArmourData
overrides need subsequent owned recipes and activation. The explicitly empty Fists of
Stone base can acquire nonzero defences from implicit per-level effects; its baseline
zeros do not assert final zero item defences. Final actor defences, mitigation and complete
original-build evaluation remain open.

Tests and tooling added for this component are Rust. Existing Python remains unchanged
until the coherent utility-and-test migration under
[T1](../../../../../docs/architecture-migration.md#t1-rust-tooling-and-test-consolidation).
