# Shared receiving defences and resistances

The native evaluator computes player Armour, Evasion, Energy Shield and four resistances
from the same ordered modifier sources used for attributes and maximum resources. Spark
and Mace Strike share this stage, including typed candidate admission and complete XML
re-evaluation. Runtime calculation uses injected data and Rust only.

## Supported source scope

The current pipeline admits global Armour/Evasion/EnergyShield BASE and INC; actual
ArmourAndEvasion BASE/INC and Defences INC; and individual/elemental resistance BASE/INC.
Complete paired and all-resistance parser expansions retain each emitted record. Supported
attribute-comparison conditions use the actor's final two-pass condition state. Explicit
`Global` tags are preserved: they prevent item-local consumption but do not restrict the
subsequent global query. In particular, the source parser adds this tag to increased
maximum Energy Shield even when the text does not say “global”.

Configuration, Weapon 1, Helmet, Body Armour, Gloves, Boots, Amulet and passive records
retain source order in one local modifier layer. The four armour slots use shared local
preparation; see [local armour](local-armour.md) and [body/movement](body-armour-movement.md). Existing two Mace bases
and seven amulet bases are available: Amber, Jade, Lapis, Bloodstone, Solar, Lunar and
Pearlescent. Lunar's base requirement is level 14; Pearlescent's is level 30. Item level
is not a substitute for a requirement; an explicit supported `LevelReq` overrides it.
Rare payload validation proves calculation syntax, not obtainable affix combinations.

Defences BASE remains rejected because its source query also contributes Ward. MORE,
OVERRIDE, maximum-resistance modifiers, donor conversions, Chaos Inoculation, slot-dependent
forms, unsupported conditions and additional actors remain explicit exclusions. This stage
does not establish full defence, reservation, mitigation, recovery or full-game parity.

## Data and API contracts

Package schema 8 introduced the required `receiving_defence` section; current schema
**10**, `poe2-native-profiles-v10`, retains it alongside [local armour](local-armour.md)
and [body/movement data](body-armour-movement.md).
It holds ordered output/query membership; balance values remain in their existing injected
character, quest, defence and source-modifier records. Rust implements operation semantics.
The loader validates complete source-reviewed query shapes and allowed operation/tag scope.

All defensive passive contributions now live in ordered `actor_modifiers`. Passive `effects`
contains offence scalars only. Packages using the former defensive scalar fields reject;
regenerate schema 1–7 packages and review/reapply custom edits. Incrementing the schema
number or guessing a mixed-record order is not a migration. The tree remains schema 3 and
its bytes, PoB revision, full source snapshot and independent goldens stay unchanged.

`CompiledGameData::prepare_actor` accepts level, actor quest selection, `ReceivingScenario`,
character input and source layers. `evaluate_actor` accepts compiled borrowed fragments and
reusable `ActorScratch`. Both return `PreparedActorResources` with a fixed `ReceivingOutput`
available through `receiving()`. Prepared actors bind the exact compiled-data owner,
character attributes, resistance penalty and resistance quests. The receiving output keeps
ratings, capped resistances, truncated pre-cap totals and floor/cap evidence; it retains no
XML, query database, source strings or candidate-result cache.

The older `prepare_actor_resources` / `evaluate_actor_resources` APIs remain available for
raw resource-function callers. They cannot silently authorize a complete skill calculation
when unconsumed receiving records are present. Complete actor preparation rejects nonzero
legacy defensive scalars. Rebinding a complete actor permits offence-only changes;
attributes, defensive scalars and receiving scenarios must agree. Raw numeric compatibility
entrypoints retain their explicit scalar behaviour.

At this PoE2 source revision Strength contributes Life, Dexterity Accuracy and Intelligence
Mana. There is no inherent Dexterity-to-Evasion or Intelligence-to-Energy-Shield multiplier.
Defence queries preserve BASE/INC operation order, final rounding and nonnegative clamp.
Resistance queries preserve individual-then-elemental ordering, nonnegative INC factor,
truncation toward zero and separately truncated floor/cap. Chaos receives neither the
resistance penalty nor elemental quest rewards. Negative defence INC is not clamped before
its product; source boundary tests include signed and fractional inputs.

## CLI and evidence

```powershell
cargo run --release --no-default-features --locked -- search-build --problem examples/receiving-defence-search.json --jobs 4 --max-evaluations 1000
cargo run --no-default-features --locked -- evaluate tests/fixtures/builds/spark-receiving-defence.xml --backend native --raw
```

Problem **8** uses the existing extensible objective/constraint and source-selection model,
with report **9**, scope `receiving_defence_native_search_v1`. The example maximizes selected
hit DPS subject to a fire-resistance and Energy-Shield floor. These are caller-selected
examples, not a fixed optimizer objective. Equipment and passive choices can change the
receiver while encounter and authored configuration stay fixed for the run.

Graph problem 7 and legacy mutation problems 1–6 keep their existing authored input scope;
new receiving configuration/equipment requires graph problem 8 or later. Armour requires
problem 9, and Body Armour/authored movement requires problem 10. Migrated passive records
continue to work through the shared stage. Typed/document evaluation, per-worker deterministic
archives, evaluation ledgers and fresh finalist export verification use the same contracts.
Player `armour` and `evasion` are now direct rating metrics, definition schema 1 with
unit `rating_points`; see [local armour and rating objectives](local-armour.md). Energy
Shield and capped resistances retain their existing units and definitions.
Unsupported metrics remain unavailable; the receiver does not add an EHP approximation.

Native profile IDs are `poe2-spark-action-timing-v7` and
`poe2-mace-strike-action-timing-v11`, with profile media versions **7** and **9**. Each
profile carries a separate `receiving_defence` evidence object (schema 1). Realization checks
recompute expected receiving evidence from selected sources and reject tampering, a foreign
data owner or different scenario. Output XML preserves source; its companion binds exact data.

Validation includes independent cold/warm original parser and receiver execution, fresh
complete PoB builds and export reimports, custom injected values, requirement boundaries,
private evidence bindings and allocation-counted varied actor/skill calls. Performance must
separate fresh source admission from reusing an already-admitted actor. See the living
[implementation checkpoint](implementation.md) for measured results and remaining work.
