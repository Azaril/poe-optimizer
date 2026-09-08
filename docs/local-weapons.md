# Local weapon modifiers

The controlled Mace profile now admits supplied normal or rare items on the two reviewed
one-hand Mace bases, with five understood local modifier families. The native evaluator
prepares each weapon once and reuses its local stats across tree/support candidates.
This extends equipment calculations within the existing diagnostic profile; general items,
minions, supporting actors and full native parity remain unfinished.

## Run the example

The [local-weapon problem](../examples/mace-local-weapon-search.json) uses problem schema
**5**, producing report schema **6**. It combines supplied weapons with the existing tree
selections and [seven support loadouts](support-loadouts.md). Its objective maximizes selected
hit DPS subject to a chaos-resistance floor. The item requiring level 80 demonstrates an
independent equip-level rejection against the level-60 template.

```powershell
cargo run --no-default-features --locked -- search-experimental --problem examples/mace-local-weapon-search.json --jobs 4 --max-proposals 8192 --max-evaluations 4412 --timeout-seconds 300
```

The example's authored names and rolls are synthetic calculation cases, including a very
large critical-chance roll. They do not claim legal affix tiers, roll combinations, rarity
affix limits, availability or acquisition cost. The 4,412 attempt ceiling accommodates the
entire supplied structural product plus template and finalist if every candidate were legal;
actual requirements can reject candidates before calculation.

Schema 5 retains `support_loadouts`, explicit `tree_search` point budgets and the existing
weapon, support-loadout, class, ascendancy and passive locks. `locks.weapon_id` fixes an exact
supplied payload. Schemas 1–4 retain their former normal, modifier-free, five-line weapon
scope; the CLI rejects extended item syntax in either their template or alternatives before
starting evaluation. The generalized library keeps `NormalMaceAlternative` as an alias of
`MaceWeaponAlternative`, preserving existing constructor calls.

