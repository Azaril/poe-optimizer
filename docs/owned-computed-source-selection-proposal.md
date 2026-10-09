# Computed conditions for shared Action source selection

**Status:** Deferred investigation; no public-contract implementation authorized or required by the current source audit.
**Date:** 2026-10-09.
**Decider:** Project owner.

**Audit update:** After this question was sent, a deeper supplier/constructor
review found no normal imported-data producer for the motivating bow-inheritance
key. The immediate work is therefore an authenticated intrinsic-source closure
and the existing Fixed selector. Do not implement this extension solely to
preserve an orphan PoB branch. Retain the option for a demonstrated mechanic
requiring a computed source choice, subject to the owner's design decision.

## Context

Original05's native physical-base arithmetic now has tested required inputs, but
its Basic Attack still routes intrinsic Actor values directly. Its source-selector
inventory is empty and Partial. No owned fact currently proves that an Action
actually uses those intrinsic values rather than a replacement source. Partial
coverage blocks complete evaluation; it is not a numerical applicability test.

The current `ActionSourcePolicy` supports Fixed selection and EquipmentEligibility.
The latter tests an EquipmentUse capability after proving exact active slot
occupancy. It cannot read a computed fact belonging to the attacking Action.
That distinction matters when a skill/support/item interaction changes which
source an Action uses: two Actions can use different sources with the same
equipment selected.

For example, the pinned source can let a minion inherit the Player's bow when
a prepared skill condition and the weapon type both permit it. Current source
review finds the condition's read sites but has not proved its potential writer
domain complete. The native implementation must not preserve an apparently dead
PoB branch merely because it exists, nor declare it absent from one observed
build. Source-domain investigation remains separate from the generic contract.

## Recommended decision

Extend the existing shared source selector with a **computed Boolean fact** policy.
It reads one explicitly declared Boolean Stat on the exact current Action and
has explicit true/false outcomes naming sources from that selector's existing
source inventory. Each outcome can also express the existing known-unavailable
case. The Boolean is computed by ordinary typed rule programs and checked
contribution queries; routing gains no expression language, game formula,
Lua predicate, numeric sentinel or implicit priority.

Keep one recorded decision per exact Action/selector. All associated endpoint,
rate, critical-chance and bonus routes consume that same decision. Reuse the
current `SelectSource` effect, lazy branch reads, worker scratch and dependency
graph. A condition on an Actor or supplying Skill is explicitly projected or
combined by the Action rule; routing does not acquire arbitrary entity access.

- Known true/false selects only its declared outcome. Missing, inactive or
  unresolved condition values do not become false or select a fallback.
- A missing selected quantity remains unavailable. An unselected quantity is
  not demanded during execution; both branches remain visible to type, stage,
  cycle and work-limit validation.
- Preserve all existing named source origins. An equipment branch must resolve
  its own exact active Player slot using the current slot resolver. Do not reuse
  the single equipment handle of the older eligibility policy for other branches.
  Empty/unresolved/ambiguous occupancy cannot masquerade as an equipped source.
  Failure on the selected branch never chooses the opposite outcome; an
  unselected branch's unavailable value does not contaminate the chosen value.
  This grants no new equipment or cross-Actor read authority.
- Every route binds every reachable source explicitly and with the correct
  target scope/unit. Source inventories and rule-owner coverage remain checked.
  A computed condition does not establish complete potential-writer coverage.
  At least one outcome must name a source; two known-unavailable outcomes would
  be a redundant invalid selector under the existing reachability invariant.
- The condition is a real dependency before the routing stage. A rule depending
  on its own selected endpoint cannot produce its selector condition. Retained
  support execution must obey the same frozen-prefix constraints.

Change the current development format and rebuild affected data/identities as
needed. Do not add a compatibility branch or a second evaluator. Existing Fixed
and EquipmentEligibility policies remain useful for their actual semantics;
they are not historical format fallbacks.

## Options and trade-offs

| Option | Benefit | Cost |
| --- | --- | --- |
| Computed fact in the existing selector (recommended) | One source choice shared by all channels; reuses native rule computation and selection execution; works for combinations of skills and items | Core/Data/Engine contract, source binding and dependency validation work before real data adoption |
| Guard each numerical producer with ordinary rules first | Uses existing APIs immediately; an unsupported branch can remain unavailable | Distributes source selection across producers and routes; later complete switching needs consolidation and consistency checks |

The second option is valid for a bounded component, but it should not grow into
a parallel source-selection system. A fixed selector or unconditional Action
zero is not an alternative while source applicability remains unproved.

## Acceptance and immediate adoption

1. Validate Boolean type, exact Action scope, named-source reachability, explicit
   branch mappings and independent equipment-slot binding. Bound validation work.
2. Extend existing `owned_source_selection.rs` tests: true/false/unknown, missing
   chosen input, harmless unchosen missing input, two Actions choosing differently,
   one decision across multiple channels, wrong type, premature read, cycle,
   complete-plan refusal and fresh/reused/four-worker equality. Preserve existing
   fixed/eligibility behavior; do not copy the test harness.
3. Authenticate the actual Sniper source constructors, replacement conditions
   and admitted supplier domains. Use source03's retained selected-pass evidence;
   do not rerun it merely to prove the same observations. Investigate unknown
   writers or an apparently unreachable source branch before adopting its rules.
4. Author the intrinsic candidate and its selection condition separately. Route
   the candidate only after selection is known. Replacement conditions unsupported
   by the owned data remain unresolved; they do not select a measured default.
5. Connect the real selected-source operands to the existing physical-base rule,
   then continue its coefficient and remaining modifier domains. Preserve all
   original build queries and production coverage gaps until independently closed.

Approval of this proposal would authorize the reusable selection contract, not
declare the Sniper predicate proved, close source coverage, admit arbitrary
replacement laws or settle the separately pending numeric-domain/configuration
decisions.
