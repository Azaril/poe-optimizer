# ADR: Typed inputs on skill occurrences

**Status:** Accepted on 2026-10-02; native component and reviewed Direct raw-input import implemented. V5/V17 release publication and five-original preservation validation passed.
**Date:** 2026-10-02
**Decider:** Project owner, following the request to discuss significant model changes.

## Context and constraints at the decision

The five originals still have no complete native evaluation. Original05 retains
six support-target obligations on manual Sand/Water Djinn groups. The
[complete-source witness](owned-djinn-provider-evidence.md) distinguishes these
from allocated tree grants reconstructed with noSupports. Each source supplies
a summon and command sharing inputs, with separate minion descendants and
support acceptance. Manual occurrences survive removal of the corresponding
allocation. This evidence determines source identity; it does not establish
native numerical parity or every supported-property consumer.

The native model could already identify an authored nonphysical SkillUse and attach
physical supports to it. However, SkillUse stored only identity, source, enabled
and scope. There was no authored Skill parameter site, complete/draft storage, or
native read for its occurrence-specific raw level and quality. Direct Skill-owned
Parameter reads resolved to MissingInput. These values cannot be stored
as definition constants or fabricated physical Gem properties.

The distinction matters beyond these fixtures: repeated uses of one definition,
item/tree-granted skills, nonphysical skills and future UI editing need exact
occurrence identity and data-defined input meaning. The evaluator must stay pure
Rust, immutable after binding, parallelizable and independent of source XML/Lua/UI.
New validation belongs in cold request/plan construction, not repeated source
interpretation inside the search loop.

This decision concerns intrinsic skill inputs. Count/reporting, action preferences
and encounter usage remain the separate [skill-preset usage proposal](owned-skill-usage-proposal.md).
Preparation-versus-execution readiness remains the separately accepted
[readiness contract](owned-preparation-readiness-proposal.md). Neither contract
establishes complete numerical coverage by itself.

## Accepted decision

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

## Invariants addressed by the versioned migration

Core authored parameter binding already checks required slots by ParameterSite.
Legacy Engine generated-context gates require every RequiredOnce Skill slot,
without checking an authored/generated domain. Adding a Direct-only raw slot to
that declaration would incorrectly require it on generated siblings.

Legacy ProjectSkillParameter requires projected-only slots (`sites=[]`). Option
A deliberately extends that rule; it must not silently reinterpret historical
packages. Skill-context programs are also instantiated for Direct and generated
occurrences of the same definition. Option B therefore needs explicit program
applicability as well as filtered input requirements; missing-input suppression
is not an implementation of either option.

The versioned implementation below defines migration and old-package semantics. Historical
omission must preserve old identities and behavior where promised. Validate the
selected input authority and required fields before publishing a native request;
generated producers and their execution dependencies still need full plan checks.

## Acceptance and implementation steps

1. [x] Finish the Djinn source witness and record actual manual/generated support,
   raw/prepared input, minion and action relationships. Keep the original saved
   Sniper selection and all 110 requested queries intact.
2. [x] Owner confirms shared typed input slots with explicit producer authority.
   The versioned implementation contract below preserves historical omission.
3. [x] Implement complete/draft storage, strict decoding, bounds, round trips,
   selected-preset isolation, binding and omission compatibility. Two Direct uses
   of one definition must retain different raw values without sharing state.
4. [x] Bind exact authored and generated reads/producers. Exercise a real native
   consumer, generated sibling isolation, missing input, double writers, inactive
   scope, mixed raw/final slots, failure cleanup, A→B→A scratch reuse and parallel
   evaluation. Retain existing projected-input failures and inactive-generation
   behavior as well as coverage gates.
5. [x] Add source-proved Direct import/data declarations for exact manual Sand and
   Water Djinn occurrences. Raw level/quality are authored quantities; unresolved
   count/global/action meanings and parameter inventories remain Pending.
6. [x] Re-finalize all five original requests after publishing the V5/V17 release.
   Account for newly exposed obligations and preserve the six selected Djinn
   support-target issues. Rebuilt artifacts and all 110 queries remain exact.
7. [ ] Supply the remaining final-input producers and topology, then run native
   evaluation when admitted. Numerical parity remains a separate final gate.

