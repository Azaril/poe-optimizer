# Injectable configuration definitions: audit and proposal

Status: source-audited proposal, with no production model or package changes. This is a
bounded next step in [breadth validation](breadth-validation.md), not a claim that the five
supplied builds can already be evaluated natively. Broad skill/actor architecture remains a
separate design discussion.

## Audit evidence

The five `breadth-20260908` builds contain five ConfigSets, all id 1 and active. There are no
inactive configuration sets in this corpus. They contain 58 `Input` nodes, 170 `Placeholder`
nodes and one `CustomModifierBlock`: 228 scalar occurrences across 59 distinct keys
(25 input keys and 34 placeholder keys). All 59 keys resolve against the pinned source.
Five attribute values change under standard XML whitespace normalization; all are Medallion
quest choices. Their original values, including LF/tab, exactly match source options.

Local machine-readable evidence is `runs/config-definition-inventory.json`, produced by
`runs/audit_config_definitions.py`. It retains build/set/node occurrence identity, source
scalar text, standard-parser differences, ordered options/defaults, current native local
admission observations and source/data/file hashes. The audit executes the original
`ConfigOptions.lua` table construction, quest-option generator and `xml.ParseXML` on exact
corpus bytes. It does not execute configuration effect, UI or tooltip callbacks, calculate
final settings, or certify general legality. Callback descriptors contain locations/hashes,
never serialized executable semantics. The static native value-guard inventory is a
read-only mirror of the current profile guard, not an extra evaluator.

The complete configuration source returns 663 table rows, including 564 variable rows for
563 distinct keys and 17 generated quest definitions. Preserve both original table order and
variable occurrence order. `conditionEnemyExitedPresenceRecently` appears at
`ConfigOptions.lua:1669` and `:1672`: one callback emits `ExitedPresenceRecently`, the other
`EnteredPresenceRecently`. `ConfigTab:BuildModList` visits both. A unique-key map would lose
a source producer. This duplicate is not used by the five supplied builds but constrains the
shared catalog design.

Only five authored corpus inputs pass the current native local scalar guard: two disabled
Silent Hall quest inputs, one disabled Molten Shrine input and two nearby-enemy counts.
That does not admit any complete corpus build. Current native profiles also reject all
Placeholder children, require a fixed explicit encounter input set and constrain skills,
equipment and saved sets. Most supported scalar values in the placeholder table are therefore
not currently representable through complete native admission.

## Source semantics the boundary must preserve

The original [ConfigTab](../vendor/path-of-building-poe2/src/Classes/ConfigTab.lua) separates
three default concepts:

- `CreateConfigSet` (`:1317–1333`) assigns each row's `defaultState` and
  `defaultPlaceholderState`, then its `defaultIndex` choice overwrites the initial input.
  Repeated rows can overwrite or remove earlier values through ordinary Lua assignment.
- `GetDefaultState` (`:980–998`) gives an existing placeholder priority over UI defaults,
  then uses a scalar-kind fallback. This display fallback is not an authored input and is
  not automatically passed to a calculation callback.
- Callback-driven settings can replace placeholders or inputs later. For example,
  `enemyIsBoss` updates level, resistances, damage, armour and evasion. Initial defaults do
  not describe the final resolved scenario.

`BuildModList` (`:1170–1199`) dispatches callbacks in definition order. Checkboxes call
`apply(true)` only for a truthy stored input. Numeric forms try a stored input and then a
placeholder; only the `count` form excludes zero. `countAllowZero`, `integer` and `float`
retain zero. List/text callbacks receive their truthy stored value. In Lua, zero and empty
strings are truthy. Custom-modifier blocks follow the ordinary callbacks in block order.
UI visibility fields such as `ifCond` do not gate this dispatch loop. Their presence is not
a proof that a saved setting is numerically inactive.

