# ADR: preset-specific raw inputs for generated Skills

**Status:** Option A accepted by the owner on 2026-10-05. The concrete Core,
Data, Engine and CLI contracts pass focused validation; source-bound Import
joins and the real-build publication remain open.
**Date:** 2026-10-05.
**Decider:** Project owner.

## Problem and existing boundaries

Saved generated quality is part of the selected skill configuration. It cannot
be replaced by a constant on a tree node or item definition. The accepted
[generated-usage decision](owned-generated-skill-usage-proposal.md) keeps usage
intent in the selected SkillPreset, with exact provider applicability. That
decision does not supply raw Skill inputs.

The [generated-setting source witness](owned-djinn-provider-evidence.md#generated-saved-settings-and-quality)
passes 46 cases, including fifteen original/archived selections and independent
generated-group mutations. In both JIT modes, saved quality 12.5 survives
reconstruction and reaches prepared quality 12.5 for Tree Sand Djinn, Tree Water
Djinn and item Firebolt, including both Djinn Commands. Reports are
`runs/owned-generated-skill-usage-source-01/source-jit-{off,on}.json`, each
64,295,778 bytes, SHA256
`75ca2ba05fdecc465d4727f43b2a31358ffcd2fb134a80281534bc4d24a867d5`.
This establishes the demonstrated input and exact PoB source-object joins; it
does not establish the Import join to owned Allocation/EquipmentUse/ItemModifier
occurrences, complete quality mechanics, arbitrary defaults, or a canonical PoB
lifecycle.

Preserving a saved raw value does not establish in-game legality or authorize
the optimizer to vary it. Admitted edit domains and legality remain injected
game rules. If evidence later classifies a source-only quality behavior as an
upstream defect, apply the existing narrow reference-exception policy instead
of promoting it into a game rule solely to match PoB.

Existing contracts already provide most of the intended model:

- [Typed Skill inputs](owned-skill-occurrence-input-proposal.md) share semantic
  slots between Direct and generated occurrences. Direct `SkillUse.parameters`
  stores authored values; generated values use checked provider projection.
  [`ParameterSlotSchema.skill_input`](../crates/poe-optimizer-core/src/owned_schema.rs)
  explicitly distinguishes authored and projected authority. It currently does
  not authorize saved preset values on a generated target.
- [`SkillTarget::Generated` and `GeneratedSkillKey`](../crates/poe-optimizer-core/src/owned_build/records.rs)
  already identify the exact provider root, grant path and supply slot. An
  ItemModifier root includes its EquipmentUse and modifier instance. No extra
  Gem or SkillUse root is needed for a saved generated representation.
- [`SkillPreset` and `compose_request`](../crates/poe-optimizer-core/src/owned_project.rs)
  provide independent skill-preset selection and one composition boundary.
  The accepted usage applicability extension now shares the checked proof and
  composition boundary described below; real-source integration remains open.
- [Preparation readiness](owned-preparation-readiness-proposal.md) and the
  [source-property relation](owned-source-property-preparation-proposal.md) keep
  raw inputs, supported properties and final inputs on one graph. The current
  source-property contract admits authored input owners; generated-owner
  aggregation is not implicitly added by this proposal.
- [`usage_programs`](../crates/poe-optimizer-engine/src/owned_plan/compile/usage.rs)
  cannot `ProjectSkillParameter` or create providers. Usage also inherits the
  target's readiness gates. Turning quality into a usage policy would introduce
  a missing-input cycle or require weakening that deliberate boundary.

Tree Djinn programs already project raw level into slots `3261` and `3263`;
their shared raw quality slots are `3262` and `3264`. The item Firebolt modifier
projects raw level into `31ca`. Those real producers remain authoritative.
Firebolt's quality declaration and every downstream consumer must be checked
separately; the source witness is not permission to invent a slot or final value.

## Ownership options

| Option | Ownership and composition | Benefits | Costs |
| --- | --- | --- | --- |
| **A. SkillPreset owns exact generated-input bindings — recommended** | A versioned preset inventory stores typed assignments addressed to exact generated Skill targets, with required or exact-source-selected applicability. Composition selects this intent alongside the skill preset and carries applicable bindings into the existing request. | Matches the demonstrated independent skill-preset quality. Reuses exact targets, shared slots, selection diagnostics and one graph. Provider changes cannot silently retarget intent. | Adds explicit generated-input producer permission, stored schema validation and input binding at cold plan construction. Applicability machinery should be shared with usage, but raw inputs retain a distinct meaning. |
| **B. Provider owns its generated-input configuration** | Typed assignments live with the exact Allocation or ItemModifier provider configuration, naming its declared supplied Skill and shared slots. Selecting the provider configuration supplies the values. | Keeps all provider-supplied inputs at their structural owner; a provider's configuration is locally reviewable. | Different quality in two skill presets requires explicit provider-configuration variants or combined selections. A skill-preset override recreates A with two competing ownership layers. It couples independent tree/equipment and skill alternatives and adds migration/editing cost. |

Both options preserve one occurrence graph and the same parameter identities.
Option B is reasonable if generated inputs should always change with provider
configuration. Current source evidence instead shows quality saved with skill
alternatives, so A is the smaller end-state change. Do not implement B by adding
duplicate provider-level quality slots and copying between parallel schemas.

## Accepted bounded contract

Names below describe semantics, not approved wire fields or version numbers.
Add one optional, explicitly versioned preset inventory of generated-input
bindings and its draft equivalent. Each binding contains an exact generated
Skill target, existing typed parameter assignments, and applicability under the
accepted generated-usage rules. Use one selected input list in the existing
build/request boundary; do not introduce another evaluation request or graph.

Schema/rule metadata must explicitly authorize the supplying declaration to
accept preset input for particular raw Skill slots. It must resolve to that
declaration's actual supplied Skill and a compatible shared slot. Existing
`AuthoredOrProjected` permission alone must not silently acquire this new
meaning. The binding becomes a checked input producer for the exact generated
parameter key, retaining preset/source provenance. It neither calls arbitrary
rule programs nor broadens `ProjectSkillParameter` to self-writes.

Within the effective selected preset and all potential native writers, there is
exactly one potential producer per target/slot. Alternative stored presets may
assign different values to the same target. A selected preset input cannot
replace an existing provider projection by priority, absence or evaluation
order. The first slice rejects such conflicts, even if one producer is inactive
or an unrelated query would not read the slot. Provider raw levels therefore
remain unchanged; quality is admitted only where explicit authority and a unique
producer are established. Do not provide zero when the saved input is missing.

Scenario usage overrides remain usage-only. Raw quality is a build input and
receives no implicit scenario override. A user or optimizer changes the selected
preset's exact input record through an explicit edit. Duplicate target/slot
records are invalid; composition does not merge duplicate values, even equal
ones. A later need for a second raw-input layer would require an explicit
precedence decision, not reuse of usage whole-record replacement by accident.

The selected provider graph supplies and activates the Skill. The input binding
supplies only its value. Bind against structural supply before preparation needs
the value, then retain ordinary activation and phase gates. Required quality must
be available before any preparation reader, including usage or support admission
that depends on it. Active or unknown activation cannot convert a missing input
to zero; proven inactivity remains inactive without relaxing coverage. Cold
validation must include all potential writers, late dependencies and cycles.

One root quality binding does not fan out to every descendant. Commands and
minion children need their declared input relationships and ordinary producers.
Reusing source-property aggregation for generated owners remains a separate
checked relation change if actual mechanics require it. This input step alone
does not certify final quality, support acceptance or complete original builds.

## Selection, persistence and validation gates

| Case | Required behavior |
| --- | --- |
| Two presets configure the same exact provider differently | Selecting either preset gives its own raw input; A/B/A restores the first result. |
| Two providers grant the same Skill definition | Preserve distinct targets and values. Never pick the first matching definition, fan out, or share by display name. |
| Exact source exists but is excluded by complete selected membership | An explicitly optional binding is dormant, with a recorded reason; a required binding fails. |
| Source was deleted, is foreign, ambiguous, or ownership is unresolved | Keep a draft obligation or reject the complete record. Historical source text does not prove deliberate nonselection. |
| Provider is selected but its supply/grant path is missing or Partial | Preserve ordinary invalid/unresolved topology and coverage results; applicability supplies no authority. |
| Selected provider is disabled or its activation is unknown | Preserve native inactive/unresolved results; do not use applicability as activation. |

Validate every stored resolved new-contract record against the exact schema,
including unselected and dormant records: namespace, target kind, provider/item
ownership, bounded path, declaration membership, producer permission, required
values within the preset-authorized input domain, types, units and ranges.
Provider-projected raw levels and downstream final inputs retain their separate
required-input producers; preset records do not have to author every RequiredOnce
Skill slot. A dormant record cannot hide malformed quality.
Unresolved draft records keep their existing isolation and cannot receive a
false not-applicable certificate. Reuse the data-aware proof required by the
accepted usage ADR instead of adding another schema checker.

Global identity/applicability proof must precede draft finalization's selected
projection, which otherwise loses the distinction between deleted and unselected
providers. Proof and cache identities include project content, preset inputs,
selection and data identity. Bound work, rows, paths and candidate bytes during
cold construction; evaluation remains immutable with worker-owned scratch.

Legacy omitted-field bytes and identities, Direct input behavior and provider
projections remain unchanged. New input contents participate in request identity.
New fields reject explicit null and
unknown fields and require an opt-in version. Complete/draft canonicalization,
checked release rebinding and stale explicit replacements need migration tests.

## Implementation and evidence gates

### Concrete implementation boundary

`SkillPreset.intent` is an optional `SkillPresetIntentV1` envelope, with separate
usage and generated-input lists. It cannot coexist with `usage_preferences`,
including an explicitly present empty legacy list. `from_legacy` is an explicit
authoring conversion; decoding does not migrate old bytes. Draft lists preserve
their unresolved fields and original issue identities.

Core proves stored intent over real project/draft occurrence tables, before
selection. Its opaque proof commits all authoring content and the injected data
identity. Stored schema checks reuse the ordinary binder without inventing a
selected character or a union of choice presets. Explicit scenario usage is
checked against the actual selected request before replacement. Required choices,
native activation, producer completeness and numerical readiness remain later
checks. A resolved record with unresolved source dependencies cannot receive a
not-applicable certificate.

Checked composition emits `BuildInput.generated_inputs`, a versioned selected
list of exact targets, shared parameter assignments and skill-preset origins.
The public structural-only composition/finalization APIs reject the new preset
envelope without its proof. The CLI's `check-owned-draft --definitions` supplies
the data-aware path and exposes its diagnostics separately from whole-request
definition binding. Neither a proof nor successful finalization means a build
can already evaluate.

Schema V6 adds `SkillGrantSlotSchema.preset_inputs`: a versioned Complete allowlist
on the exact supplying declaration. Each member belongs to that supplied Skill
and has explicit projected authority; legacy implicit authority and Direct-only
slots are insufficient. `ParameterSite::SkillParameter` continues to describe
Direct authored input and is not an extra requirement on projected-only slots.
The generic Core schema index validates semantics independently of a particular
package format; the Data loader owns the V6 format gate.

Operations V19 admits a literal request producer in the existing parameter graph.
It retains the supplying parent's gates, uses Structural readiness, and declares
no artificial rule stage. Preparation includes the producer through its actual
dependencies and rejects late parent dependencies. Cold compilation checks all
potential provider projections before activation or query filtering. There is
no priority override or scenario raw-input layer. Migration V4 moves reviewed
V5/V17–V18 or V6/V19 endpoints to V6/V19, preserving prior numerical programs,
queries and required evaluation artifacts. Earlier migration meanings remain
unchanged.

These contracts contain no skill, item, source text or PoB runtime dispatch.
The published game-data baseline remains the V5/V18 Command package until a
source-authenticated successor supplies actual permissions and input joins.

### Acceptance gates

1. [x] Implement the versioned preset/draft/request and exact supply-input
   permission, sharing applicability validation with generated usage while
   retaining separate raw-input records. Concrete contracts are described above.
2. [x] Bind inputs through the existing Core schema and Engine parameter
   machinery. Focused component tests cover duplicate providers/writers,
   required missing input, wrong owner/unit/range, dormant-invalid records,
   deleted source, late dependencies, inactive/unknown activation, bounded
   failure/retry, independent preset A/B/A and parallel scratch reuse. The support
   suffix preserves literal inputs in the executed prefix and checks ordinary
   dependency stages through them; no authored stage is invented for an input.
3. [ ] Add source-bound Import joins for the actual selected/archived provider
   occurrences. Tree node IDs alone are insufficient across allocation presets;
   item source text must resolve through the actual EquipmentUse and modifier.
   Preserve ambiguous, stale and missing joins as Pending. Create no authored
   SkillUse or physical Gem for a generated representation.
4. [ ] Replay raw quality 12.5 and independent preset/provider controls using the
   authenticated source witness. Preserve the provider level programs, manual
   counterparts, all five originals and 110 queries. Any new default/domain or
   shared-effect propagation needs separate source evidence and declared rules.
5. [ ] Keep usage inventories, final quality, support/source-property completeness
   and remaining mechanics open until their own consumers pass. Closing a raw
   input obligation is useful progress, not permission to evaluate a Partial build.

The implemented component contracts do not establish source conversion or
whole-build numerical coverage. They add no new persistence layer and do not
revisit the handling of upstream source bugs.
