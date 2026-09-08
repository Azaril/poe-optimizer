# Native game-data packages

The native evaluator loads a versioned JSON package into `GameDataSnapshot`, compiles it
once into `CompiledGameData`, and shares immutable data across prepared builds and Rayon
workers. The native calculation path uses injected records; it performs no configuration
parsing, I/O or data hashing. This implements the current-profile slice of the
[game-data boundary](game-data-boundary.md). Delivery evidence and remaining gates live in
[implementation.md](implementation.md).

## Included data and current limits

The reviewed package contains complete structural ordinary-node metadata, capability-admitted
passive views, the existing class roots and selected ascendancy passives, character
resource and accuracy parameters, level-one Spark/Mace identities and values, three reviewed supports,
two weapon bases, seven jewellery bases with source-derived implicit ranges and requirements,
five local item-modifier grammar/mapping rules, shared actor configuration,
quest rewards/defaults, defence coefficients/caps, 100-level
monster armour/evasion tables, encounter defaults, typed owned passive effects and explicit
level/attribute requirements for the included skills, support and weapon bases. Support
color and per-color aggregate attribute costs also come from data. Display
stat text is source evidence; typed effect IDs and values drive the calculation.

The `actor` section supplies the shared attribute and maximum-resource stage used by Spark
and Mace. Its normalized modifier records retain typed targets, numeric or flag effects,
source, flags/keyword flags and ordered attribute-condition tags. The section includes
359 reviewed custom-modifier templates, the complete 40-record source precision table,
three Spirit quest records and the additional source constants for attribute bonuses,
pool thresholds and Spirit. Existing Life/Mana/Accuracy balance fields remain in
`character`; existing quest fields remain in `quests`.

The package is explicitly partial. It records 4,109 structural ordinary nodes and 4,758
source views: 1,282 have complete admitted effects and 3,476 are explicitly excluded.
Class/ascendancy replacements and physical attribute choices resolve before capability
admission. Its ascendancy effect coverage includes four directly connected
ascendancy small nodes with signed BASE player resistance effects. Local item rules admit
normal/rare supplied Maces with the five reviewed modifier families; see [local weapons](local-weapons.md).
It also represents all seven zero/one/two-support loadouts from
Brutality I, Heavy Swing and Rapid Attacks I; see [support loadouts](support-loadouts.md).
The two Mace weapon IDs are compiled capability slots whose records come from the package.
Unknown fields/operations, malformed identities, missing records and unsupported selectors
reject. Gameplay input bounds and supported operations remain enforced in Rust.

Structural tree content and class attributes remain bound to the reviewed tree artifact
and exact source compatibility guard. Custom numerical/effect records can change; editing
topology or its provenance requires the remaining source/compatibility migration. The
supplied minion build and unrestricted native evaluation remain unsupported.

## Actor configuration and modifier records

`ActorModifierRecord` represents numeric BASE/INC/MORE/OVERRIDE or boolean FLAG effects.
Supported numeric targets cover attributes, Life/Mana/Spirit, Accuracy and the explicit
inputs to the maximum-resource, admitted receiving-defence and movement functions. `ActorModifierTag::Condition` preserves ordered
OR variables and negation; `Global` retains exact nonrestricting item-scope metadata.
Multiple tags retain source order. Global records require zero
flags and keyword flags. Unknown fields, targets, tags and incompatible operations reject.
The raw stage can represent donor conversions and Chaos Inoculation, but this does not
admit those mechanics in a complete build: receiver defences and the full immunity effects
remain outside the supported native profiles.

`actor.modifier_rules` contains exact text templates, capture kinds and complete ordered
effect mappings. Signed decimal BASE captures and unsigned integer INC/MORE captures retain
the source grammar; reduced/less forms carry their source-derived negative multiplier.
Fixed phrases can emit flags or zero-valued overrides. For example, the reviewed rules
include `+5 to Strength`, `15% more maximum Mana`, `Removes all mana`, and the suffix
` if Strength is higher than Intelligence`. Combined attribute wording that emits an
additional `All` bookkeeping record is not silently reduced to three attribute records.
The configured templates are a bounded parser surface, not unrestricted PoB modifier syntax.

