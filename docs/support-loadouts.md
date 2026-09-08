# Configurable native support loadouts

The current Mace profile accepts zero, one or two level-1 quality-0 supports from
Brutality I, Heavy Swing and Rapid Attacks I. All seven unordered combinations can be
searched jointly with class, ascendancy, passive and supplied weapon choices, including
the bounded [normal/rare local modifier profile](local-weapons.md). These remain diagnostic
profiles. Supporting active skills, general equipment modifiers, minions, other skills and
complete native build evaluation remain unfinished.

## Configuration and calculation

Current package schema **10**, semantics **`poe2-native-profiles-v10`**, retains the support
model introduced in schema **4**, which replaced `mace.brutality` with a top-level `supports`
section. Schema 5 adds injected item modifier rules and the character critical-chance cap;
see [package migration](native-data.md#schema-migration). Each support record stores its data
key, source identities, family, color, represented level/quality, requirements,
required/excluded skill types,
typed numerical modifiers and explicit damage-disable effects. Mace's source skill types
and zero mana cost are recorded separately. Support mana multipliers are retained as
source metadata; the current profile rejects nonzero skill costs because it has no cost
or reservation pipeline. Unknown fields, operations and unsupported source expressions
reject. Older package schemas require regeneration and review of custom edits.

| Key | Reviewed effect | Color |
| --- | --- | --- |
| `brutality_i` | 25% more physical damage; disables elemental and chaos damage | Red |
| `heavy_swing` | 35% more melee physical damage; 10% less attack speed | Red |
| `rapid_attacks_i` | 15% increased attack speed | Green |

The actual values come from the selected package. Native code implements the operation
semantics and does not contain a second balance-value database. The extractor reads the
pinned Lua gem records and stat maps, including modifier flags and eligibility. Source
revision and the retained tree artifact are unchanged by the schema-4 support migration
and schema-5 item-rule migration.

`CompiledGameData` compiles valid loadouts once through the shared modifier database.
`PreparedMaceSupports` retains canonical data keys, numerical aggregates, damage flags
and a binding to its originating compiled dataset. The prepared calculation path performs
no support-name parsing, data loading, XML construction or allocation. It rejects a
prepared loadout from another compiled instance, even if that instance has equivalent
content. Whole prepared native evaluations retain their originating compiled data and
remain reusable under the existing backend/content-identity contract. Each selected weapon
also has immutable prepared local stats, reused across support and tree combinations.
The successful numeric snapshot path avoids allocations; preparation, owned metric
conversion, objective assessment, search bookkeeping and diagnostics still allocate.

Physical MORE modifiers multiply before damage endpoint rounding. Speed INC and MORE
combine before the combined speed multiplier rounds to two decimals. Critical and ordinary hits undergo
separate armour mitigation. Removing Brutality restores the weapon's fire contribution;
removing Heavy Swing removes both its physical modifier and its speed penalty. The legacy
`MaceInput.brutality` convenience API remains for existing callers, while native build
preparation uses explicit loadouts and the prepared calculation entry point.

## CLI problem and locks

Use [the support search example](../examples/mace-support-search.json):

```powershell
cargo run --no-default-features --locked -- search-experimental --problem examples/mace-support-search.json --jobs 4 --max-evaluations 2942 --timeout-seconds 120 --export runs/support-winner.xml
```

Problem schemas **4** and **5** require `support_loadouts` and explicit `tree_search` budgets.
Both retain the 0/1 ordinary and 0/1 ascendancy-point scope. A loadout is an array of data
keys; an empty array means no supports. The supplied alternatives are explicit:

```json
"support_loadouts": [
  [], ["brutality_i"], ["heavy_swing"], ["rapid_attacks_i"],
  ["brutality_i", "heavy_swing"],
  ["brutality_i", "rapid_attacks_i"],
  ["heavy_swing", "rapid_attacks_i"]
]
```

`locks.support_loadout` fixes the exact set, including an empty set. Keys canonicalize
before matching, so pair order does not create another candidate. Repeated keys, repeated
families, duplicate equivalent alternatives, missing locked choices and ineligible
supports reject. Existing weapon/class/ascendancy/paid-passive locks remain independent.
Schemas 4–5 reject legacy `supports` and `locks.support` fields. Problem schemas 1–3 retain
their existing `none`/`brutality_i` inputs and report layouts; their maximum support count
stays one. Schema-4 problems produce report schema **5**, with array-valued alternative
support loadouts and data-bound candidate identities. Schema-5 problems produce report
schema **6** and admit the supplied normal/rare local weapon grammar. Schemas 1–4 keep the
original normal, modifier-free item scope, including in their imported template.

Use the [local-weapon search example](../examples/mace-local-weapon-search.json) to combine
all seven loadouts with modified items. Native search defaults to prepared typed candidates;
`--native-evaluation document` compares full document evaluation under the same budgets.
Both modes evaluate the template and reserved finalist through the full document path.
The optional PoB backend retains its document path and rejects the native-only mode flag.

Catalogs admit at most 64 supplied weapons, seven loadouts and 105 class/tree selections:
47,040 structural alternatives. The existing 256 MiB estimate and cumulative source
hashing limits still apply; this is a preparation-work bound, not process-memory admission.
Preflight covers the entire lock-admissible domain, independently of proposal count.
Increase `--max-proposals` explicitly for products larger than its default 4,096.

Each enabled support adds the selected per-color cost in its matching attribute. With
reviewed costs, two red supports require 10 strength, while red/green requires 5 strength
and 5 dexterity. The final requirement is the maximum of this aggregate and individual
item/active/support requirements. For schema-5 weapons, explicit `LevelReq` selects the
equip-level requirement instead of the base level; item level is independent. Search rejects
illegal candidates before dispatch; a fully illegal locked domain uses zero calculations. Diagnostic evaluation can inspect such
a build without certifying its legality.

## Projection, evidence and validation

`ControlledMaceCatalog::with_loadouts` and `with_tree_loadouts` take canonical
`MaceSupportLoadout` values. Borrowed loadout resolver methods address complete candidates.
Legacy constructors/resolvers translate old choices through the same implementation;
`NormalMaceAlternative` remains an alias of `MaceWeaponAlternative`. The shared item parser
preserves exact weapon source and binds its local rolls to the selected package.
Materialization preserves unrelated source bytes, retains the main skill first, and emits
supports in canonical order. Imported source order remains explicit realization evidence.

Native Mace profile attachments now use media version **3**, retaining the loadout and
exact configured support records introduced in version 2, and adding exact `weapon_item`
source/roll evidence plus prepared `weapon_stats`. Tree evidence remains version **2**, with
separate ordinary/ascendancy allocation categories. Strict realization checks bind the
exported XML, selected data/backend identity, item diagnostics, support instances and
configured effects. The XML companion retains the dataset identity and trust required to
replay custom-data exports. Fresh finalist verification remains one counted full document
calculation; exporting the verified source adds no calculation. The reference backend
allows only the specifically derived metadata and neutral modifier ranges described in
[local weapon verification](local-weapons.md#prepared-calculations-and-verification).

Validation includes actual pinned modifier/offence branches in interpreted and warmed Lua,
fresh full PoB builds across all seven loadouts, both weapons, armour/fire-resistance
scenarios, the Monk speed entrance and all four resistance passives. Serial/Rayon search,
small exhaustive/guided references, exact locks, per-color requirements, custom data,
partial/empty budgets and export reimports complement the original independent goldens.
Completed counts and publication status live in [the implementation log](implementation.md).
