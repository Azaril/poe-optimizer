# Support receiving, applicability and activation

Status: accepted by the project owner on 2026-09-27; shared rule/routing implementation is pending.
This is the next D3 integration priority. It preserves the existing BuildSpec identities,
whole-plan coverage gates and PoB-independent native evaluator. See the
[domain architecture](domain-architecture.md) and [implementation resume](implementation.md).

## Problem and existing seams

The engine currently rejects every SupportAssignment before instantiating its rules.
Simply removing that rejection would be incorrect: action contexts require the action's
provider to equal the rule provider, while a support must retain its own origin and affect
a different, explicitly selected receiver. Core already gives each assignment an exact
support GemInstance, enabled flag and authored/generated SkillTarget. SupportApplicability
already emits a Boolean, but does not define receiving scope or activation.

The five original saved projects contain 338 assignments; 273 have known identity, enabled
state and authored target. Catalog-driven input conversion now supplies Known Gem schemas
and two reviewed intrinsic scalars for all 338 physical support occurrences. Their parameter
collections remain Pending. The [full-release correction](owned-releases.md) also reopens
the prematurely Complete-empty direct parameter and choice declarations on Twister/Sniper.
All 478 original Gem parameter collections remain Pending. The subsequent active-Gem
conversion adds Known schemas for 36 definitions / 77 occurrences; 17 active definitions /
51 occurrences retain Unmapped schemas. The correction preserves known
members and every original query, without relaxing existing successor contracts.

The injected [skill-scope policy](owned-skill-scopes.md) admits the missing parent slot on
all 140 authored skills as Shared; enabled state, global effects and generated providers
remain independent. Input knowledge and potential Skill memberships do not establish
support applicability or delivery. Support integration removes a universal engine limitation;
it cannot by itself complete any original build. All 110 original queries remain.

## Accepted contract

1. **Explicit receiving scope in definition data.** A support definition declares which
   outputs of its assigned skill receive each effect. Effects on generated actors/actions
   require an explicit, bounded declared path or receiving role. Resolve these against
   finite owned declarations; never propagate by UI group, name, incidental provider ancestry
   or an unbounded descendant search. Do not equate applying to the summoning action with
   applying to every action of every generated actor.
2. **Separate origin and receiver.** Effect provenance and gem level/quality/parameters/
   choices remain bound to the support assignment. Action, actor and supported-skill reads
   use the exact receiving context. Reusing a definition for two assignments cannot merge
   their intermediate state or spill an effect into an equal-named sibling skill. Internal
   application identity includes the assignment, resolved receiver and any prepared-list position retained by the selected policy; see the selection evidence below.
3. **Explicit applicability and activation.** Enabled assignment, available/active target,
   and known true applicability are required before delivering effects. Known false means
   inactive effects plus an explicit applicability/legality result. Missing, unsupported or
   unknown applicability remains unresolved. Never default missing evidence to true or
   unknown to false. Keep invalid-but-computable reporting separate from numeric omission.
4. **One reviewed final applicability producer per application.** Conditions may be combined in
   one ordinary typed expression. Competing final producers and dependency cycles reject;
   no implicit OR/AND reduction or winner among final numeric writers. Component source facts may be
   computed separately, but effects on receivers require the application gate.
5. **No coverage relaxation.** Partial receiving declarations, target topology, source
   inputs, support programs or active contributors remain gaps. Preserve all 110 original
   queries and current whole-plan completion checks. No per-metric bypass, source UI replay,
   fixture dispatch or default empty input collections are part of this proposal.

Exact versioned routing/selector DTOs follow this contract after review. Prefer the existing
actor/action/provider bindings and rule expressions. Add a narrowly scoped owned relation
only where existing selectors cannot express the origin/receiver distinction. The executor
must receive injected data and remain native, deterministic and independent per worker.

## Implementation slice and proof

First bind authored and generated targets, source reads, receiving contexts and application
gates with directly authored owned requests. Cover player/minion receivers together, disabled
assignment/target, false/unknown applicability, missing inputs, competing producers, cycles,
duplicate definitions/assignments, generated targets, sibling non-propagation and bounded
expansion. Existing unsupported cases stay explicit until their declared semantics exist.