`actor.high_precision_mods` retains all 40 source entries and their operations. The engine
selects the two reviewed MORE entries from this explicit table; it does not inherit a
pinned precision table from a primitive helper. Custom precision data is part of the
selected package and its identity.

`actor.spirit_quests` retains three independent checkbox keys, defaults and exact modifier
sources for King In The Mists, Ignagduk and Lythara. Their reviewed BASE values are 30, 30
and 40; all three source checkbox defaults are enabled independently of character level.
Keys must be unique and disjoint from the existing six quest records. Build admission
also rejects collisions with other recognized configuration fields. Quest choice lists,
reservation and supporting skill effects are not inferred from these records.

## CLI loading and authoring

`evaluate`, `metrics`, `benchmark-native`, `search-experimental` and `search-build` accept `--data <package.json>` for the native
backend. Omission selects the embedded reviewed package through the same validated loader.
An external byte-identical copy has the same identity and reviewed status. Other packages
are custom/unreviewed unless the host supplies an expected digest from an external review.
An expected hash verifies identity; it does not establish numerical parity.

```powershell
cargo run --no-default-features --locked -- evaluate examples/native-witch-entrance.xml --data crates/poe-optimizer-data/data/game-data.json --raw
cargo run --no-default-features --locked -- benchmark-native examples/native-witch-entrance.xml --data crates/poe-optimizer-data/data/game-data.json --mode prepared --jobs 4 --evaluations 10000
```

`--data-sha256 <lowercase-sha256>` requires `--data` and rejects mismatched bytes without
falling back. Native-only packaging needs no PoB checkout. The PoB backend rejects data
selection because it uses its own reviewed source/runtime.

For local edits, copy the package to a separate file and change supported records. Refresh
the section digests with the developer authoring helper, which validates the result and
requires a new output path:

```powershell
Copy-Item crates/poe-optimizer-data/data/game-data.json runs/custom.edit.json
# Edit the copied JSON's supported values before sealing it.
cargo run -p poe-optimizer-data --example seal_package --locked -- runs/custom.edit.json runs/custom.json
cargo run --no-default-features --locked -- evaluate tests/fixtures/calibration/spark-mapping.xml --data runs/custom.json
```

`seal_package` decodes bounded regular-file input, refreshes section hashes, validates the
package, then writes it. It does not derive values from PoB, confer trusted origin,
or overwrite existing output. The native executable can read the resulting package without
recompilation. Do not edit the reviewed artifact or original numerical goldens to manufacture
parity. The optional [pinned source exporter](game-data-extraction.md) produces all current
sections from source; broader source-version updates remain unfinished.

To regenerate the reviewed package from the pinned PoB checkout, use the development CLI:

```powershell
cargo run --locked -- extract-game-data --output runs/extracted-game-data.json
```

It also writes an `.extraction.json` evidence companion. Native evaluation can load the
result through `--data`; extraction is separate from the native runtime.

Native search loads the selected snapshot once and shares it between the
controlled catalog and evaluator. Skill/support identities, generated XML, weapon names,
quest selectors/defaults and requirement checks consume that snapshot. PoB reference binding
still accepts only the reviewed default content.

```powershell
cargo run --no-default-features --locked -- search-experimental --problem examples/mace-search.json --data runs/custom.json --jobs 4 --max-evaluations 10
```

Search rejects candidates that fail the closed profile's level/attribute requirements
before calculation. The required value for each attribute is the maximum of individual
item/active/support requirements and the support-color aggregate. Item level is separate
from equip-level requirements. Reports retain rejected choices and their required/available
values; if every locked choice fails, `empty_legal_domain` reports zero evaluations and
writes no XML. Diagnostic `evaluate` remains distinct from search legality. See
[controlled mutations](controlled-mutations.md) for the supported scope.

## Library composition

```rust,ignore
use std::sync::Arc;
use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy};
use poe_optimizer_native::{CompiledGameData, NativeBackend};

let snapshot = GameDataLoader::from_bytes(
    &bytes, &TrustPolicy::AllowCustom, &LoadLimits::default(),
)?;
let data = Arc::new(CompiledGameData::compile(Arc::new(snapshot))?);
let backend = NativeBackend::with_data(Arc::clone(&data), host_clock)?;
let prepared = backend.prepare(&request)?;
let result = backend.evaluate_prepared(&prepared, budget)?;
```