Detailed contract audit: `runs/owned-direct-skill-input-design-audit.md`. Relevant
boundaries are Core `owned_build/records.rs`, `owned_draft/records.rs`,
`owned_binding/values.rs`, and Engine `owned_plan/compile/reads.rs` and
`owned_plan/compile.rs`. The source plan is
`runs/owned-default-encounter-djinn-next-review.md`.

## Versioned implementation contract

`SkillUse.parameters` and `SkillDraft.parameters` are optional typed inventories
on the exact authored occurrence. Omission preserves historical serialization;
explicit null is rejected. A physical-Gem use cannot carry this layer. Draft
Pending values remain Pending through finalization and selected-preset composition.

Owned schema V5 adds optional `ParameterSlotSchema.skill_input`, with authored,
projected, or authored-or-projected authority. Authored authority requires the
`SkillParameter` input site; projected-only authority has no authored site. These
declarations belong only to Skill slots. Historical omission retains the existing
generated-only interpretation of slots with no input sites. Required inputs are
checked in their declared producer domain; reading a slot from the wrong domain
remains unavailable, never an instruction to skip a program.

Operations V17 admits these explicit permissions and exact Direct reads. It
inherits V16's checked readiness stages and existing child-only projection
authority. A generated occurrence still cannot receive an authored assignment;
a Direct occurrence still has no computed-parameter self-writer. Raw-to-effective
calculation uses ordinary typed derived channels. Defaults remain schema V4 and
operations V14. The explicit V3 release migration now supports a reviewed endpoint
with schema V5 and operations V17; this does not reinterpret older packages.
The first real Direct-input endpoint passed publication and rebuild validation.

The public native integration proof uses two Direct instances and a generated
sibling of the same injected Skill definition. Their child metrics are
`[40, 40, 44, 12]`; independent raw changes produce `[46, 46, 44, 18]`.
Missing child projections stay unavailable, duplicate writers reject, and unused
required slots gate only their declared producer domain. Reused scratch, a failed
bounded attempt and 48 evaluations across four Rayon workers preserve full reports.
These are synthetic contract tests, not Djinn numerical parity. Topology authoring
and full supported-property assembly remain unfinished.

## Implemented manual source import

The optional `PobManualDirectSkillV1` normalization policy reads reviewed raw
parameters from the exact saved source row. It checks the manual group, canonical
saved-preset framing, source/catalog/role commitments, exact primary Skill and
authored slot authority before creating a Direct occurrence. It creates no
physical Gem. Repeated uses and archived presets retain separate identities and
values; allocation-generated source rows do not become manual Direct instances.
Policy omission preserves historical normalization.

The authored packet in
`data/owned/poe2/3887ae68/direct-skill-inputs/` promotes the two Djinn input schemas
and declares four raw slots. Level is a Count quantity and quality is a
percentage-point quantity, preserving finite fractions and negative raw values.
Missing, malformed and unsupported values remain Pending. The
[passed source witness](owned-djinn-provider-evidence.md#raw-input-boundary-witness)
keeps loaded fields separate from source level validation and final preparation;
the packet imports neither observed final levels nor actor-level constants.

Each imported parameter inventory stays Pending even when its two known scalars
are present. Usage inventories, command/minion topology, action preferences and
support destinations also remain unresolved. Existing exact support-target
obligations are retained; this import supplies no complete-build authority.

V3 migration commits the exact predecessor and validates replacements and new
slot allocations together. It rebinds inherited import dependencies while
preserving query content and existing rules. The authored Direct endpoint adds no
rule programs or evaluation bundle. Import regressions, the optional source
witness, publication and five-original validation have passed. The release in
`runs/owned-direct-skill-inputs-06/package` rebuilds all eighteen files byte for
byte and preserves all 110 queries. Manual Direct row counts are
`[2, 0, 0, 0, 9]`; the same counts of generated siblings remain unmaterialized.
Selected obligations are `119 / 116 / 108 / 121 / 20`, with no complete native
original evaluation. Existing proved payload inventories remain unchanged;
parameter, usage, topology and support coverage is not supplied by this checkpoint.
[Implementation status](implementation.md) records subsequent progress.