Then add a real injected support definition and import mappings, selected from the originals.
Original02's selected Twister uses Elemental Armament II. Its game ID retains
SupportGemPrimalArmamentTwo, while its variant is ElementalArmamentSupportTwo and granted
effect is SupportElementalArmamentPlayerTwo. Source tests establish this exact correspondence;
the existing synthetic engine fixture alone does not establish source parity. Original05's selected Sniper has no attached support and can provide a no-spill control
for supported sibling groups. Add an actual supported minion contrast after reviewing its
receiver semantics. Compare applicability, receiving identities and intermediate contributions
with the optional PoB oracle; final metric parity still requires remaining producers.

Close gem input collections, skill scopes and selected-preset membership only from reviewed
schema/source facts. This path must use the same data, engine and interfaces as future search.
Do not add a support-specific or third-skill Rust evaluator. Existing socket-configuration and
per-metric coverage proposals remain separate pending decisions.

## Support interaction evidence and accepted parity policy

Four optional Rust tests in `crates/poe-optimizer-pob/tests/owned_support_reference.rs`
execute authenticated, unchanged source functions at pin `3887ae68`. Actual data shows:

- Arcane Surge adds Duration to Firebolt, enabling Prolonged Duration in either tested order.
- Brutus' Brain adds an undamageable-minion type to Wolf Pack, excluding Feeding Frenzy
  even when it was initially eligible. Eligibility is not monotone merely because types grow.
- Elemental Armament II emits an attack-keyword-filtered elemental MORE modifier; its
  source merge gives 1.25 for an Attack query and 1 for Spell. Its cost field is observed,
  but full cost calculation remains untested.
- Original01's selected Skeletal Cleric/Meat Shield II is an active minion contrast. Its
  Damage/DamageTaken effects retain nested minion wrappers. Original05's saved Wolf Pack/
  Feeding Frenzy instances occur in inactive presets; they are not its selected Sniper.

**Design refinement:** add a separate bounded type-preparation stage before final
applicability and effect delivery. Finite injected type declarations, explicit receiving
contexts and bounded repeated eligibility checks belong here. Keep ordinary numerical rule
programs acyclic; a support interaction is not permission for arbitrary numerical cycles.
Recheck final applicability after type preparation. Do not infer complete activation from
an early accepted support or conflate summoner requirement types with exclusion types.

The source algorithm retries rejected supports, but it is not a correct general fixed-point
algorithm. A separate test with explicitly synthetic require/add pairs proves that deleting
an early rejected-list entry leaves a hole that terminates a later `ipairs` retry. One order
admits a delayed support without applying its added type, excluding a downstream support;
a reordered input admits both. This is a demonstrated algorithm edge case, not a claim that
a supplied original build currently hits it.

**Owner decision accepted (2026-09-27):** preserve pinned PoB behavior by default, with
ordering and selection quirks isolated in an explicit versioned preparation policy.
The separate origin/receiver and false/unknown contracts above apply. The policy is
approved but not yet implemented. Do not bake a sparse Lua table into the owned model or
claim corrected closure is exact parity. Compatibility remains native and injected, with
no source UI/runtime dependency. An intentional deviation requires a separate decision.

The earlier interaction tests exclude full preset/enabled selection and support replacement
precedence; the selection tests below establish specific replacement branches. Remaining gaps include
item-granted supports, additional granted effects, effective minion transfer, complete costs
and whole-build metrics. Close these through the same declared semantic path before claiming
complete support or original-build parity.

## Selection is a separate ordered phase

Four optional Rust tests in `owned_support_selection_reference.rs` execute complete,
authenticated `CalcSetup` selection/construction closures and `createActiveSkill` with JIT
disabled/enabled. They do not assert compiled traces, execute full `initEnv`, or establish
final numerical modifier delivery.

| Pinned source case | Observed selection |
| --- | --- |
| Same granted-effect identity | Higher level, then higher quality wins; an exact tie retains the first occurrence. |
| Different definitions with intersecting families | The later encounter replaces the earlier entry, regardless of tier, level or quality. |
| Both definitions have family lists | The family branch precedes plus-version handling, even when the families do not intersect. |
| One incoming effect intersects multiple earlier entries | Multiple list positions can retain the same incoming effect object. |