The host supplies bytes and clock. `NativeBackend::new` / `with_clock` are convenience
constructors for the compiled reviewed default. Explicit `with_data` never silently selects
another package. The default uses a cache for immutable data initialization; it does not
change the dataset of existing instances or cache calculated build results.

Prepared inputs retain shared data ownership and originating backend identity. Distinct
instances with the same verified data/semantic identity may reuse them. Different datasets
reject before calculation. `PreparedEvaluation::calculate` uses the retained data directly.
`CalculationBackend::identity` optionally advertises full instance identity; the core engine
checks it in addition to capability IDs. Native always advertises it.

Pure Spark/Mace `evaluate_with_data` functions receive `&CompiledGameData` explicitly.
Legacy `evaluate`, `evaluate_with_character`, `data()` and default-character helpers obtain
the same reviewed package. They contain no duplicate balance-value database. New hot paths
should use the injected APIs. `CompiledGameData::prepare_actor_resources` produces
`PreparedActorResources` bound to the exact compiled instance. Both skills consume that
shared actor stage; its base/class/quest and modifier inputs are prepared before repeated
numerical evaluation.

## Identity, reports and exports

`BackendIdentity.data` records game, release, schema, semantics version and verified package
byte digest. Data identity is separate from implementation/source fingerprints and trust.
Changing only a numeric record changes data identity while retaining the implementation
fingerprint. JSON formatting changes can change the byte digest; that conservatively prevents
cache reuse. All section digests must also match canonical typed records.

Evaluation reports use schema **3**; saved assessment accepts schemas 2 and 3, and schema 3
native reports require a valid data identity. Native results carry reviewed/custom trust in
warnings, so it remains visible without `--raw`; raw mode additionally retains structured
`game-data` and configured entrance-effect diagnostics. Controlled-search schema **2** records selected data identity/trust, requirement rejection
evidence and the existing evaluation budget ledger. Expanded class/tree search uses report
schema **3**, adding ordered tree choices and canonical admission evidence. The same selected
snapshot supplies class attributes, resolved physical/effective entrance views, graph
projection, generated XML and numerical configuration. The schema-3 problem adds paid ascendancy choices and uses report schema **4**. Schema-4
support-loadout problems use report **5**; schema-5 local-weapon problems use report **6**.
Schema-6 actor-customization problems use report **7**. The lazy schema-7 `search-build`
problem uses report **8**; graph problem **8** adds authored receiving scope and report **9**.
Graph problem **9** adds local armour/report **10**; graph problem **10** adds Body Armour
and movement/report **11**. Graph problem **11** adds action timing/report **12**. Mace profile evidence uses media **9**, and Spark uses media **7**, with normalized actor modifier evidence and maximum Spirit
alongside the existing outputs. Mace additionally retains exact parsed item provenance and
prepared local weapon stats. Native-tree
diagnostics use version **3**, with separate ordinary/ascendancy counts, physical/effective
source-view keys and explicit attribute choices. Its native export companion retains
structured trust and the selected package path as a reload hint. Benchmark schema **2** reports
structured `data_trust`, identity and separate backend/data initialization time.

Native `evaluate --export <file.xml>` and controlled native search write `<file.xml>.data.json`
next to XML. The companion records backend/data identity and XML hash for reload. Output
collisions reject before evaluation. XML bytes remain source-preserving; re-evaluate custom
exports using the matching data package. A filename hint is neither content identity nor
trust. Existing source-only imports still preserve input bytes.

## Schema migration

Current package schema **13**, semantics **`poe2-native-profiles-v13`**, adds the complete
constructed `skill_identities` metadata section: 967 gem declarations / 966 final gems and
1,439 effect declarations / 1,436 final effects. Original and constructed references remain
distinct, including 16 missing-reference records. `GameDataSnapshot::skill_identities()`
exposes immutable identity lookup without native skill compilation. All preceding 22 section
contents and digests remain unchanged; the package now has 23 sections. Regenerate older
packages. This section grants identity evidence only, not numerical capabilities. See
[skill source and identity contracts](skill-source-and-identities.md).

