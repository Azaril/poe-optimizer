# Summon count and Spirit reservation

Status: native component, source replay and five-original publication validation passed. The checked baseline
and completed validation are recorded in [the implementation plan](implementation.md).

The native model stores an occurrence's count as a typed usage preference on its
skill preset. Request composition applies a scenario override to that exact
target. Count does not change the reusable Skill definition, create one actor
record per summon, or multiply every damage metric. Reservation and combined
damage reporting are separate consumers.

## Source import and authority

The additive `PobPhysicalPrimarySkillV2` policy carries the existing Boolean
rows unchanged and adds explicitly authored numeric rows. It uses the shared
numeric codecs, schema ranges, occurrence identities and source-evidence bounds.
No Lua runs during owned normalization or native evaluation. Historical V1
serialization and its physical-inventory proof remain unchanged.

A numeric row declares both the occurrence and containing-group source frames.
An optional group override wins by **presence**: a present zero wins over the
occurrence value. A malformed, duplicated or out-of-range override remains
Pending; it does not fall back. Absent occurrence values remain Pending rather
than receiving PoB's implicit count of one. Both source locations are retained
in provenance, and a row cannot move its usage to another saved preset.

PoB's reservation helper can select the first matching granted effect in a
group instead of the exact physical gem that supplied the active skill. This
is an import/reference concern. The native evaluator keeps occurrence-specific
semantics. Numeric authoring can request raw occurrence transport or require a
reviewed, unambiguous source fallback. The latter admits the target once and
only explicitly reviewed companion identities whose primary and additional
effects cannot match it. Unknown companions and ambiguous duplicates retain a
Pending value. An explicit group override bypasses that fallback search.

The companion evidence is acquired offline against the bound catalog, including
unresolved declared additions. It is injected into Import as finite identities;
the runtime does not inspect PoB catalog internals. Transporting these values
does not prove complete physical parameters, skill usage, command membership,
minion selection or action selection.

The [skeletal count packet](../data/owned/poe2/3887ae68/skeletal-counts/README.md)
extends this same projection to Arsonist, Frost Mage and Reaver. It adds fifteen
occurrence preferences across the original and archived presets, using the
existing Skill-targeted policy and reviewed integer domain 0..4. Each family's
companion-negative authority is checked independently. This transports requested
counts only: Sniper's intrinsic reservation table is not authority for another
family, and their final-level and reservation producers remain incomplete.

## Native arithmetic

The existing Sniper Action rule reads its prepared level and the injected
`sniper.spirit-reservation` table to produce the intrinsic flat coefficient.
The new consumer reuses that result. It does not introduce another level table
or a second reservation engine.

Reservation multipliers are independent of ordinary resource-cost multipliers.
Pinned `CalcActiveSkill.lua:735-739` emits `SupportManaMultiplier` from
`manaMultiplier` and `ReservationMultiplier` from `reservationMultiplier`.
`CalcOffence.lua:2215-2217` consumes the former; `CalcDefence.lua:201-202`
consumes the latter. The published Magnified Action channel `32fa` therefore
cannot supply reservation input `326e`. The witnessed 1.3 reservation control
below uses Hulking Minions, not Magnified. Equal numeric values or units do not
establish shared meaning or contributor coverage.

For the ordinary flat Spirit branch, the intended operation order is:

1. Add explicit extra Spirit to the intrinsic flat coefficient.
2. Apply the reservation multiplier after the source's four-place floor helper.
3. Apply increased and more reservation, then increased and more efficiency.
4. Round the per-use result and clamp it to zero.
5. Multiply by `max(selected count - free count, 0)`.

The order matters: rounding a total after multiplication is different from
rounding each paid summon first. The source helper's epsilon and its
`floor(value + 0.5)` behavior are represented by ordinary typed arithmetic
operations. They are not replaced by a differently defined generic rounding
mode. Game constants, ranges, units and programs stay in owned data.

The consumer requires explicit branch inputs. Forced costs, reservation that
becomes a cost, mana-cost conversion, base overrides, mines and Spirit-to-Life
conversion are not silently treated as the ordinary branch. Sniper's finite
definition frame also excludes companion, spectre and Blasphemy branches.
The intrinsic table applies only where preparation has not replaced its base
coefficient. PoB's prepared level row is a copy that may include stat-set
overrides or `noReservation`; a matching table index alone is insufficient proof.

Final level, final quality, modifier aggregates, free count and branch facts
need real producers with complete contributor coverage. Until those producers
are integrated, the production package remains Partial. Finite component tests
may explicitly inject observed inputs; production callers do not receive
neutral defaults. Observed costs such as 39 and 50 are test evidence, not game
definitions.

The arithmetic amount zero and PoB's absent per-skill `SpiritReservedBase` field
are distinct observations. Actor reservation totals, capped totals, unreserved
Spirit and combined damage require their own aggregation and availability
semantics. Passing this component does not establish whole-build parity.

## Validation and next work

The optional source witness records complete PoB loads, original helper-call
intermediates, prepared rows, modifier provenance and physical occurrence joins
in both JIT modes. Fresh and normally rebuilt stages are preserved separately as
diagnostics. The later owner decision treats Frost's initialization discrepancy
as a narrow upstream exception and requires deterministic replay elsewhere;
no warm/retry canonical lifecycle is adopted by this component. See the
[determinism contract](execution-and-interfaces.md#reproducibility-and-throughput).

The complete-source witness passed 21 cases in each JIT mode: 42 independent
loads and 126 stage snapshots. The two 10,530,864-byte reports are identical,
with SHA-256 `c28c08a4f40bc17a930d678fc53da6656fd811befca02230119e31ec3097d98f`.
They are reproduced by the ignored `owned_minion_reservation_source` test and
stored in `runs/owned-minion-reservation-source-01/`. The source revision,
manifest, catalog and 21 relevant source files are bound by the packet's
`authoring.json`. Lua debug hooks observe original calls without replacing
business functions. Mixed Lua tables preserve both named fields and numeric
entries, including tagged modifier provenance.

The unchanged selected Sniper has final level 22, intrinsic flat cost 29 and
reservation efficiency -25, producing 39 Spirit per paid summon. Three copies
reserve 117. A witnessed additional Hulking support changes the multiplier to
1.3 and the cost to 50. Separate controls cover reservation reduction, more
reservation, efficiency, their composition, duplicate effect matching and the
Warrior's free-count mechanic. Warrior observations do not validate Sniper's
level table or establish native Warrior coverage.

Eight native tests pass, exercising the actual authored programs, missing inputs,
unsupported branches, independent occurrences, scenario replacement, scratch
reuse, parallel worker isolation and authenticated source replay in all three
stages. Two CLI tests pass, reproducing package bytes and preserving all five
original sources, saved selections, 110 queries and unrelated obligations. Ten
mutation controls preserve archived presets and physical inputs. The implementation
plan records the current receipt and remaining build blockers.

Next, connect real preparation and modifier producers, resolve Sniper's remaining
minion/action/command input dispositions, and complete the first unchanged native
build. Extend count ranges and mechanic branches through new source evidence
and injected data. Do not use this finite component to claim support for every
summon or reservation mechanic.
