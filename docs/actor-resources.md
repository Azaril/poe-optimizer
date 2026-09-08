# Shared actor attributes and maximum resources

The native evaluator prepares one immutable actor component from selected game data,
class/tree inputs, quest selections and ordered global modifier records. Spark, Mace Strike
and controlled equipment/support requirement checks share this calculation. It performs no
Lua calls, process creation or I/O. The intended full evaluator will reuse this stage as
item, passive, skill and supporting-actor coverage expands.

## Data and calculation boundary

The `GameDataPackage.actor` model (introduced in schema 6) owns normalized records, literal modifier
rules, source-derived precision and quest/default constants. Existing class and level
bases remain in the character model. `CompiledGameData` validates and compiles the selected
snapshot; callers can inject a reviewed or custom dataset without changing evaluator code.
A custom dataset is identified separately and does not inherit source-parity certification.

`ActorModifierRecord` preserves target, numeric operation or flag value, source, flags,
keyword flags and ordered condition tags. Unsupported selected forms fail admission.
The reviewed textual rules expose a bounded set of attributes, Life, Mana, Spirit and
Accuracy operations and inherent-attribute flags. The broader numeric record model is
not a claim that every recorded mechanic is admitted by complete build profiles.

`CompiledGameData::prepare_actor_resources` consumes level, `ActorQuestSelection`,
`CharacterInput` and ordered modifier layers (local first, then parents). Its
`PreparedActorResources` is bound to that compiled dataset and input metadata. It keeps
numeric output and a private shared binding; it retains no XML, modifier database or
source strings. Reusing it with another compiled owner or different character inputs fails.
The compatibility Spark/Mace entrypoints delegate to the same stage. The empty-record
path avoids building an owned database while preserving the shared operation order.

The implementation follows the pinned source stages:

1. Assemble class, level and enabled quest bases from injected data.
2. Calculate Strength, Dexterity and Intelligence in exactly two ordered passes. Update
   the twelve source comparison conditions after each pass; do not iterate to convergence.
3. Apply inherent attribute bonuses with source suppression, halving, doubling and
   Dexterity Accuracy override behavior.
4. Calculate maximum Life, Mana and Spirit, preserving zero overrides, donor conversion,
   Extra/INC/MORE/Total order, rounding and minimums. Derive global Accuracy and thresholds.

The raw maximum-pool function includes donor conversions and Chaos Inoculation for
function parity. Complete Spark/Mace admission rejects those mechanics because their
receiving defences, reservation and downstream effects are unfinished. A correct donor
value cannot certify the complete conversion. Actor recursion, minion inheritance,
reservation and general defensive/offensive pipelines remain separate work.

## Source configuration and requirements

The shared importer accepts source `Config/ConfigSet/CustomModifierBlock` elements and
legacy `Input name="customMods" string="..."` configuration. It preserves exact source
fragments, block order/title/enabled state, duplicate lines and per-line rule/roll evidence.
Active unknown or partially parsed lines fail. Disabled blocks preserve inactive text.
Legacy input and explicit blocks cannot be mixed ambiguously. Limits include 8 KiB decoded
text, 64 enabled lines, 16 blocks, 512 mapped records and 64 KiB aggregate encoded actor
source. Current Mace realization caps the resulting diagnostic at 8 MiB; arbitrary custom
rule fan-out must stay within the data model bounds.

PoB's XML reader preserves literal whitespace in legacy attributes. A narrow native XML
compatibility gate permits raw CR/LF/tab only in that exact unnamespaced actor input;
the actor parser reads its original attribute span. Other attribute/path normalization
mismatches still fail. Native export preserves source exactly. Reference realization
checks the upstream migration to blocks and the exported block contents explicitly.

Controlled catalog requirements privately calculate available attributes for each
class/tree through the same Rust actor stage. This introduces a pure Rust import-to-engine
dependency for semantic catalog admission; the generic container reader does not invoke
Lua. It avoids maintaining a second attribute formula or accepting caller-supplied available
attributes. Normalized passive actor effects and selected global equipment records enter this shared
preparation layer. [Receiving defences](receiving-defences.md) extends it with ordered ratings
and resistance calculation through `prepare_actor` / `evaluate_actor`.
Available attributes must be whole, nonnegative and representable in the requirement
model; casts must not silently saturate. Optional PoB controlled search uses this tested
attribute preflight, while its build calculations remain explicit PoB evaluations.

## Prepared searches and reports

Problem schema **6** enables authored attribute/resource configuration, with existing local weapons,
class/ascendancy, finite passive and support-loadout axes. Schemas 1–5 reject authored actor
blocks, legacy custom modifiers and explicit Spirit quest keys. Report schema **7** uses
scope `mace_actor_local_weapon_class_passive_support_loadouts_v1`.
The example fixes actor configuration for a run; actor modifiers are not a new optimizer
axis yet. Its objective and constraints remain configurable.

```powershell
cargo run --release --no-default-features --locked -- search-experimental --backend native --problem examples/mace-actor-search.json --jobs 4 --max-proposals 8192 --max-evaluations 4412
```

`PreparedMaceCandidates` stores one actor component per tree/scenario alongside weapon
and support components. `actor_preparations` reports that numerical setup work and its
elapsed time. A zero preparation `calculations` field means no full build evaluation;
it does not mean that resource preparation is free. Candidate skill output is freshly
calculated and no candidate-result cache is stored. Selected composition failures remain
associated with their affected axis. Baseline and finalist are complete fresh evaluations
counted in the same attempt budget. Empty legal domains still use zero build attempts.

`player.spirit` is a pool-points metric for maximum Spirit before reservation in both
backends. It is appended to the native catalog, preserving previous indices. Mace's
`selected_average_hit` remains explicitly unavailable. Current native profile evidence uses
Spark media version 6 and Mace version 8 and retains actor source evidence and numeric
attributes/resources. Tree media version 3 adds connected source views and physical attribute
choices; see [passive/equipment assembly](passive-equipment-assembly.md).

## Evidence and limits

Validation combines independent pinned-source function tests (cold and warmed Lua),
fresh complete PoB build comparisons, source export/reimport checks, custom-data injection,
full typed/document candidate matrices, serial/Rayon searches and allocation regressions.
Each checks a different boundary. Typed/document agreement alone is not an independent
numerical oracle. Six original calibration goldens and the source pin remain unchanged.

The benchmark's explicit `--candidate-set actor-resources` uses the actor fixture and
local weapon/support choices with all selected-data tree choices. Setup, fresh equivalence
calculations and timed API layers are reported separately. Bounded Mace throughput does
not establish general-build speed or optimizer quality. Dated results and remaining work
belong in [the living implementation record](implementation.md).
