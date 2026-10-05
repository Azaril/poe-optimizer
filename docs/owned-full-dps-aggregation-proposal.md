# Proposal: owned damage reporting and aggregation

**Status:** Proposed; no Core contract or native reporting consumer is changed.
The source-attribution witness **passes all 44 cases in both JIT modes**.
Existing count transport and raw-input publications do not establish aggregate
damage parity or complete usage inventories.
**Date:** 2026-10-05.

Use exact native occurrence and action identities to build a checked contribution
inventory, then evaluate explicitly declared reductions in the existing rule
graph. Keep aggregate reporting separate from skill activation, actor population
and selected-action DPS. The current requested `TotalDPS` metric explicitly
excludes Full DPS rollups; this proposal must not change its meaning.

## Evidence and remaining uncertainty

Pinned source revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` provides the
following reporting boundaries in `src/Modules/Calcs.lua`:

| Source boundary | Observed source-code behavior |
| --- | --- |
| `calcFullDPS`, line 251 | Enumerates included active skills and evaluates each in the original calculation environment. |
| Count lookup, lines 302–312 | Uses the existing count helper unless a captured pass is reused. The helper is not a general actor-population contract. |
| Harvesting, lines 329–371 | A skill pass can harvest a minion, player and mirage separately. It retains distinct count and damage-over-time scaling fields. |
| `mergeFullDPSPass`, lines 170–198 | Adds counted hit DPS, applies field-specific maximum or additive reductions, and records the display source of a strictly greater maximum. |
| Final rollup, lines 392–437 | Adds separate ailment/DoT rows, applies the DoT cap, then calculates the culling contribution. |

The existing generated-usage reports in
`runs/owned-generated-skill-usage-source-01/` authenticate retained count/group
settings and exact source effects. They deliberately leave reporting rows
unjoined. Count-three changes Sand and Firebolt reporting in those finite
controls; Firebolt's strongest Ignite row remains separate from its counted hit
row. Display labels in those reports are insufficient to attribute a contribution
to one of several uses of the same skill. Water's absence of a positive reporting
row is not evidence that its usage or actions are irrelevant.

The new optional Rust test
[`owned_full_dps_attribution_source.rs`](../crates/poe-optimizer-pob/tests/owned_full_dps_attribution_source.rs)
and its test-only Lua observer target this missing attribution. They authenticate
the original functions, capture loaded source objects, join harvested output
tables to their actual actors, and observe the original merger's writes and
maximum-branch entry. They do not replace formulas, source functions or upvalues.
Same-name rows are never identity keys. Aggregate rows retain their observed
aggregate field and contributing events; they must not fabricate a single source
for a sum or culling rollup.

The first full instrumented run also exposes a later presentation mutation:
`Build.lua:2393` sorts the same `SkillDPS` array in place by counted damage. The
zero-count Firebolt and simultaneous manual/generated Sand controls reorder MAIN
rows without changing their values. Reference attribution must preserve the
calculation index and prove the final display index through actual row-object
identity. Sorting labels or matching equal row values is insufficient when two
sources produce identical rows. This UI ordering is not a native damage rule.
The same Firebolt control keeps a positive strongest-Ignite row even when its
requested hit count is zero. Reporting count therefore cannot act as a universal
activation switch or multiply every damage channel; each reduction needs its
own declared semantics.

The checked run covers 44 cases in both JIT modes, each with a fresh load and two
explicit normal rebuilds. Every case also has an independent unobserved lifecycle
for same-phase output/selection comparison. It includes all five originals,
manual and generated Djinn, item Firebolt, count one/three, group zero/four,
inclusion, disabling, simultaneous same-family sources and a positive Ignite tie
between distinct same-label manual sources. The Frost source-initialization
bug remains a narrow known exception: no retry, warm-until-pass protocol or
fresh-versus-rebuilt equality requirement is introduced.

Evidence: `runs/owned-full-dps-attribution-source-03/source-jit-{off,on}.json`.
Both schema-2 reports are **16,334,100 bytes**, SHA256
`c25597169569bbc08d57d5043b296cfd67327ae9855e9638e55cb73296d95018`.
The original-function witness passes exact source/effect/actor and stat-set
joins, contribution/write/max-branch references, final row-object correspondence,
count/inclusion/disable controls and ties in each reporting phase. Parent-frame
observer deltas are explicitly excluded from actor-contribution lists. Results
match independent unobserved lifecycles and each other across JIT modes.
This proves the finite source observations, not native aggregation or unseen
contribution families. Culling stays a composite aggregate, not a fabricated
single-source winner; full native dependency semantics remain to be specified.

Failed `source-01` retains the observer's C-call hook defect; `source-02` retains
the initially unaccounted-for UI permutation. The saved-report validator can
recheck captured assertions with a 32 MiB bound and content hash, but explicitly
does not claim a fresh replay, cross-JIT agreement, or row-object proof absent
from historical reports. The full `source-03` run supplies those live checks.

Reproduce the full witness with the diagnostic environment variable unset:

```text
cargo test -p poe-optimizer-pob --test owned_full_dps_attribution_source complete_full_dps_attribution_observes_original_contributors --locked -- --ignored --nocapture --test-threads=1
```

`POE_FULL_DPS_ATTRIBUTION_CASE` selects a diagnostic subset only. The separate
`validate_saved_full_dps_attribution_report` test requires
`POE_FULL_DPS_ATTRIBUTION_REPORT` and never runs the source VM.

## Minimal owned seam

Prefer a bounded compiled reporting layer over the existing occurrence graph,
typed outputs, usage policies and metric declarations. Do not create a second
build model or introduce PoB socket groups into Core.

1. **Declare the report's meaning.** An injected metric/rule definition identifies
   the contributor roles, eligible output types, units, reductions and required
   scenario. A PoB-compatible Full DPS metric must have its own identity and
   documented assumptions. It cannot replace selected-action DPS or silently
   become a claim about sustainable encounter damage.
2. **Bind exact contributors.** A compiled contributor references its source
   occurrence, producing actor, exact action/part/stat-set and typed output.
   Several contributors may share one source without sharing an action or actor.
   Display labels and source XML ordinals remain presentation/import metadata.
3. **Apply composed preferences.** Reuse accepted skill-preset intent and exact
   scenario overrides for inclusion and requested multiplicity. Reuse existing
   provider applicability and count transport where their declared meaning and
   finite domain fit. Do not add another preference store or provider matcher.
4. **Reduce typed values with provenance.** Extend existing bounded reduction and
   explanation machinery only where it lacks the demonstrated operation. Bind
   contributors once; immutable compiled data and worker-local scratch support
   Rayon without XML, Lua or source subprocesses.

These are proposed semantic responsibilities, not approved wire fields. Before
adding a public report-target variant, test whether a new metric on an existing
actor scope plus explicit contributor bindings is sufficient. An additional
public model should solve a demonstrated addressing need, not mirror PoB's
`FullDPS` table.

### Source, effect, action and actor are different identities

One Djinn source supplies a summon and a Command. The summon supplies an actor
with child actions; the Command is an independent player action. They can share
raw source inputs and requested count while producing different damage outputs.
Inclusion of that source therefore needs an explicit declared expansion to its
eligible outputs. It must not mean “sum every descendant action.” The actor's
selected child, every additional effect and any auxiliary actor need their own
correspondence and coverage.

Likewise, two copies of a source remain independent even when they have the same
definition and label. A generated occurrence stays attached to its exact
allocation or equipment/modifier provider. Replacing its item does not transfer
the preference to another item that grants an identically named skill.

Reporting inclusion does not activate a disabled source, create a minion, grant
a skill, or select a new main action. Requested count does not itself create
multiple actors or change reservation. Each such consumer needs its own declared
game semantics. Existing [generated preference ownership](owned-generated-skill-usage-proposal.md)
and the [single preparation/execution graph](owned-preparation-readiness-proposal.md)
remain authoritative.

### Reductions and ties

Use explicit contribution roles rather than a universal `DPS * count` formula.
The pinned source distinguishes counted hit damage, strongest contributions for
several ailments/ground effects, counted Impale, additive Decay, separately scaled
DoT, a capped combined DoT amount and culling calculated after the base rollup.
These source algorithms are evidence to validate; their labels are not proof of
general game rules or of feasible simultaneous use.

For owned maximum reductions, recommend retaining every equal winning origin in
canonical identity order, consistent with existing effect-application traces.
The scalar maximum is unambiguous. The PoB reference trace should separately
retain the actual first winner selected by its strict comparison and traversal
order. Do not copy an incidental label winner into native gameplay semantics.
If an origin controls a later calculation, an equal scalar is insufficient:
that origin-sensitive rule needs its own explicit tie contract and evidence.

Additive traces retain all participating occurrences, including distinct sources
with equal values. Roundings, units, caps, scaling order and empty-reduction
identities must be injected declarations with tested meanings. Neither missing
inputs nor an incomplete candidate list supplies zero or a default count of one.

## Completeness and compatibility boundaries

A complete report needs a complete contributor inventory, resolved eligibility
and inclusion, known multiplicities, and complete required output producers.
Known exclusion can avoid evaluating an excluded contribution. Unknown inclusion
cannot exclude it. Unknown potential winners block a maximum; a known partial sum
is not a completed aggregate. Keep partial diagnostics separately identifiable
and unsuitable for a supposedly complete optimizer score.

PoB's group-count override, generated global-switch rewriting, source-order count
lookup, UI main-group exceptions, cached pass reuse and first-label winner are
source behaviors requiring individual review. Normalize reviewed syntax offline;
do not reproduce Lua truthiness, numeric aliases, iteration order or UI state in
the native report contract. Semantic game behavior, required compatibility and
upstream defects need distinct dispositions under the
[legacy retirement register](legacy-retirement.md).

Aggregation also does not establish a legal rotation, resource sustainability or
simultaneous uptime of competing actions. Those inputs belong to their own
scenario/mechanics contracts. An optimizer must expose the report's assumptions
and must not treat a convenient sum as evidence of those missing constraints.

## Implementation gates and owner input

- [x] Pass and authenticate the new source witness, including independent same-phase
  replay, exact duplicate-source joins, actual tie branches and unchanged five
  originals. Record unresolved secondary attribution explicitly if encountered.
- [ ] Specify a narrow report contract from the observed contributors. Reuse the
  accepted usage store, composition, provider graph and typed rules. Keep current
  count transport and the five-build finalization work independent of this gate.
- [ ] Validate native components across player attacks/spells, minion child actions,
  primary/secondary effects, hit-plus-ailment damage, multiple distinct sources,
  ties, group override zero, inactive sources, missing producers and partial
  contributor inventories. Include a contrasting family beyond Djinn/Firebolt.
- [ ] Test scenario replacement, exact-provider nonselection, missing sources,
  serialization/identity changes, bounded failures, scratch reuse and parallel
  determinism. Preserve all existing selected-action metric requests.
- [ ] Publish the new aggregate metric only after its numerical and contributor
  coverage passes. Update the living implementation checkpoint with its actual
  support limits; whole-build completion remains a separate gate.

**No owner decision is needed for the current evidence work or to reuse the
already accepted preference ownership.** Before introducing the public aggregate
metric, review its documented semantics and whether a PoB-compatible report is a
separately named diagnostic/optimization metric. The recommendation is a separate
explicit metric, preserving selected-action DPS and exposing encounter/uptime
assumptions. A new public report-target shape or an origin-sensitive tie rule would
be a design decision if demonstrated necessary; neither is authorized by this note.
