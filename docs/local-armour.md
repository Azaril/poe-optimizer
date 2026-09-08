# Local armour equipment and rating objectives

The native Spark and Mace pipelines accept source-reviewed fixed helmet, glove and boot
bases. [Body Armour and movement](body-armour-movement.md) extends this pipeline to the
fourth slot in schema 10. Item and rule data is injected through the same immutable package used for passives,
requirements, resources and receiving defences. Native execution uses Rust only; the pinned
PoB checkout is an explicit development oracle. This is a complete calculation path for its
admitted forms, with other mechanics rejected at admission.

## Data and supported inputs

Package schema **9** introduced `armour_bases`; the current schema **10** retains these
three-slot inputs within its expanded armour data. Each record
contains source identity, display name, slot, equip requirements, default quality and fixed
Armour/Evasion/Energy-Shield bases. Extraction selects **288** complete fixed bases from
**649** final source definitions by their represented field shape. No base-name list or
patch-specific base values are embedded in the evaluator. The current extraction excludes
implicit-bearing and otherwise unrepresented base shapes. The current four-slot package has **35** verified source files and **19** sections;
this three-slot subset retains its source-defined bases.

Normal/rare items use source text, item level 1–100 and quality 0–20. Rare item text must
include the rare name and base name. These bases have zero implicits. Explicit level
requirements and final attribute requirements retain the existing equipment admission
checks. A supplied synthetic rare modifier combination is not proof of obtainable affix
tiers, cost or acquisition legality.

Ordered actor modifier rules provide individual Armour/Evasion/Energy-Shield BASE/INC,
paired Armour-and-Evasion, Armour-and-Energy-Shield and Evasion-and-Energy-Shield BASE/INC,
and Defences INC. The nine new plain paired rule aliases extend the grammar to **329**
templates at that checkpoint; movement extends the current grammar to 347. Numeric captures and accepted wording come from the selected package. Source
local consumption is record-specific: untagged records with zero flags are eligible;
explicit Global markers and attribute conditions remain global. New local-only paired
stats reject if left outside their local consumer, including configuration and passives.

Per-level item bases, Ward, block, alternate quality,
ArmourData overrides, slot-specific conditions, conversions and item-granted skills remain
unsupported. These effects cannot be silently discarded to admit an item.

## Item source formatting

The required `item_formatting` section retains **83** exact source scalability keys, including the original 77, numeric
precision and display rules. Item text passes through this stage before modifier parsing,
including surviving global weapon/amulet modifiers and implicit values. Case and literal
number specialization matter: missing source keys preserve raw values. Configuration
modifiers do not use item formatting. For example, the reviewed item key formats `+17.5
to Evasion Rating` as `+18 to Evasion Rating` before local calculation. Original text,
raw captures and effective formatted captures remain distinct evidence; exports retain
original source bytes. Rust implements formatting operations; selected data supplies the
matching keys and precision policy.

## Rust calculation seam

`CompiledGameData::prepare_armour_with_source` binds base identity, quality, item level,
source identity and normalized records to the selected compiled dataset. The compatibility
`prepare_armour` helper rejects bases requiring generated source-bound movement records. `PreparedArmour` retains fixed local
stats and a compiled surviving global program. It is immutable and can be shared across
workers. `ArmourSlots` borrows optional prepared helmet, glove, boot and body components.

Local arithmetic follows the original item calculation: sum individual and paired bases,
apply summed local INC, multiply quality separately, then round. The Energy-Shield BASE
and INC pair orders differ in source and remain distinct. Local negative values are
retained; final receiving aggregation applies the source clamp. A local stat is never
flattened into a global BASE surrogate.

`prepare_actor_with_armour` / `evaluate_actor_with_armour` extend complete actor preparation.
They validate data ownership and slot placement. Callers append surviving global programs
exactly once in source order: configuration, Weapon 1, Helmet, Body Armour when present,
Gloves, Boots, Amulet and passives. The slot input contributes local numerical bases only. Existing complete actor
APIs delegate with empty slots; raw helpers do not establish complete build admission.

Receiving defences accumulate Helmet, Gloves, Boots and Body Armour in that source order,
followed by the global term, then apply final rounding and clamp. The prepared actor stores fixed numerical
results, without retaining XML or borrowed item references. Repeated skill calculations
reuse these outputs with task-local scratch through Rayon.

## CLI and objectives

```powershell
cargo run --no-default-features --locked -- evaluate tests/fixtures/builds/mace-local-armour.xml --backend native --metric player.armour --metric player.evasion --raw
cargo run --release --no-default-features --locked -- search-build --problem examples/local-armour-search.json --jobs 4 --max-evaluations 1000
```

Graph problem **9** produces report **10**, scope `local_armour_native_search_v1`. Graph
problems 7/8 reject local armour in the source template or supplied alternatives, even if
an alternative is initially unselected. Earlier receiving scope gates remain in effect.
Locks, user-selected objectives, supplied item instances, connected passive choices,
attribute choices, class/ascendancy choices and supported skill loadouts use the existing
lazy search model and counted fresh finalist verification.

Both backends expose player `armour` and `evasion`, definition schema **1**, with unit
`rating_points`. These are final ratings. Physical mitigation and chance to evade depend
on the encounter and require their own metrics. Selected-minion queries are unsupported
for these definitions. The example maximizes selected hit DPS subject to resistance,
Energy-Shield, armour and evasion floors; these are configurable examples.

Current native profile IDs are `poe2-spark-action-timing-v7` and
`poe2-mace-strike-action-timing-v11`; profile media versions are **6** and **8**. Separate
`local_armour` evidence schema 2
records each selected component's source hash, local values and surviving records.
`receiving_defence` evidence remains schema 1. Fresh document realization reconstructs
both from selected source/data and rejects altered evidence. XML exports preserve source
formatting and carry the existing exact-data companion.

## Validation and remaining coverage

Independent checks execute original parser, item-local and receiving Lua code in cold and
warmed modes, then compare full fresh PoB builds and native export reimports. Tests cover
signed and fractional bases, quality, pair ordering, conditions, local/global partition,
requirements, data injection, owner/slot binding, source tampering and one/many-worker
search equality. Release measurements distinguish fresh source admission from reused
actor calculation. See the [living checkpoint](implementation.md) for current results.

The broader equipment, support, supporting-skill, reservation and minion pipelines remain
unfinished. Neither these fixtures nor portable compilation establish full PoB parity.
