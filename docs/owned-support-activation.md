# Support receiving, applicability and activation

Status: accepted by the project owner on 2026-09-27. Ordered input, native preparation,
and computed preparation through finite rule stages are implemented. Shared effect-plan
receiving/application integration remains pending.
This is the next D3 integration priority. It preserves the existing BuildSpec identities,
whole-plan coverage gates and PoB-independent native evaluator. See the
[domain architecture](domain-architecture.md) and [implementation resume](implementation.md).

The owner reaffirmed the stalled recommendations on 2026-09-30. The accepted actor,
support and finite-stage decisions require no further approval. Independent numerical
coverage for a stage under Partial whole-owner membership remains a separate decision.

## Problem and existing seams

Ordinary effect/metric plans still reject support delivery. A separate preparation plan
admits only assignment-local preparation programs. Simply removing the delivery rejection
would be incorrect: action contexts require the action's
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
implemented in the native preparation component; effect-plan integration remains pending.
Do not bake a sparse Lua table into the owned model or
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
compatibility now have a component implementation. All original-build and whole-plan
gates remain unchanged.

## Next implementation boundary

### Delivered preparation boundary

Builds and skill presets carry optional `support_origins` sequences keyed by exact
`SkillTarget`. Record tables still canonicalize membership by ID; sequence members retain
semantic order. The extension is omitted from old artifacts, preserving their bytes and
identity. Explicit null is rejected. Omission is unconverted order, never permission to
infer order from occurrence IDs. Drafts retain Pending sequence membership and order;
selected unresolved order prevents finalization.

The import adapter has an opt-in `SavedManualGroupOrder` policy. It retains physical
assignment encounter order for targets already proven by the manual-group import rule,
including disabled assignments and duplicate definitions. It neither guesses targets nor
closes merged/generated/item origin discovery: each opted-in preset's outer order list
stays Pending. The five-original test preserves 338 assignments and all 110 query rows,
with local sequences for the 273 known authored-target assignments. Omitted policy retains
historical serialization and allocator behavior. No production release has enabled it yet.

`PreparedSupportKey` separates a target, physical origin and retained position.
`SupportApplicationKey` adds the exact actor or action receiver. These are derived keys,
not new physical Gem occurrences. Actual effect-plan instantiation is still outstanding.

The injected `OwnedSupportPreparation` package owns finite type/effect/family vocabularies,
typed Boolean predicates, selection relationships and an explicit preparation policy.
It binds exact schema and rule identities and a declared effective-quality unit. Unknown
definitions remain Unmapped or absent. Family absence and present-empty are distinct;
same-effect declarations must agree. Construction, decoding and encoding enforce bounded
size, traversal and predicate depth, including tighter caller limits.
Effect symbols encode reviewed selection identity. An adapter must not collapse distinct
source effect objects merely because their display names or external IDs match.

The native preparation component receives explicit resolved facts and private per-call
state. It implements ordered replacement, retained duplicate positions, the pinned retry
frontier, retained type additions and final eligibility recomputation. Missing relevant
effective values or eligibility facts remain unresolved. Child added types and summoner
eligibility types are separate. An absent summoner minion-type collection can use the
child collection; a present-empty collection cannot. Neither a component result nor a
Known definition closes contributor coverage or proves game legality.

The build-backed preparation entry point consumes validated BuildSpec ordering and derives
each assignment's Gem, target and enabled state from that build. Supplied effective values
cannot replace these facts. It respects authored target disablement and weapon-loadout
scope; generated topology and activation still require an explicitly resolved receiving
context. Missing order stays unresolved. Physical Gem level/quality never substitute for
effective inputs, and missing order never becomes ID-sorted order.

Both component entry points also accept a shared evaluation-attempt work budget. Binding,
selection and type preparation consume one decreasing allowance, bounded by the component
limit as well. Failed attempts and early unresolved/inactive outcomes consume their work;
exhaustion cannot be retried with a replenished allowance inside the same attempt. This
resource contract does not make caller-provided effective values authoritative.

The initial package binds one declared preparation effect per Gem definition. Additional
granted effects, item-granted origins and cross-target discovery remain explicit integration
work. Before admitting those families, extend discovery with ordered effect identities and
bind its completeness; never infer extra effects from names or silently treat this initial
mapping as exhaustive. Receiving selectors belong in a separately validated relation bound
into plan identity. No support delivery or numerical coverage is authorized by the
preparation-only package.

### Remaining effect-plan integration

The post-actor audit found that both `BuildInput.supports` and `SkillPreset.supports`
canonicalize by occurrence ID. Those collections establish membership, not semantic
selection order. Add explicit ordered origin sequences keyed by the exact preparation
target, preserve them through project composition and give drafts an explicit Pending
representation (now implemented above). Follow the existing distinction between item modifier membership and
`modifier_order`; do not reinterpret allocated IDs as ordering evidence.

A small versioned receiving relation must accompany the preparation package while
carrying finite receiving declarations. Both digests belong in plan identity. Existing
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

Selection's retained positions depend on computed effective values, whereas the current
scalar-effect plan has fixed topology. Model preparation as a bounded finite stage whose
input dependencies come from ordinary producers and whose result is private to the worker.
Bind subsequent applications to that result and its exact input commitment. Do not execute
preparation during plan compilation from caller-supplied effective scalars, and do not
expand every origin against every possible position to simulate dynamic selection. The
accepted finite-stage design needs a compact application scheduling boundary here.

