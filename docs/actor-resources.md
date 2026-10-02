# Shared actor attributes and maximum resources

> Status, 2026-10-02: this document records retained Engine/Import component behavior
> and historical NativeBackend integration. The NativeBackend crate, its CLI and
> Import's controlled-build/actor-assembly coordinators are removed. See the [execution overview](data-and-evaluation-overview.md) for the active
> owned evaluator and remaining retirement work.

The retained Engine calculation prepares one immutable actor component from selected game data,
class/tree inputs, quest selections and ordered global modifier records. Legacy Spark/Mace
calculations share this component; Import's former requirement coordinator is removed. It performs no
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
source. The removed Mace realization adapter capped its diagnostic at 8 MiB; retained
custom rule expansion still observes the data model bounds.

PoB's XML reader preserves literal whitespace in legacy attributes. A narrow native XML
compatibility gate permits raw CR/LF/tab only in that exact unnamespaced actor input;
the actor parser reads its original attribute span. Other attribute/path normalization
mismatches still fail. Native export preserves source exactly. Reference realization
checks the upstream migration to blocks and the exported block contents explicitly.

The retired controlled catalog calculated available attributes for each class/tree through
the same Rust actor stage. Its Import-to-Engine coordinator avoided a second attribute
formula and caller-supplied available attributes; that coordinator and its exclusive
admission tests are now removed. Independent actor modifier parsing, expansion and
diagnostic assertions remain. Engine's [receiving defences](receiving-defences.md) retain
ordered ratings and resistance calculation through `prepare_actor` / `evaluate_actor`.
The former controlled-search preflight is historical evidence, not an active API.

## Historical prepared searches and reports

The finite Mace actor catalog, benchmark and later graph search adapter are retired.
That adapter reused shared actor components and checked requirements before candidate
calculation. Independent Engine actor/condition tests and PoB reference fixtures remain;
NativeBackend integration tests are removed. See [retirement inventory](legacy-retirement.md).

The old `player.spirit` metric represented maximum Spirit before reservation in both
backends. It was appended to the native catalog, preserving previous indices. Mace's
`selected_average_hit` was explicitly unavailable. Historical native profile evidence used
Spark media version 6 and Mace version 8 and retains actor source evidence and numeric
attributes/resources. Tree media version 3 adds connected source views and physical attribute
choices; see [passive/equipment assembly](passive-equipment-assembly.md).

## Evidence and limits

Historical validation combined independent pinned-source function tests (cold and warmed Lua),
fresh complete PoB build comparisons, source export/reimport checks, custom-data injection,
full typed/document candidate matrices, serial/Rayon searches and allocation regressions.
Each checks a different boundary. Typed/document agreement alone is not an independent
numerical oracle. Six original calibration goldens and the source pin remain unchanged.

Historical bounded-Mace benchmark results remain in the implementation record; the
retired harness is not a current performance tool. General-build throughput remains open.
