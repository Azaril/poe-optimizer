# Body armour and movement speed

The native Spark and Mace pipelines extend fixed local armour preparation to Body Armour
and calculate movement alongside actor resources and receiving defences. Data is loaded
from the selected immutable package. Native execution remains Rust-only; the pinned PoB
checkout supplies optional independent development and update-parity tests.

## Data and admitted mechanics

Schema 10 adds fixed body bases and the `movement` section. Base requirements, quality,
ratings and movement penalty come from source extraction. `movement_penalty` preserves
absence separately from an explicit zero: the original Lua branch generates a modifier
for zero because zero is truthy. The source-aware item path emits the selected data's
negative MovementSpeed BASE record with its negated IgnoreMovementPenalties condition,
source identity and source ordering. Generated records remain distinct from parsed item
text in evidence, then join the final global contribution exactly once.

The shared actor IR supports MovementSpeed BASE, INC, MORE and OVERRIDE, the ignore-armour-
penalties flag and the cannot-be-below-base flag. Reviewed attribute conditions can control
ordinary numeric movement records. Special override/ignore/minimum phrases admit only
their plain source wording; appended condition text is rejected. The numerical IR also
represents attribute-conditioned flags for source-function parity and future producers. Dynamic IgnoreMovementPenalties conditions are limited to movement numeric
records; recursive flag dependencies and feedback into earlier actor stages are rejected.
Whole-source capability checks determine passive admission; unrepresented modifiers are
never discarded to admit an item or node.

Exact source wording and capture rules are injected. Override text uses a division capture
rather than a reciprocal multiplication, retaining the original floating-point operation.
Item formatting runs before parsing; its keys are case-sensitive. Modifier wording follows
the source ASCII case normalization while preserving original source bytes. Configuration
captures retain their raw values. Differently capitalized item text can therefore use a
different formatting key before producing the same modifier kind.

The current complete build pipelines require neutral ActionSpeed and reject action-speed,
party-linked movement, moving-while-using-skills, reservation, minion and other unrepresented
effects. A non-neutral injected action-speed default is rejected because it also requires
complete offence consumers. The raw movement primitive accepts an explicit action-speed
input; that helper alone does not establish complete build admission.

## Shared calculation and ordering

`prepare_armour_with_source` binds local values and generated/global contributions to the
selected compiled dataset and source item. `ArmourSlots` borrows four optional immutable
components. Equipment globals follow Weapon 1, Helmet, Body Armour, Gloves, Boots and Amulet;
receiving numerical inputs follow Helmet, Gloves, Boots and Body Armour. These different
orders are intentional original-source behavior.

After attribute conditions resolve, actor preparation resolves the ignore-penalty flag and
calculates movement. An OVERRIDE, including zero, replaces the ordinary calculation and
skips its first rounding step. Otherwise the calculation follows
`(base_multiplier + BASE) * ((1 + INC / 100) * MORE)`, rounded at the injected precision.
The optional minimum-speed flag applies afterward, including to overrides. The effective
result multiplies by the admitted action-speed modifier and rounds again. Negative values
are retained when the source does not apply the minimum.

Prepared actors retain fixed movement ratios and flags. Typed skill calculations reuse
those results with independent scratch per worker. Fresh candidate admission and repeated
calculation have separate costs; neither requires a Lua host or candidate-result cache.

## CLI and metric contract

```powershell
cargo run --no-default-features --locked -- evaluate tests/fixtures/builds/mace-body-armour.xml --backend native --metric player.movement_speed_pct --raw
cargo run --release --no-default-features --locked -- search-build --problem examples/body-armour-search.json --jobs 4 --max-evaluations 1000
```

Player `movement_speed_pct`, definition schema 1 and unit `percent`, is
`100 * EffectiveMovementSpeedMod`. **100 means baseline; 120 means 20% faster.** This is
neither percent increased movement speed nor absolute travel speed. Both backends expose
the same contract; selected-minion requests are unsupported for this definition.

Graph problem 10 produces report 11 with scope `movement_native_search_v1`. Older graph
schemas and legacy mutation inputs retain their authored mechanic boundaries. Body items,
authored movement records and generated movement penalties require the new graph scope,
even when present only in unselected supplied alternatives. Constraints, required item
subsets, locks, class/ascendancy choices and supported passive/skill choices remain
configurable. The example maximizes DPS under resistance, defence and movement floors.

Native profile IDs are `poe2-spark-body-movement-v6` and
`poe2-mace-strike-body-movement-v10`, with profile media versions 6 and 8. The thirteen-metric
snapshot appends movement after evasion. `local_armour` evidence schema 2 distinguishes
source and generated global records; `movement` schema 1 records three movement ratios and
three flags. Receiving evidence remains schema 1. Fresh realization validates all evidence
against exact selected data and source; exports preserve original bytes and the data companion.

## Validation and remaining work

See the [living implementation checkpoint](implementation.md) for current counts, source
identity, publication status and measured performance. Validation includes actual source
parser/item/movement functions, fresh full PoB builds, exact export reimports, changing
four-slot actor inputs, source/data binding, injected penalty ranking and parallel search.

Per-level ratings, Ward, block, alternate quality, general skills/supports and the supplied
minion build remain unfinished. These admitted pipelines do not establish full PoB parity.
