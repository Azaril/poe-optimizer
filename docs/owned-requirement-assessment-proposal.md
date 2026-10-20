# Proposal: requirement assessment beside native measurements

**Status:** Accepted and implemented locally October 19, 2026: assessment alongside metrics. Ordinary/support regression tests and strict Engine/CLI lint pass. Complete candidate admission and mechanical Action census remain unfinished.

## Motivation and the concrete blocker

Typed rules already emit Requirement effects. Native effects execution retains
each exact Boolean result, including false and unavailable. Before this change,
metric projection collected measurements and dropped those effects. The CLI
continues to report broader game legality as `not_checked`: assessing injected
requirements is not a claim that every gameplay rule is implemented.

The accepted progression model exposes the dependency. Two distinct Rewards
can each assert the same campaign count. Numeric agreement accepts their value,
while a separately authored exact authority count correctly yields a false
Requirement. That detects the duplicate authority, but no owned final admission
consumer currently prevents its use as a feasible search candidate. Retaining
computable measurements for an invalid build is intentional.

## Recommended contract

Add an explicit **requirement assessment alongside native metric reports**, from
the same execution attempt. Reuse the existing graph, exact Requirement targets
and worker scratch. Do not add another rule interpreter, validation graph or
progression-specific check.

- A known true Requirement is satisfied; a known false one is violated.
- A proven inactive Requirement is inapplicable. Unknown activation or an
  unavailable result remains incomplete, never false or inactive by default.
- Preserve exact occurrence/code and source/recipient diagnostics. Report both
  known violations and incomplete coverage when both occur; do not collapse one
  into the other or suppress independently computed measurements.
- Assess every applicable selected Requirement independently of the metric list.
  Empty or unrelated metric queries must not omit a requirement. Complete
  assessment requires complete relevant owner/program inventories; an empty
  observed effect list is not a completeness certificate.
- This assesses the injected requirements only. Other unsupported mechanics,
  inputs and metrics remain separate barriers to complete-build admission.
- Include ordinary graph effects and support-stage overridden/appended effects.
  Collect before the support projection callback clears scratch, so the result
  describes the same attempt as its measurements.
- Keep deterministic ordering and bounded collection. Fresh/reused/parallel runs
  and changed metric lists must preserve the same assessment for the same build,
  Scenario and rule identity.

The future owned search adapter must require complete, satisfied assessment as
well as existing coverage/metric gates before admitting a candidate. It must
repeat this during finalist verification. User objective constraints remain
separate from injected gameplay requirements. A violation is not a PlanError,
numeric unsupported-domain result, or automatically missing measurement.

## Alternative and trade-off

| Option | Benefit | Cost |
| --- | --- | --- |
| Assessment beside each metric report (recommended) | Measurements and feasibility evidence describe one attempt; callers cannot accidentally validate a different candidate or omit support effects. | Adds a public report field and requires consumers to handle its explicit coverage/status. |
| Separate explicit assessment API | Callers can request measurements or assessment independently. | Needs an exact attempt/plan binding and coordination in every finalization/search caller; assessing twice may duplicate work. |

The existing diagnostic effects API may remain useful. Neither option should
retroactively claim complete game legality for the current partial packages.

## Implementation and validation

The smallest internal seam is a bounded collector over
`owned_plan/graph.rs::ExecutionGraphView` and current scratch. Ordinary metric
finalization and `support_metrics`' final projection can share it. The existing
support suffix admits Requirement effects but does not place them in the metric
value map, so scanning only metric values is insufficient.

Prove true, false, unavailable and guarded-inactive results; equal and conflicting
duplicate progression authorities; ordinary and support-added/overridden effects;
empty/unrelated metric queries; incomplete inventories; exact origins; and
fresh/reused/Rayon equality. Preserve current finite measurements and explicit
`not_checked` behavior until the new assessment is actually supplied. Rebuild
affected semantic identities and all-five fixtures if their contracts change.

The collector and CLI are implemented locally with passing regression tests. Ordinary
and support metric reports contain `requirements`, with a separate coverage
result and exact occurrence outcomes: `satisfied`, `violated`, `inapplicable`, or
`unavailable`. Complete coverage can coexist with known violations; incomplete
coverage can retain known violations. `is_satisfied()` requires complete coverage
and no violated or unavailable result. The CLI labels the scope
`injected_requirements`; `game_legality` remains `not_checked`.

The collector reads the ordinary graph or final support graph before worker
scratch is cleared. Failed support preparation retains requirements already
evaluated in its prefix and marks bound, unexecuted requirements unavailable.
Its incomplete coverage also accounts for delivery contexts that preparation
could not establish; it does not invent exact occurrences for those contexts. Metric
plan identities use the current `owned-metric-plan-v3` digest domain to invalidate
cached reports with the old semantics. No compatibility reader or second
validation graph is introduced. The future owned search adapter remains separate
unfinished work; this report is its required input, not its implementation.

### Staged Action coverage

Current preparation creates some Action contexts only because they appear in a
metric query. Collecting all bound effects therefore cannot prove that every
mechanically relevant Action Requirement has been considered. This checkpoint
must conservatively report incomplete coverage for potential Action Requirement
families without a complete, metric-independent context census, even if a query
instantiates some observed contexts. Such effects may remain useful diagnostics;
they must never upgrade coverage merely by being queried. Do not enumerate all
possible Actions or introduce another Action registry as part of this collector.
Finish the mechanical context census before claiming complete query-independent
Action assessment. Reward/Actor scopes can be validated independently where their
existing context discovery and inventories are complete. Missing or Partial
selected ActionOutput and ActorSlot rule owners also produce exact coverage gaps
before query-driven Action creation, even if no Requirement effects are known.
While this limitation remains, observed Action effect rows can vary with the
requested Actions; the assessment stays incomplete in every such query scope.

### Selected-owner correction, October 20

The initial assessment scanned every packaged Action Requirement family.
Consequently Twister's `intrinsic-action-coefficients` could keep Sniper's
assessment incomplete despite Twister being absent. The private compiler now
records potential owners during its existing structural traversal, independently
of metric queries and before inactive-provider filtering. It includes selected
provider owners, potential Skills, exact output slots and the explicit outputs
of supplying Skill grants. Selected support assignments retain their receiving
programs even when no Action is requested.

Missing and Partial owners remain gaps. Potential Skills with no established
structural supply remain unresolved; this reuses the existing Scenario-intent
topology census without introducing another graph or creating Actions.
Ordinary numerical gaps and collected Requirement outcomes are not filtered.
Unrelated packaged owners can no longer create a requirement-family gap, but
selected Action families still require a complete mechanical context census.
Focused ordinary/support, deferred-discovery and CLI regressions pass, including
fresh/reused/Rayon results. The existing owner limit independently bounds
deferred providers and potential requirement subjects; output owners count in
the latter even without invocations. Exact-bound and duplicate tests preserve
those limits. The implementation plan records the complete validation and
resume point; hosted CI remains pending publication.

The selected `sniper-ordinary-flat-reservation` program therefore remains an
actual admission dependency. Twister's requirement must likewise remain blocking
when its source is selected or potentially supplied.
The accepted Skill-owned resource-obligation migration separately replaces the
Action-owned reservation responsibility; it must preserve its requirements.
Neither narrowing the metric list nor deleting applicable validation is a way to
complete this assessment.
