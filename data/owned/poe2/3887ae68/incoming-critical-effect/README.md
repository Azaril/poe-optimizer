# Incoming critical effect

**Status: unpublished authoring draft, not a usable evaluator component.** The
existing shared Actor contract currently rejects Enemy Stat and Flag-query reads.
The authored formula and its ordinary native tests intentionally retain that
binding and fail at this authority gate. Adoption of the bounded read authority
is awaiting review in
[the proposal](../../../../../docs/owned-actor-enemy-reads-proposal.md). Source
measurements do not authorize publication or establish native execution coverage.

This packet adds the incoming critical chance and expected critical damage factor
from pinned `CalcDefence.lua:2268–2274`. The final outputs belong to the receiving
Actor, initially the existing shared Player Actor `332a`. They depend on that
Actor's evasion, incoming critical chance modifiers, unlucky-critical flag and
critical damage reduction. They are not global Enemy outputs.

`extension.json` uses the existing typed rule expressions. It adds no interpreter,
Lua runtime dependency, calculation API or build-specific branch. `policy.json`
contains two raw numeric configuration rows to append to the current import
policy. Presence and value are separate: explicit zero is a supplied value, a
false presence selects the default, and missing presence or a demanded missing
value stays unresolved. No source placeholder is imported as a raw input.

The raw defaults are 5 percentage points critical chance and 30 percentage points
extra critical damage. These are authored for the exact existing Encounter
`31d1` (Pinnacle, no selected boss skill). They do not claim to cover boss-skill
overrides or arbitrary encounter defaults. Enemy configuration becomes named
resolved Stats so future supported configuration producers can replace this
bounded derivation without embedding source-language override behavior in the
calculation. The source's lowercase `enemyCritChance` override hook has no known
writer in the pinned supported producer domain; source evidence must establish
that absence before this packet is published. No false presence or zero override
stand-in is manufactured.

`queries.json` declares three unordered Flag/Any inventories: Enemy NeverCrit,
Enemy AlwaysCrit and the receiving Actor's EnemyUnluckyCrit. All three remain
Partial. A true known flag never proves complete source coverage. The calculation
has six further named resolved-stat dependencies:

| Stat | Meaning | Recipient |
| --- | --- | --- |
| `335c` | Increased incoming critical chance | Actor |
| `335d` | Increased critical chance | Enemy |
| `335e` | Configured evasion chance after its own upstream adjustments | Actor |
| `335f` | Additional critical extra damage bonus | Enemy |
| `3360` | Increased critical extra damage bonus, Hit-filtered | Enemy |
| `3361` | Resolved critical extra damage reduction | Actor |

`3361` is **not** the raw BASE modifier total: its future producer must implement
`min(BASE ReduceCritExtraDamage, 100)` from source line 1995. Negative reduction
is allowed. This component consumes the resolved result and does not clamp it a
second time. Numeric Enemy contribution membership and ordering remain unproved;
these dependencies use no unchecked reductions and have no empty identities.

NeverCrit takes precedence over AlwaysCrit. Those branches skip ordinary chance
inputs, increases and evasion. The normal branch preserves the source's arithmetic
association and clamps chance to 0–100 before unlucky probability is applied.
The critical extra damage bonus is clamped only after BASE and INC arithmetic,
with a minimum of zero and no maximum. The expected extra damage calculation then
applies the resolved reduction. All component effects are gated off for Damage
Over Time, without demanding hit inputs or returning a fabricated critical result.

Outputs `3362` (chance), `3363` (critical extra damage bonus diagnostic) and `3364`
(expected critical factor) are percentage points, percentage points and a
dimensionless factor respectively. The bonus diagnostic exposes a source local
for component validation; it is not a PoB output field. The factor is not total
incoming damage or EHP. Conversion, gain, damage multipliers and Player mitigation
still need their own validated producer chains.

Publication preserves unrelated Partial Actor/Encounter programs and Encounter
input membership. Ordinary Rust tests execute the authored programs unchanged in
an explicitly finite world, inject resolved operands only in that fixture, and
check missing-input refusal, flag coverage, lazy branches, fresh/reused scratch
and Rayon results. Closing that test world does not close a real build.

The ordinary native replay test consumes measured `source-vectors.json` operands
and expected outputs rather than implementing the source formula in test Rust.
Unobserved numeric query results remain missing. When NeverCrit prevents the
source from querying AlwaysCrit, the native test evaluates both Boolean
completions and requires the measured result for each; neither completion is
misrepresented as a source observation. DOT vectors must have absent critical
outputs and evaluate to inactive native effects.
