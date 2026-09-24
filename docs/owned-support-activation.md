# Support receiving, applicability and activation

Status: proposed; owner review pending before changing shared rule/routing contracts.
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
state and authored target. Before the gem-input checkpoint, all 478 gem parameter collections
and 140 skill scopes remained Pending. The generic [gem-input proof](owned-gem-inputs.md) now closes twelve intrinsic
parameter collections; 466 remain Pending. Only two Gem definitions have Known schemas,
covering those twelve instances. The injected [skill-scope policy](owned-skill-scopes.md)
now admits the missing parent slot on all 140 authored skills as Shared; enabled state,
global effects and generated providers remain independent. Support integration
removes a universal engine limitation; it cannot by itself complete any original build.

## Proposed contract

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

## Support interaction evidence and pending parity policy

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

**Owner decision pending:** preserve pinned PoB behavior by default with the quirk isolated
in an explicit versioned preparation policy, or use a corrected order-independent semantic
resolution and report a parity difference. The full-parity objective favors preserving the
reference behavior unless the owner authorizes the deviation. Neither behavior is adopted
in native code yet. The separate origin/receiver and false/unknown contracts above still
apply. Do not silently bake a sparse Lua table into the owned data model or silently claim
that corrected closure is exact parity. Any compatibility policy must remain native and
injected, with no source UI/runtime dependency.

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
findings refine the pending owner decision; no native selection, multiplicity or reference
quirk policy has been adopted. All original-build and whole-plan gates remain unchanged.

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