[List choices](../vendor/path-of-building-poe2/src/Modules/ConfigOptions.lua) describe UI
options, not a universal import whitelist. `ConfigTab:Load` (`:878–974`) retains arbitrary
string inputs; the generated quest callback splits and parses that string. The dropdown's
`SelByValue` searches exact values without rewriting the input on a miss. A native policy
may explicitly admit only reviewed option values, but must report that as its own capability
limit rather than claim the source rejects other strings.

The load path also has explicit compatibility behavior: boss-name title casing and legacy
aliases (`enemyIsBoss`), removal of the exact leading `Uber ` preset prefix, and migration of
legacy `customMods` into a block under a specific existing-block condition. A string-valued
`Placeholder` unexpectedly populates the input map; numeric placeholders populate the
placeholder map. Preserve raw node kind and scalar kind before making a compatibility
decision. Numeric XML entities and other unsupported lexical forms retain their existing
source-reader guards; a configuration catalog does not authorize lossy XML rewriting.

## Proposed data and API boundary

Add a portable `ConfigDefinitionCatalog` in `poe-optimizer-data`, exposed through the selected
data snapshot and injected into configuration consumers. Extract the complete source
metadata catalog rather than selecting definitions by the current corpus or skill profile.
The first package integration can contain this catalog as a `configuration` section. Its
public read API should accept the catalog independently of `CompiledGameData`, so generic
input preservation and metadata inspection do not require Spark/Mace calculation compilation. Keep
catalog scalar-kind metadata in data/core; the data crate must not depend on import. The
borrowed source projection stays in import, and admission/preparation bridges that source
model to the injected catalog.

Proposed shapes, subject to the source validation gates below:

```rust
struct ConfigDefinitionCatalog {
    schema_version: u32,
    source: ConfigSourceIdentity,
    definitions: Vec<ConfigDefinition>, // all ordered source rows with a var
}
struct ConfigDefinition {
    id: ConfigDefinitionId,            // source-bound occurrence, not just key
    key: String,
    source_order: u32,
    source_widget: ConfigWidgetKind,
    scalar_kinds: Vec<ScalarKind>,     // list choices can mix numbers and strings
    options: Vec<ConfigOption>,       // exact ordered typed values and labels
    defaults: ConfigDeclaredDefaults,
    source: ConfigDefinitionSource,
    behavior: ConfigBehaviorEvidence,
}
struct ConfigDeclaredDefaults {
    input: Option<Scalar>,
    placeholder: Option<Scalar>,
    option_index: Option<u32>,         // original one-based defaultIndex
}
```

Reuse `core::options::Scalar` for boolean/number/text values; keep numbers finite at the
portable catalog boundary. An absent default must remain distinct from explicit false,
zero and empty text. Retain exact string bytes and original scalar lexemes in the shared
source projection; a parsed `Scalar` alone cannot preserve XML representation. A mixed
number/text list must not become a text-only enum or stringify numeric options.

`ConfigDefinitionId` should identify an occurrence within its source/data identity. Build a
secondary `key -> ordered definition indexes` lookup, not an overwriting map. Exact source
file/hash plus row/quest provenance bind definitions; source option order is part of that
identity. Hashes for Lua callback spans can identify unsupported behavior without embedding
callbacks or pretending that a function name describes complete semantics. Tooltip/UI
metadata can be separate, optional presentation evidence.

The implemented source-preserving import seam exposes immutable ordered sets through
`ConfigurationProjection::sets()` and `active_set()`. Each `ConfigSetProjection` exposes
`inputs()`, `placeholders()`, `blocks()`, `unknown_records()` and `records_in_source_order()`;
records retain source range/text and ownership. Definition lookup should enrich this
projection without deleting unknown fields, choosing a fallback build, trimming quest
strings or marking mechanics supported. Duplicate authored XML inputs remain distinct from
duplicate definition rows. The generic projection currently rejects duplicate same-kind
Input/Placeholder names, while preserving cross-kind order; retain that policy until a
reviewed last-write policy is implemented. Inactive set retention must not activate its
modifiers.

Use a separate capability-aware resolution step:

```rust
resolve_configuration(
    projection: &ConfigurationProjection,
    definitions: &ConfigDefinitionCatalog,
    capabilities: &ConfigConsumerCapabilities,
    context: &ConfigResolutionContext,
) -> Result<PreparedConfiguration, ConfigAdmissionReport>
```

`PreparedConfiguration` binds selected data and source identities, effective scalar origins,
ordered supported producers and explicit unresolved dependencies. Defaults should resolve
through versioned source operations; dynamic callbacks stay reference-only until their full
behavior is translated and tested. `ConfigBehaviorEvidence` can record `ReferenceOnly` or a
versioned implemented handler/effect binding. Data cannot grant capability merely by naming
a handler: the engine must validate its vocabulary, scope and all downstream consumers.
Context must distinguish character inputs, actor/skill dependencies and encounter state;
metadata lookup must not eagerly execute all producers or invent missing context.

## Reuse and migration

`QuestData` already owns six numeric quest selectors/defaults; `ActorData.spirit_quests`
already owns three ordered modifier-producing quest records. Reuse those numerical owners
while introducing definition references. Do not duplicate their values or add a tenth
quest-name branch to the native profile. Generated list quests should eventually bind each
reviewed exact choice to source-ordered modifier records and requirements. Existing actor
modifier IR can represent a subset, but a parseable Armour/resistance/movement choice does
not make charm, flask, recovery, ailment, cooldown or every alternate choice implemented.
Keep unsupported effects explicit and migrate old positional quest consumers only with
source-order and no-double-count checks.

`EncounterData` already owns the resistance penalty, default boss and selected resistance/
level constants. Reference these values when mapping corresponding definitions; their
presence does not implement all source placeholder mutations. Current native key/range
checks can be moved behind reviewed consumer bindings without changing their numerical
scope. The generic `EvaluationContext` scalar maps remain useful output evidence, but lose
occurrence, provenance and default origin; they are not the source configuration model.

The actor condition enum currently represents attribute comparisons and movement-penalty
suppression. The generic modifier engine has broader condition primitives, but arbitrary
configuration conditions such as Combat/Effective, ailments and recent actions require
source-owned dependencies and actual receiving consumers. Do not turn definition metadata
into unvalidated condition flags or treat a UI `ifCond` as an evaluated condition.

## Bounded delivery and parity gates

1. Extract/authenticate all definition metadata and exact quest choices with ordered
   duplicate rows, scalar kinds, declared defaults and callback source evidence. Test both
   cold and warm construction against the original source. Source changes that cannot be
   represented must fail extraction or remain explicitly reference-only.
2. Attach the catalog to generic source-preserving configuration projection/admission.
   Exercise the five corpus inputs without dropping their 170 placeholders or altering
   multiline choices. Native unsupported-mechanic failures must remain visible afterward.
3. Translate existing native quest/encounter bindings through the catalog with unchanged
   scope before adding effects. Test injected renamed keys, reordered definitions/options,
   changed defaults and modified numerical records; no production fixture-ID decisions.
4. Add complete configuration producer families with source callback and full-build parity.
   Allocate preparation data once, preserve configuration-before-equipment-before-passive
   source ordering, then use immutable prepared state in native search hot paths.

Independent tests must cover missing versus false/zero/empty inputs; defaultIndex versus
initial placeholders; explicit zero across all numeric widgets; dynamic placeholder writes;
repeated source keys and repeated authored nodes as separate cases; scalar type mismatches;
exact and unlisted option strings; legacy aliases; unknown preserved keys; source strings
with LF/CRLF/tab and named entities; active/inactive set selection; disabled custom blocks;
multiple record effects and unsupported receivers; cold/warm source dispatch order; and
selected-data/owner binding. UI visibility must not silently suppress source effects.

No product answer blocks this metadata/projection phase. General normalized skill, actor,
trigger/minion and cross-set architecture should be discussed before claiming broader build
admission. The endpoint remains a fully native, data-driven evaluator with optional PoB
parity loading; this catalog is one reusable boundary toward it.