Discover finite topology before instantiating owner programs. Gem/Skill Action programs
previously ran during provider discovery; adding support receivers only before the later
ActionOutput pass would miss those programs. The compiler now records a bounded FIFO owner
inventory, leaving a receiving-action registration boundary before owner replay. Replay
preserves invocation/contribution order and first-occurrence diagnostic order. Its
independent owner-binding cap does not consume the actual invocation limit. Generated-supply
producer coverage is audited after replay. Concrete support receivers still need binding.

Preserve origin-bound Parameter/Choice/Gem reads. Receiving Skill and Action inputs need
explicit typed read forms; changing a context pointer cannot satisfy their existing
declaration-ownership checks. Source-owned grant/projection effects require their own
declared contract before being admitted to support delivery. A new operation version must
retain explicit v11 actor-supply semantics and its existing plan identity while giving
support-capable plans a new identity binding the preparation and receiving relations.
The closed operation-version registry now freezes v6 through v11 capabilities and plan
domains independently of the latest-version alias. Schema v4 / operations v12 now add
assignment-scoped `SupportOrigin` and exact `Skill` value channels; ordinary v12 effect
plans use the v9 identity domain. V2/v3 schemas and explicit v6–v11 rules retain their
historical semantics. This revision does not authorize receiver delivery.

### Computed preparation boundary

`OwnedEvaluationStages` binds a finite stage DAG, owner-qualified program assignments,
routing stage and typed frozen channels to exact schema, stored rules and routing artifacts.
The typed channel model distinguishes final values, contributions, modifier transforms,
grant activation and generated-skill input projections. Static validation includes potential
writes even when currently false; the bound plan additionally checks concrete activation
and required-input dependencies. A Complete program classification means only that all
known programs have a stage. It does not close Partial rule owners or contributor sets.

`OwnedSupportInputBindings` maps preparation inputs to computed channels. Effective level
and quality belong to each exact assignment; type membership and eligibility facts belong
to the exact assigned SkillTarget. The data declares Boolean membership for the entire
finite support-type vocabulary. Optional minion and summoner collections have explicit
computed presence channels, preserving absent versus present-empty semantics. Missing
presence or membership facts never imply false or empty. Quality uses the exact injected
unit. All exported channels must be frozen at or before preparation.

The native `OwnedSupportPreparationPlan` compiles from these immutable packages and an
ordinary owned request. It schedules existing rule effects, privately exports computed
inputs and calls the existing bounded native preparation policy. Caller-supplied scalar
facts and public diagnostic reports cannot enter this path. Stored-rule identity and
compiled executable identity are distinct: `compile_stored` retains both commitments;
raw compilation does not manufacture stored-artifact provenance.

Origin programs may write only their own values/contributions/capabilities and requirements.
Player reads remain explicit; assigned-skill reads cannot become receiver or summoner reads.
Actor/action delivery programs keep preparation unavailable until receiving applications
are bound. Normal effect plans preserve their support rejection. Full owner, declaration
and contributor coverage remains required even when the selection policy does not need
level or quality, such as a target with one support.

Generated-target activation follows the discovered full supply path, including the final
entering grant, actor activation and required projected inputs. A parent provider path alone
is insufficient. Worker scratch has a private plan/attempt seal; scalar stages, exports and
preparation share one decreasing allowance. Reports retain the exact failed origin/skill
and stat when a demanded computed input is unavailable. Public results remain preparation
diagnostics, not permission to deliver effects or mark a build complete.

Implement and test this boundary first with directly authored player and generated-actor
requests, including same-template actors, false/unknown/disabled cases, sibling isolation,
ambiguous writers, limits, ordering changes and scratch/parallel reproducibility. Then
connect the documented selection/type witnesses and real Twister/Elemental Armament II,
Cleric/Meat Shield II and Sniper no-spill contrasts. All five originals and 110 query rows
remain the integration corpus. This is the next increment within the accepted design,
not a new source-runtime interpreter or an exemption from complete-build validation.

### Finite-stage execution requirements

Stage scheduling must reuse the prepared rule executor and its existing effect dependency
graph. Introduce assignment-scoped effective-value channels before binding preparation
outputs; two physical assignments of the same Gem definition cannot share those values.
Program stage membership, stage predecessors and frozen output channels belong in injected
data bound to the exact schema/rules identities. Every known program needs an explicit
classification. Missing classification stays unresolved; it never means a delivery-stage
default. Programs with effects in different stages must be split when authored.

Stage assignment schedules work; it does not create completeness authority. The first
integration must retain the existing full owner/declaration/contributor proof. A dependency
ancestor walk, one known writer, or a Known diagnostic effect cannot close missing item,
passive or Gem-property contributors. Independently complete preparation membership under
Partial whole-owner programs would require the separate scoped-coverage decision. Do not
introduce that relaxation while implementing this scheduling boundary.

Freeze typed output channels at their declared stage. Reject later writes to earlier
consumed final stats, capabilities, contribution groups, transforms and projected inputs,
including potential writes whose current activation is false. Dependency validation must
include gates, selectors, all reduction members and transform inputs. Ordinary numerical
cycles remain invalid; type preparation retains its separately bounded native policy.

Use private attempt-bound results for intermediate exports. Reset worker scratch once at
the start, share one decreasing budget across scalar preparation, support preparation and
delivery, and clear partial state on failure. The current whole-plan executor resets on
each invocation, so repeatedly invoking it cannot carry stage values forward. Do not accept
public reports or caller-authored scalars as a substitute for these private exports.

Only retained positions instantiate delivery applications. Bound graph size by discovered
origins plus retained positions times explicit receivers/effects, without a matrix of all
origins against every possible position. Verify fresh/reused A→B→A and Rayon execution,
ordinary single-writer/cycle checks, failure cleanup, and unchanged final metric gates.

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