Package schema **12**, semantics **`poe2-native-profiles-v12`**, adds the complete
ordered `configuration` metadata section. It contains 564 definition occurrences (563 keys),
17 generated quests, exact typed options/defaults and 540 inert callback descriptors.
`GameDataSnapshot::configuration()` exposes an immutable indexed catalog without requiring
native skill compilation. All preceding 21 section contents and digests remain unchanged;
the package now has 22 sections. Regenerate older packages; recognition of configuration
metadata does not grant support for its effects. See [configuration definitions](configuration-data-proposal.md).

Package schema **11**, semantics **`poe2-native-profiles-v11`**, adds the `action_speed`
and `direct_action_timing` sections, source MAX/positive queries and shared ordinary timing.
The package has 21 sections, 402 fixed bases, 359 actor templates, 87 formatting keys and
1,282 admitted passive views. Movement consumes the same resolved action result as offence;
its independent neutral-action default was removed. Regenerate older packages and review
custom data; see [action timing](action-timing.md).

Package schema **10**, semantics **`poe2-native-profiles-v10`**, introduced the `movement`
section and source-selected Body Armour. Every armour base has an explicit nullable
`movement_penalty`, preserving absence versus zero. Source-generated modifier metadata,
movement query/default/rounding data and exact division captures remain injectable data.
The package has 402 fixed bases, 347 actor templates, 83 exact item-format keys and 1,282
admitted passive views. That checkpoint required neutral ActionSpeed until offence also
consumed it in schema 11. Previous source values/tree semantics remain unchanged; regenerate older
packages and review custom changes. See [Body Armour and movement](body-armour-movement.md).

Package schema **9**, semantics **`poe2-native-profiles-v9`**, introduced `armour_bases`
for source-selected fixed Helmet/Gloves/Boots, nine local paired grammar aliases and
`item_formatting` for source-keyed pre-parser numeric formatting. Configuration records
do not undergo item formatting.
The 288 base records and 329 grammar templates are injected configuration. Local-only
paired records must be consumed by the item pipeline; global paths reject them. Tree
schema 3 and previous receiving/passive semantics remain unchanged. Older packages
require regeneration and review of custom changes. See [local armour](local-armour.md).

Package schema **8**, semantics **`poe2-native-profiles-v8`**, introduced
`receiving_defence` with ordered source query groups. Defensive passive scalars move into
ordered actor records; only offence remains in scalar passive effects. Unknown or mixed
legacy defensive representations reject. See [shared receiving defences](receiving-defences.md)
for supported operations, scenario binding and schema-8 search.

Package schema **7**, semantics **`poe2-native-profiles-v7`**, introduced `jewellery_bases`
and `passive_exclusions`. Passive effects are keyed by physical node and source-view
selector (`base`, class, ascendancy or explicit attribute option), retaining the effective
node separately. Effects may contain scalar operations and compiled actor records. The
schema-3 tree retains 4,141 physical records, including 28 roots, 4,109 ordinary nodes and
four ascendancy nodes; 773 physical records remain excluded. Runtime selection checks every
effective view, connectivity, ownership and explicit attribute choice. Custom data may edit
supported values within the source-reviewed capability-key ceiling; it cannot admit an
excluded view, invent topology or add an unreviewed operation. See
[passive/equipment assembly](passive-equipment-assembly.md).

Schema **6**, semantics **`poe2-native-profiles-v6`**, introduced the required `actor`
section. It contains normalized actor records/rules, additional source constants, the complete
precision table and three Spirit quest selectors/defaults. All preexisting non-manifest
sections remain unchanged. Schema-1/2/3/4/5 packages require regeneration and review of custom
changes; increasing the schema number alone does not supply the missing configuration.