The multiple-position witness uses real Salvo after real Multishot and Unleash definitions
in an explicitly authored component input. It is not an original-build combination or a
claim of legal equipment. Complete `createActiveSkill` retains both selected positions;
final double numerical delivery has not been established. Plus-version contrasts are
explicitly synthetic because the pinned game data declares no such relation. MAIN/CALCS
superseded/display flags differ and must not become domain authority.

The same tests retain the exact saved active-preset Twister and Cleric support rows, game/variant
IDs and order. Empty surrounding property/socket modifiers are controlled inputs, so these
observations do not establish whole-build effective gem levels, costs or minion transfer.
Source-generated and cross-linked support merges also depend on surrounding group traversal;
a native sorted set cannot claim parity merely because its members match.

Refine the proposed pipeline to **origin discovery → ordered selection → bounded type
preparation → final applicability → effect delivery**. A prepared application is distinct
from its physical support gem and authored assignment. If the chosen compatibility policy
retains multiple positions, derived application identity must distinguish them for the same
origin and receiver without fabricating duplicate physical gems. Assignment/receiver alone
is then insufficient as an application key. Keep candidate legality and numerical selection
separate so an invalid support combination cannot silently become a valid optimizer result.

The native boundary should receive explicit owned relationships and ordering semantics,
with import correspondence outside evaluation. It must not traverse PoB UI groups. These
findings constrain the accepted policy. Native selection, multiplicity and reference-quirk
compatibility are approved but remain unimplemented. All original-build and whole-plan
gates remain unchanged.

## Next implementation boundary

The post-actor audit found that both `BuildInput.supports` and `SkillPreset.supports`
canonicalize by occurrence ID. Those collections establish membership, not semantic
selection order. Add explicit ordered origin sequences keyed by the exact preparation
target, preserve them through project composition and give drafts an explicit Pending
representation. Follow the existing distinction between item modifier membership and
`modifier_order`; do not reinterpret allocated IDs as ordering evidence.

A small versioned support-definition package should bind the schema and rules while
carrying finite receiving declarations, selection/family relationships, type-preparation
facts and the accepted policy identity. Its digest belongs in plan identity. Existing
action-stat routing should keep its separate responsibility. Missing definitions or
semantic ordering remain unresolved. Effective support level/quality must come from
explicit preparation dependencies when needed; raw physical Gem values are not fallback
effective values.

The first Engine increment must split effect origin from receiving context. Origin Gem
values/choices and provenance remain on the assignment; actor/action and generated-input
reads use the receiver. Key each prepared application by assignment, exact receiver and
retained list position. Give it a unique applicability producer in the ordinary dependency
graph before delivering effects. The current unkeyed applicability effect cannot enforce
that single-writer contract. Applicability must not depend on its own delivery gate; actor
effects must not be repeated once per receiving action.

Implement and test this boundary first with directly authored player and generated-actor
requests, including same-template actors, false/unknown/disabled cases, sibling isolation,
ambiguous writers, limits, ordering changes and scratch/parallel reproducibility. Then
connect the documented selection/type witnesses and real Twister/Elemental Armament II,
Cleric/Meat Shield II and Sniper no-spill contrasts. All five originals and 110 query rows
remain the integration corpus. This is the next increment within the accepted design,
not a new source-runtime interpreter or an exemption from complete-build validation.

## Related membership scaling investigation

The compact authoring patch still emits materialized per-template memberships. The latest
two-family expansion occupies 9,516,263 of the 16,777,216 allowed bytes. A broadly assigned
family repeats 1,756 identifiers, adding approximately 189,648 membership bytes before rules
and descriptors; the remaining room is at most 38 families from IDs alone, actually fewer.
Repeated one-family commands cannot avoid copying accumulated descriptors.

Investigate shared immutable finite membership sets or equivalent factoring before dozens
more broad families. Preserve typed identities, Partial/Complete meaning, deterministic
bounded lookup, artifact identities and separate affix legality. Structural membership is
not proof an affix can roll on a base. Do not raise caps or add a wildcard-all-modifiers
fallback as a substitute. This is a separate format proposal; no migration is approved here.
