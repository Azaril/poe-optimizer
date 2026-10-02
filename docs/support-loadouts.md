# Configurable native support loadouts

> Historical integration, updated 2026-10-02: the NativeBackend crate and its search
> CLI are removed. Engine's legacy support arithmetic and independent source fixtures
> remain; this document does not describe an available optimizer command. The
> [execution overview](data-and-evaluation-overview.md) describes the active owned path.

The historical Mace profile accepted zero, one or two level-1 quality-0 supports from
Brutality I, Heavy Swing and Rapid Attacks I. All seven unordered combinations could be
searched jointly with class, ascendancy, passive and supplied weapon choices, including
the bounded [normal/rare local modifier profile](local-weapons.md). These remain diagnostic
profiles. Supporting active skills, general equipment modifiers, minions, other skills and
complete native build evaluation remain unfinished.

## Configuration and calculation

At this checkpoint, package schema **10**, semantics **`poe2-native-profiles-v10`**, retained the support
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
content. The removed backend's prepared evaluations retained their originating compiled data
under its backend/content-identity contract. Each selected weapon had immutable prepared
local stats reused across support and tree combinations. Its numeric snapshot path avoided
allocations; preparation, metric conversion, assessment, search and diagnostics still allocated.

Physical MORE modifiers multiply before damage endpoint rounding. Speed INC and MORE
combine before the combined speed multiplier rounds to two decimals. Critical and ordinary hits undergo
separate armour mitigation. Removing Brutality restores the weapon's fire contribution;
removing Heavy Swing removes both its physical modifier and its speed penalty. The legacy
`MaceInput.brutality` convenience API remains in Engine; the removed native adapter used
explicit loadouts and the prepared calculation entry point.

## Search and validation

The finite Mace catalog/command and the later [graph search adapter](passive-equipment-assembly.md)
are retired. Their instance-lock and budget behavior is historical validation evidence.
The [domain architecture](domain-architecture.md) expresses support applicability and costs
in injected semantic rules; general owned search integration remains unfinished.

The retired backend integration tests exercised all seven-loadout/two-weapon/scenario cases
through the lazy domain, including effect removal and fresh PoB export reimports. Shared
Engine numerical tests and independent PoB fixtures remain. They do not establish full-build
coverage through the owned evaluator.

Historical validation included actual pinned modifier/offence branches in interpreted and warmed Lua,
fresh full PoB builds across all seven loadouts, both weapons, armour/fire-resistance
scenarios, the Monk speed entrance and all four resistance passives. Serial/Rayon search,
small exhaustive/guided references, exact locks, per-color requirements, custom data,
partial/empty budgets and export reimports complemented the original independent goldens.
Completed counts and publication status live in [the implementation log](implementation.md).