Package schema **5**, semantics **`poe2-native-profiles-v5`**, adds `item_modifier_rules`
and `character.critical_chance_cap`. Rule templates, numeric capture kinds and stat/operation/
scope mappings are configuration. Supplied item values are validated separately and compiled
once per weapon; Rust defines operation and parsing semantics. The global critical cap and
two-decimal offence rounding apply to both Spark and Mace. See [local weapon data](local-weapons.md).
Schema-1/2/3/4 packages require regeneration and review of custom changes.

Package schema **4**, semantics **`poe2-native-profiles-v4`**, moved `mace.brutality`
to the source-derived `supports` section and adds Mace skill types and explicit zero cost.
See [support-loadout configuration](support-loadouts.md#configuration-and-calculation) for
fields and compatibility. Older packages require regeneration and review of custom changes.
The following historical resistance migration is superseded by schema-8 ordered records.

Package schema **3** and semantics **`poe2-native-profiles-v3`** rename `entrance_effects`
to `passive_effects`. Each record has an explicit nullable `ascendancy_id`: null selects
an ordinary class entrance, while an internal ID selects that ascendancy's admitted node.
Selectors include class, owner, physical ID and effective ID; every admitted view has exactly
one record. That historical migration used bundle schema **2** with 44 physical nodes;
schema 7 replaced those effect selectors with complete source-view keys.

The schema-3 scalar operations were `fire_resistance_flat`, `cold_resistance_flat`,
`lightning_resistance_flat`, `chaos_resistance_flat` and `elemental_resistance_flat`.
Among the passive-effect operations, these admit signed finite values in -1,000,000..1,000,000. Other passive operations
retain nonnegative validation. Source stat strings remain provenance; custom typed values
and supported operations may change, retaining custom/unreviewed trust.

Both supported native pipelines sum the applicable BASE values, then truncate toward zero
and apply injected floor/cap values. `defence.resistance_maximum_cap` separately caps the
base player maximum, sourced from PoB's global maximum rule. Elemental bonuses, penalties and quest rewards do not
apply to chaos. Fractional custom floor/cap values are also truncated, correcting the older
custom-data behavior to match source. Reviewed integer-default outputs remain unchanged.
Schema 8 additionally admits supported conditional and INC resistance records through the
shared receiver. Maximum-resistance, MORE, override, conversion and other-actor mechanics
remain excluded.

Schema-1/2/3/4/5/6/7 packages fail explicitly. Regenerate with `extract-game-data`, then review/reapply
custom edits and reseal; changing the version number alone cannot migrate missing records
or the retained-tree artifact. Schema-2 level/attribute/support-color requirement records
remain unchanged. PoB source revision, full source snapshot and numerical goldens stay fixed.

## Validation and update procedure

The default package SHA-256 is
`fdd924e0449d06c338df95cf8e309986abf07a73e3f2f0131acf222d5d24ae30`.
The partial tree SHA-256 is
`31cac8a09de2babc34e450d0caf975c45aca3caf222853d863dcad607c6f8779`.
The full source snapshot/manifest and six independent goldens remain unchanged; the partial
retained-tree bundle is unchanged by the receiving-defence migration.

```powershell
cargo test -p poe-optimizer-data --locked
cargo test -p poe-optimizer-engine --test data_injection --locked
cargo test -p poe-optimizer-cli --no-default-features --test native_data --test native_data_cli --locked
cargo test -p poe-optimizer-pob --test game_data --locked
cargo test -p poe-optimizer-cli --test native_passive_parity --locked
```

The optional source tests verify package records against pinned Lua data, modifier parsing
and source expressions, including warmed functions and every effective entrance. Earlier
receiving-defence validation covered 3,642 independent cold/warm parser inputs for its 320
reviewed templates, complete precision records and actual Spirit quest configuration callbacks.
Independent original `PassiveTree.ProcessStats` tests compared all 1,270 views admitted at
that checkpoint in cold and warm source runs; jewellery tests checked every admitted base.
These historical counts precede local armour and movement; current coverage and validation
counts are recorded in the [living implementation checkpoint](implementation.md). Those parser
comparisons establish normalized input parity, not complete native build parity. The full
matrix checks complete native builds against fresh PoB evaluations. Custom-package tests
are isolation and controlled-change evidence, not PoB parity claims. Preserve source review,
strict compatibility and new differential evidence when deliberately updating a package.
