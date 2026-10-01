# ADR: Typed inputs on skill occurrences

**Status:** Proposed; not accepted or implemented.
**Date:** 2026-10-01
**Decider:** Project owner, following the request to discuss significant model changes.

## Context and constraints

The five originals still have no complete native evaluation. Original05 retains
six support-target obligations on manual Sand/Water Djinn groups. The
[complete-source witness](owned-djinn-provider-evidence.md) distinguishes these
from allocated tree grants reconstructed with noSupports. Each source supplies
a summon and command sharing inputs, with separate minion descendants and
support acceptance. Manual occurrences survive removal of the corresponding
allocation. This evidence determines source identity; it does not establish
native numerical parity or every supported-property consumer.

The native model can already identify an authored nonphysical SkillUse and attach
physical supports to it. However, SkillUse stores only identity, source, enabled
and scope. There is no authored Skill parameter site, complete/draft storage, or
native read for its occurrence-specific raw level and quality. Direct Skill-owned
Parameter reads currently resolve to MissingInput. These values cannot be stored
as definition constants or fabricated physical Gem properties.

The distinction matters beyond these fixtures: repeated uses of one definition,
item/tree-granted skills, nonphysical skills and future UI editing need exact
occurrence identity and data-defined input meaning. The evaluator must stay pure
Rust, immutable after binding, parallelizable and independent of source XML/Lua/UI.
New validation belongs in cold request/plan construction, not repeated source
interpretation inside the search loop.

This decision concerns intrinsic skill inputs. Count/reporting, action preferences
and encounter usage remain the separate [skill-preset usage proposal](owned-skill-usage-proposal.md).
Preparation-versus-execution readiness remains the separate
[readiness proposal](owned-preparation-readiness-proposal.md). Neither is implicitly
accepted or solved here.

## Recommended decision

Keep one Skill definition and one occurrence graph. Add typed authored raw inputs
to the exact SkillUse in complete and draft models. Values follow that occurrence
through existing SkillPreset selection. Definitions, parameter IDs, bounds, units,
defaults and raw-to-effective calculations remain injected game data.

Explicitly declare which input slots permit authored values, provider projections,
or both. Where the same raw meaning applies to Direct and generated occurrences,
use the same semantic slot: an authored occurrence supplies its value, while the
exact generated provider supplies it through a checked projection. Shared native
programs consume that slot without guessing its producer from a missing value.

Raw and effective values must have separate identities and producers. A slot
accepting raw level does not accept final level simply because both are numbers.
Likewise, a source setting is not automatically a physical Gem property or an
action choice. The actual source witness determines which declarations are valid
for Djinn; this proposal supplies a reusable contract, not those game semantics.

Preserve one source identity when an occurrence supplies several effects. Existing
Skill and Actor grants can represent a Direct root's command and minion
descendants. This is a candidate topology requiring explicit declarations and
source correspondence; two independent Direct uses are not equivalent. Input
authority and provider ancestry alone do not supply a once-per-source
supported-property aggregation contract.

There must be exactly one valid producer for each required input at each admitted
occurrence. Reject duplicate writers, wrong contexts and cycles; unavailable
projections, missing required values and unresolved producer domains retain the
existing binding/activation semantics. In particular, known-false generated
activation may leave a missing required producer inactive, while active or unknown
activation must not turn that missing input into a value. Preserve completeness
checks. This is not permission to omit a requirement because one fixture does not
read it.

Shared raw slots do not create a Direct final-input writer. ProjectSkillParameter
currently writes a declared generated child only. Direct raw-to-effective
calculations should use existing exact Skill-targeted derived-stat channels;
another computed-parameter writer would need its own explicit contract. Input
permissions alone do not make legacy generated-only consumers executable on Direct
occurrences. Prove every consumer's producer before admitting that definition as
directly selectable; never seed a generated-final slot with raw input.

## Options considered

| Option | Complexity and cost | Benefits | Consequences |
| --- | --- | --- | --- |
| **A. Shared typed input slots with explicit producer authority — recommended** | Changes complete/draft persistence, schema authority, binding and native reads; requires a versioned extension to projection validation. No new evaluator or game interpreter. | One semantic input and downstream calculation can serve manual and provider-generated instances of one Skill definition. Exact occurrences retain independent values. | Must prove producer uniqueness and preserve all legacy required-input behavior. Raw/effective ordering remains explicit. |
| **B. Separate Direct-only and projected-only input domains in one schema** | Preserves current projection exclusivity, but adds explicit program applicability and context-filtered requirements. | Strong separation between authored values and provider computations; potentially smaller initial projection change. | The same mechanic may need parallel input slots and assembly programs. Shared definitions need explicit domains so the wrong program never runs on a sibling occurrence. |
| **C. Separate authored-skill source owner and schema** | Adds another owner/record, lifetime relation and transport into Skill. | Useful if evidence establishes an independently configurable provider with semantics distinct from its supplied skill. | More topology and persistence machinery; current evidence has not established that a second schema is necessary. |

All three can execute with compiled indices and worker-owned scratch. The decision
is about semantic ownership and maintenance, not choosing a runtime database or
another interpreter. Scenario/MechanicChoice storage is not an alternative for raw
inputs: those independently selected presets would introduce unrelated coupling.

## Existing invariants requiring explicit migration

Core authored parameter binding already checks required slots by ParameterSite.
Engine generated-context gates currently require every RequiredOnce Skill slot,
without checking an authored/generated domain. Adding a Direct-only raw slot to
that declaration would incorrectly require it on generated siblings.

ProjectSkillParameter currently requires projected-only slots (`sites=[]`). Option
A deliberately extends that rule; it must not silently reinterpret historical
packages. Skill-context programs are also instantiated for Direct and generated
occurrences of the same definition. Option B therefore needs explicit program
applicability as well as filtered input requirements; missing-input suppression
is not an implementation of either option.

Specify wire/version migration and old-package semantics before coding. Historical
omission must preserve old identities and behavior where promised. Validate the
selected input authority and required fields before publishing a native request;
generated producers and their execution dependencies still need full plan checks.

## Acceptance and implementation steps

1. [x] Finish the Djinn source witness and record actual manual/generated support,
   raw/prepared input, minion and action relationships. Keep the original saved
   Sniper selection and all 110 requested queries intact.
2. [ ] Owner confirms the direction. Resolve the exact input-authority/version
   contract before making a public Core or schema change.
3. [ ] Implement complete/draft storage, strict decoding, bounds, round trips,
   selected-preset isolation, binding and omission compatibility. Two Direct uses
   of one definition must retain different raw values without sharing state.
4. [ ] Bind exact authored and generated reads/producers. Exercise a real native
   consumer, generated sibling isolation, missing input, double writers, inactive
   scope, mixed raw/final slots, failure cleanup, A→B→A scratch reuse and parallel
   evaluation. Retain existing projected-input failures and inactive-generation
   behavior as well as coverage gates.
5. [ ] Add only source-proved Direct import/data declarations. Preserve unresolved
   count/global/action meanings; do not complete a list merely from known members.
6. [ ] Re-finalize all five original requests and run native evaluation when
   admitted. Account for newly exposed obligations before claiming any reduction
   in the six support-target issues. Numerical parity remains the final gate.

Detailed contract audit: `runs/owned-direct-skill-input-design-audit.md`. Relevant
boundaries are Core `owned_build/records.rs`, `owned_draft/records.rs`,
`owned_binding/values.rs`, and Engine `owned_plan/compile/reads.rs` and
`owned_plan/compile.rs`. The source plan is
`runs/owned-default-encounter-djinn-next-review.md`. No implementation is authorized
by this proposed ADR alone.