Native search uses its prepared typed path by default. Add `--native-evaluation document`
to compare complete document evaluation under the same problem and budget. The template
and reserved finalist use full document evaluation in both modes. With the default `pob`
feature enabled, `--backend pob` selects the optional reference path and rejects the
native-only mode flag. See [search accounting](experimental-search.md#strategies-and-accounting).

## Item grammar and source preservation

The shared [item parser](../crates/poe-optimizer-import/src/mace_item.rs) accepts this ordered
header followed by zero or more fully recognized local lines:

```text
Rarity: RARE
Study Hammer
Wooden Club
Item Level: 10
Quality: 20
LevelReq: 1
Implicits: 0
Adds 2 to 5 Physical Damage
Adds 3 to 7 Fire Damage
50% increased Physical Damage
20% increased Attack Speed
100% increased Critical Hit Chance
```

`Rarity: NORMAL` omits the rare-name line. A rare name is a bounded ASCII label containing
letters, digits, spaces, apostrophes or hyphens. `LevelReq` is optional; the other metadata
fields and `Implicits: 0` are required. Reviewed bases are Wooden Club and Smithing Hammer;
selected custom data can rename those two supported base records.

The parser admits item level 1–100, quality 0–20 and explicit equip level 0–100. Header
numbers require canonical unsigned integer syntax (no leading zeroes except `0`). A payload is bounded to 8 KiB, lines to 256 bytes
and explicit modifier lines to 64. The latter is an implementation bound, not an affix-limit
check. Ordered duplicate modifier lines remain distinct and contribute independently.

The reviewed numeric grammar uses unsigned integer rolls from 0 through 1,000,000. Flat
minimum cannot exceed maximum. Signed numbers, exponent notation, decimal rolls, roll
ranges and trailing unconsumed text reject under the reviewed rules. Each modifier line
must match exactly one configured template in full. Local physical/fire additions, local
physical increase, local attack-speed increase and local critical-chance increase are the
only admitted families. Global or conditional wording, damage "to Attacks", other damage
families, sockets, runes, enchants, alternate quality, granted skills, corruption metadata,
uniques and unrecognized lines remain unsupported.

Line-edge ASCII whitespace and blank lines do not change interpretation. The parser retains
the exact source text, its SHA-256 and each modifier's literal line, one-based line number,
byte range, rule identity and numeric values. Inner template spelling and whitespace remain
significant. Item identities therefore distinguish different authored byte sequences even
when they calculate equally.

`parse_mace_item(input, package)` handles a supplied item string. Native profile parsing and
catalog template admission share `parse_mace_item_element(item, package)`, which reads the
original XML range before line-ending normalization. It retains literal CRLF, supports
named XML entities and whole-element CDATA, and rejects split text or numeric character
references incompatible with PoB. Materialization inserts the supplied text without adding
a newline, escaping XML syntax as needed. All unrelated template ranges, including notes,
comments and configuration, remain unchanged.

## Injected rules and equipment requirements

[Game-data schema 5](native-data.md#schema-migration) adds the `item_modifier_rules` section
and `character.critical_chance_cap`. Each item rule carries a stable ID, exact template,
ordered numeric capture kinds and typed mappings containing the target stat, operation,
capture index, exact flags and keyword flags. The [rule model](../crates/poe-optimizer-data/src/item_rules.rs)
rejects unsupported mappings, incomplete families, duplicate identities/templates and
ambiguous capture boundaries. Rust implements bounded matching and operation semantics;
concrete rolls come from the supplied payload, while mappings and base values come from
the selected package.

The [extraction policy](../crates/poe-optimizer-pob/src/game_data_policy.json) selects the
five source forms. Extraction derives their capture grammar and modifier mappings from
[the pinned ModParser](../vendor/path-of-building-poe2/src/Modules/ModParser.lua), including
exact local scope. Custom packages can explicitly change rule IDs, literal wording or use
the supported unsigned-decimal capture kind. Such data has its own identity and remains
unreviewed; changing grammar does not establish PoB parity or permit a reviewed-data fallback.

An item's effective equip level is its explicit `LevelReq`, when present, otherwise the
selected base requirement. This follows the non-unique item path in
[Item.lua](../vendor/path-of-building-poe2/src/Classes/Item.lua); an explicit value replaces
the base level rather than adding to it. Item level never supplies an equip-level limit.
The candidate domain then takes the maximum of that effective level and individual
active/support gem requirements. Base attributes and support-color aggregates retain the
existing [requirement rules](controlled-mutations.md#requirement-validation).

Source admission and numeric evaluation remain available for diagnostic builds that fail
requirements. Search admission and `validated_native_candidate` reject those failures before
a calculation handle is issued. Item understanding does not certify affix or general build
legality; item diagnostic evidence explicitly retains `affix_legality_verified: false`.

## Prepared calculations and verification

`ValidatedMaceWeapon` exposes immutable metadata, ordered `ItemModifierRoll` values and
source diagnostics. `CompiledGameData::prepare_mace_weapon(weapon, quality, item_level, rolls)`
resolves those rolls against its selected rules and returns `PreparedWeaponStats`. The
[weapon assembly module](../crates/poe-optimizer-engine/src/weapon.rs) validates the concrete
values again and rejects any unconsumed modifier. Prepared values belong to their compiled
dataset; the native adapter binds them to the catalog's exact private candidate handles.

Local consumption follows the literal source predicate: exact flags, zero keyword flags,
and either no first tag or a first `InSlot` tag. This differs from a general subset query.
The low-level helper retains leftovers for inspection; the public item grammar does not
admit tagged or conditional lines. The assembly sequence preserves source rounding:

- Local attack speed produces an attack rate rounded to two decimals before global/support speed.
- Physical endpoints combine base and flat damage, apply physical increase, then a separate
  quality multiplier before integer rounding. Fire does not receive weapon quality.
- A damage pair is emitted only when both rounded endpoints are positive.
- Local critical chance rounds to two decimals; actor calculations apply the injected cap
  before the attack's second accuracy roll.

[Mace candidate preparation](native-candidate-evaluation.md) stores one prepared value per
weapon axis and combines it with immutable tree and support axes. It does not store results
for the Cartesian product or retain candidate XML. Successful prepared numeric snapshots
continue to avoid heap allocation. Item preparation, owned metric conversion, objective
assessment, search bookkeeping, diagnostics and reporting still allocate.

The native Mace profile attachment uses
`application/vnd.poe-optimizer.native-profile+json;version=3`. Its `weapon_item` evidence
records exact source identity, rarity/name, metadata, equip-level selection, rule IDs and
roll provenance. `weapon_stats` exposes prepared local stats and consumed-modifier counts;
existing base/quality/item-level fields remain available. Full native realization checks
the parsed item evidence and exact materialized XML against the selected candidate. The
fresh finalist remains one budgeted full document calculation, followed by assessment
consistency checks; exporting it adds no calculation.

PoB's full reference export regenerates item metadata and includes `LevelReq`, including a
derived default when none was authored. For each fixed explicit modifier line it also emits
one neutral `ModRange`. Reference realization requires the exact ordered IDs, matching
count and `range="0.5"`; changed ranges, extra/missing entries, altered names, rolls or equip
levels reject. Those specific derived exports are accepted only at the reference realization
boundary. Source native item admission still rejects `ModRange` input children.

The [parser tests](../crates/poe-optimizer-import/src/mace_item_tests.rs) and
[catalog tests](../crates/poe-optimizer-import/src/controlled_mace_tests.rs) exercise source
fidelity, duplicates/removal, unknown scopes, bounds, custom grammar, requirements and
private handles. Source extraction and cold/warm parser comparisons live in
[PoB data tests](../crates/poe-optimizer-pob/tests/game_data.rs). These bounded checks do not
establish general item coverage or full-build parity outside the admitted profiles.
