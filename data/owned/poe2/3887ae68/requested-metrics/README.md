# Requested metric identities

This data defines the twenty semantic measurements already requested by the five
original builds. It adds exact external catalog-key mappings so those requests can
name owned Metric definitions. It adds no numerical producer or metric-to-stat
binding. Build completion and calculated metric availability remain separate.

The predecessor is the checked ordinary-item-input release with input digest
`6e7f65c0424e13405da05668cf2ad7be7b27da6fe5455a25f38e68494193d6db`.
`authoring.json` binds its schema, registry, item policies, normalization and mapping.
It records the hashes of all five query files, which must remain byte-identical.

## Artifacts

- `extension.json` is an `OwnedRecipeExtension`: one new damage-per-second Unit
  and twenty Known Metric schemas; no changes to existing declarations, programs,
  tables, receivers or operation versions.
- `mappings.json` contains twenty `MappingEntry` records. Each key is exactly
  `Catalog(Metric, Text(name), Missing version, Missing variant)`. These are exact
  semantic identities, with no wildcard, fixture-name or request-index matching.
- `bindings.json` is an authoring ledger of meaning, unit, allowed target roles
  and reference observations. Its reference raw fields are documentation for parity;
  they are not evaluator inputs or runtime metric bindings.
- `authoring.json` records the predecessor and the established semantic catalog in
  `crates/poe-optimizer-pob/src/metrics.rs`. Its catalog hash uses UTF-8 with LF line
  endings, allowing the same evidence on Windows and Linux checkouts.

The registry appends keys `312c` through `3140`, preserving all 12,587 prior
entries. Keys below have prefix `def.000000000000` and the existing namespace
`poe2/owned-mechanics-v1`.

| Metric | Definition | Unit |
| --- | --- | --- |
| life |312d|3119 Life points|
| mana |312e|0003 Mana points|
| energy_shield |312f|29ed Energy Shield points|
| fire_resistance_capped_pct |3130|0002 percentage points|
| cold_resistance_capped_pct |3131|0002 percentage points|
| lightning_resistance_capped_pct |3132|0002 percentage points|
| chaos_resistance_capped_pct |3133|0002 percentage points|
| pob_total_ehp |3134|1d3a damage|
| physical_max_hit |3135|1d3a damage|
| fire_max_hit |3136|1d3a damage|
| cold_max_hit |3137|1d3a damage|
| lightning_max_hit |3138|1d3a damage|
| chaos_max_hit |3139|1d3a damage|
| selected_hit_dps |313a|312c damage per second|
| selected_average_hit |313b|1d3a damage|
| spirit |313c|0004 Spirit points|
| armour |313d|29ee rating points|
| evasion |313e|29ee rating points|
| movement_speed_pct |313f|0002 percentage points|
| action_speed_pct |3140|0002 percentage points|

The new Unit312c has dimension Rate and the meaning damage per second. It is
distinct from attack-rate1d39 and recharge-rate29ef. Sharing a dimension does not
make those units interchangeable in an owned metric binding.

## Scope and availability

Eighteen definitions admit Actor targets with Player actor role and Character
provider role. The two selected-hit definitions also admit Action targets, Owned
actor role and SkillUse provider role. This preserves the four exact existing
Action queries: two for the original02 player Twister and two for the original05
generated Sniper action. Their provider paths, outputs and actor identities remain
in the supplied request. The eight unresolved selected-minion target rows in the
other query sets remain unresolved. There are 98 Player rows and four Action rows
across the 110 original requests.

The complete meanings are recorded per metric in `bindings.json`:

- Resource metrics mean maximum pools; Mana and Spirit are before reservation.
- Armour and evasion are final ratings. They are not mitigation or evade chance.
- Capped resistance uses percentage points: 75 means 75 percent.
- Movement and action speed use percentages of baseline: 100 means baseline.
  The reference observations scale source factors by 100; this data does not add
  that calculation to the native graph.
- Maximum-hit metrics depend on the actual defensive scenario. `pob_total_ehp`
  retains its explicitly PoB-qualified aggregate meaning and encounter, avoidance
  and recovery assumptions. Nonfinite immunity results must not become finite zero.
- Selected hit DPS excludes separate damage-over-time and Full DPS rollups.
  Average-damage mode still requires an explicit usage model before DPS is
  available. Average hit includes critical-strike weighting. An absent or
  non-damaging selected action does not produce either measurement.

An Actor(Player) selected-hit request retains its established selected-action
observation meaning. Defining its identity does not invent a native default action
or permit a cached source result to stand in for calculation.

## Numerical work remains open

No artifact here is a `MetricMappingInput`. In particular, flat Life311a,
resistance09d4/09d5, movement30ff, raw base defences and local contribution channels
are not final metric producers. Their existing Partial coverage remains intact.

When an otherwise ready owned request has a known metric identity but no
calculation binding, the existing metric engine reports `MissingMetricBinding`
with no stat. A binding whose final producer is missing reports `MissingProducer`.
Target activation and whole-plan coverage gates continue to apply; no missing
measurement becomes zero. The real original builds remain incomplete until their
remaining input and numerical dependencies are implemented and validated.

Publication tests must preserve every original source input, saved selection,
query byte, existing registry entry and program. The intended normalization delta
is solely the 22 requested metric identities per original becoming Known, with
their exact issue/link and allocator consequences. This authoring directory alone
does not certify that publication or calculation has succeeded.
