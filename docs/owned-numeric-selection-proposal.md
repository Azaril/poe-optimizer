# Checked optional numeric selection in the contribution graph

**Status:** Accepted by the owner on 2026-10-09; generic native contract implemented in operations V27. Real Mana adoption remains pending.
**Date:** 2026-10-08.
**Decider:** Project owner.

## Context

The next Player Mana consumer needs to distinguish an absent override from an
override whose value is zero. The original complete-build witness confirms that
the ordinary pool clamps to at least one, while a present zero override produces
zero. Current owned queries provide numeric Sum/Product and Boolean Any, with
explicit empty identities. None expresses an optional numeric selection.

Using separate Boolean-presence and numeric-Sum sources would lose the required
relationship between a candidate's activation, value and identity. Encoding
absence as zero would produce the wrong result. A Mana-specific Boolean for
"no Mana" represents the current pinned zero override but would not express
other numeric resource, defence or action overrides without another mechanism.

Pinned PoB `ModDB.lua:344` selects the first matching active override in its
local database, then searches parents. This is observed source implementation,
not established game precedence. The literal Mana override producers located
in the pinned parser all supply zero; this scan is not a general generated-
producer completeness proof. Do not import database insertion order or Lua
truthiness as native rules.

## Recommended decision

Extend the **existing checked contribution graph** with numeric selection that
retains one coherent result: absent, selected value, or unavailable. Expose
typed presence and value projections of that same checked result to ordinary
rule programs; do not introduce a second collector or an optional value type
through every authored build parameter.

Start with an explicit **require agreement** policy:

- A complete empty/inactive domain resolves absent, not numeric zero.
- One active known value resolves present, including zero and negative values
  where the channel's declared numeric domain permits them.
- Several active equal values resolve that value while retaining each exact
  source for coverage and diagnostics. They are not added or deduplicated.
- Conflicting values, an unknown potentially active source, or incomplete
  membership make the selection unavailable. No arbitrary winner is chosen.
- Presence and value share candidate discovery, activation, units, recipient
  authority, completeness and cached result identity. A value read when absent
  stays unavailable; a normal conditional can avoid demanding it.
- Preserve source/recipient multiplicity and the existing potential-writer
  checks, dependency stages and scratch isolation. Nonfinite values remain
  invalid. Specify signed-zero normalization once in the native contract.

This policy is sufficient for a guarded empty domain and agreeing zero-override
sources. It deliberately does not certify conflicting overrides. Add another
selection policy only when game evidence establishes it; if source precedence
is the only available evidence, retain that uncertainty explicitly and discuss
the boundary before adopting it. Future selection policies belong to this same
graph, with explicit semantic priority rather than implicit traversal order.

## Options and trade-offs

| Option | Benefit | Cost |
| --- | --- | --- |
| Coherent numeric selection in the existing graph (recommended) | Reusable presence/value contract, exact source accounting, no sentinel or arbitrary ordering | Core/Data/Engine contract work and generic tests before final Mana adoption |
| Dedicated zero-Mana Boolean | Small initial data change for known zero override behavior | Does not handle general numeric overrides; later consumers need a separate model |
| Separate presence and summed value channels | Uses existing operations | Requires additional coupling proofs and gives incorrect results for duplicate nonzero overrides unless restricted |

## Implementation and acceptance

1. Specify the typed contribution/query contract and both projections, including
   absent-value reads, disagreement, signed zero and unknown activation/value.
2. Implement through the current compiler/planner/worker path. Rebuild current
   development artifacts; no historical-format compatibility is required.
3. Exercise mixed active/inactive/unknown sources, repeated equal sources,
   conflicts, units, exact recipients and unlisted zero/inactive writers.
   Verify stages/cycles and fresh/reused/four-worker equivalence.
4. Adopt Mana's actual override domain with an offline source census and controls;
   retain global mechanics gaps. An empty observed build is not that census.
5. Finish conversion/extra/total inputs and the resource arithmetic proof before
   publishing final Mana. This proposal approves neither new precedence laws,
   complete resource coverage nor a rounding compatibility mode.

The exact-Action query proposal is a separate scope/authority extension. The
owner explicitly approved both decisions on 2026-10-09; neither grants unrelated
producer origins or an override precedence law beyond require agreement.

## Implemented contract

Operations V27 adds `Override`, unordered `RequireAgreement`, and explicit
`ContributionSelection` reads with `Present` or `Value` projections. Fold groups
still require an explicit identity; selection groups require `empty: null`.
This changes the current Rust authoring contract without a historical reader.
Integer selections retain exact integers; quantities require the stat's exact
unit. Agreement is exact numeric equality, with both signed zeros normalized
to positive zero. No tolerance or implicit precedence is introduced.

Cold planning authenticates the existing query membership and interns one
effect node per exact recipient/channel/query/group. That node evaluates its
candidates once per worker attempt. All programs and both projections read its
cached result by index. The result's diagnostic effect uses `Known` for present,
`Inactive` for proved absence, and the ordinary unavailable variants for failure.
Presence maps only proved absence to false; an absent value read reports
`AbsentSelection`. Conflicts report `ConflictingContributors`. Consumer read
identifiers can annotate the same underlying failure without changing it.
Every original contribution occurrence remains in the report.

Staged evaluation requires an explicit frozen contribution channel for this
shared result. Producers cannot write after its freeze, and consumers cannot
read before it. Existing readiness and potential-writer proofs still apply.
Retained support programs can add selection consumers to the execution suffix;
the suffix rebinds existing result candidates, and the frozen-prefix check rejects
changes to any selection already executed. No second collector or worker cache
is introduced. The V27 effect-plan identity domain is `owned-effect-plan-v24`.

Acceptance tests use generic integer/quantity sources, duplicate equipment
occurrences, exact Action selections, empty and inactive domains, unknown values
and activation, conflicts, partial/unread membership, invalid units, cycles,
stages, storage round trips, work bounds and reused/four-worker execution.
Real Mana source census and data publication remain separate work; this contract
does not establish final resource coverage or close any original build.
