# Effect applications

**Status:** General model accepted by the project owner; native contract implemented
and tested. The first Partial Pain Offering data integration is published and
validated; final input/activation producers and the complete damage consumer
remain unfinished.
**Date:** 2026-10-02
**Decision:** Reusable source, recipient, activation and stacking model, rather than
an Offering-specific model. This approval does not approve the separate raw-input,
usage-preset or preparation-readiness proposals.

## Problem demonstrated by the original build

The fresh Original05 observer records ten passive increases totaling 68 and a
separate Pain Offering increase 62 at effective skill level 22. The original
noncritical physical calculation reads their combined increase factor 2.3 and a
Gigantic factor 1.2, returning 574/1068 before later hit factors. This is observed
modifier membership, not an inference from those endpoints. Fresh reference
validation covers modes, disabled sources, duplicate copies, source and recipient
scaling, added damage, conversion and gain. Its receipt and remaining integration
gates belong in the [implementation log](implementation.md).

Pain Offering is not an ordinary owner-granted passive modifier. Pinned PoB
`CalcPerform.lua` scales each active buff for its recipient, then `mergeBuff`
selects the highest value of each matching modifier within the same buff family.
Two copies cannot simply contribute twice. Recipient BuffEffectOnSelf,
source BuffEffect/Magnitude, combat mode, source enablement and effect eligibility
are independent inputs. Damage and speed are separate modifier entries.

The existing player-granted minion channel 1d33 and exact actor receiver 1d34 are
appropriate for the passive increases. Their contract explicitly does not define
Offering or general aura delivery. The preceding dynamic reductions offered
Sum/Product; adding Maximum alone would have lacked exact source/recipient and
activation semantics. Payload links describe container/payload Skills, not buff applications.
Feeding 62 as an external scalar would hide the missing producer and would not
advance unchanged-build parity.

## Accepted direction

Introduce a general, typed **effect application** boundary. Game data describes
the effect family, producer, eligible recipients, activation, strength calculation
and stacking policy. Native planning binds exact occurrence identities once;
evaluation executes the resulting typed graph with worker-owned scratch.

An application contains or binds:

- The exact supplying Skill/Actor/provider occurrence and declared effect output.
  A second use of the same Skill definition remains a distinct source.
- The exact recipient occurrence. Owner, minion, ally, enemy and environment
  relationships require explicit applicability declarations; no global broadcast
  or definition-name lookup is implied.
- Typed source and recipient inputs for strength, scaling and activation.
  Source enabled state is not automatically encounter uptime or successful use.
- A declared semantic stacking family and modifier identity, independent of PoB
  display names or Lua flags. Scaling occurs at the required source/recipient
  stage before the declared family reduction. The first required behavior is
  per-modifier highest applicable value, not highest skill level or sum of copies.
- Traceable winning/contributing source occurrences and deterministic tie rules.
  Missing candidates or unknown applicability cannot prove a complete maximum.

Application discovery, enabled membership, contribution completeness and output
readiness remain separate. Inactive applications must not demand irrelevant
strength inputs; unknown activation cannot become false or an empty identity.
Discovery must not depend on which output the UI requests. Bound all source,
recipient and application counts before parallel execution; do not build an
unrestricted source-by-actor Cartesian product.

Reuse the existing typed rule graph, dependency validation, units, declarations,
canonical identities and failure cleanup. Source and recipient read authority must
be explicit, following the exact-origin principle already used by supports.
Do not overload SupportAssignment or PayloadLink to obtain that authority.
Keep PoB adapters outside Core/Engine; no Lua object, parser, database or display
string becomes a runtime effect identity.

## Native contract and compatibility

Operations V15 opts into an explicit application registry. V14 remains the
default operations alias; the reviewed Pain Offering publication explicitly uses
V15. Earlier packages omit the
new field and retain their serialized bytes and plan identities; an omitted
registry under V15 cannot establish that no effects exist.

Each registry row binds a Skill definition or owned actor slot to a finite list
of declared recipients. Discovery supplies exact occurrence identities. Its
ordinary typed rule program reads the recipient through Current/Actor and the
source through a separate, read-only EffectSource scope. Source parameter and
choice reads refer only to inputs already admitted for that exact Skill; they do
not authorize raw import fields or implicit reads from its supplying Gem.

A Boolean activation node guards every contribution, preserving any existing
effect condition. An inactive application does not demand its strength inputs.
Unknown activation or incomplete source/recipient inventories remain coverage
gaps. Contributions are grouped by exact recipient, declared effect family and
modifier identity. Maximum selects among the scaled candidate values, including
negative values; a proved empty group emits no contribution. Equal winners retain
their source provenance. Data validation requires compatible stat, contribution
kind, unit and value type across every shared group.

Applications participate in the existing dependency graph and bounded work
accounting. Staged evaluation explicitly classifies application IDs and their
shared reduction groups; no fabricated owner or implicit execution stage is
permitted. This extends the existing typed evaluator rather than introducing a
second interpreter. Planning is reusable; numerical execution uses worker-owned
scratch and remains suitable for Rayon and WebAssembly.

The first version supplies Skill and owned-actor sources, and Player, Enemy or
declared owned-actor recipients. It supports numeric Maximum stacking. Broader
target selectors, other stacking policies and additional source kinds require
their own explicit contracts; the generic name does not confer those semantics.
Game coefficients, family identities and applicability remain injected data.

## Alternatives

| Approach | Benefit | Cost |
| --- | --- | --- |
| **General typed applications — accepted** | Direct path to offerings, buffs, debuffs and auras; preserves source identity, recipient scaling and stacking as reusable concepts. | Larger initial contract and integration tests before the first Offering works. |
| Dedicated Offering application model | Smaller initial implementation for this real blocker while still modeling occurrences and non-stacking correctly. | Likely migration when another buff needs the same relationships; risks repeated delivery logic. |
| Aggregate bonus on the player or action | Small formula change. | Rejected: loses actual producers, recipient scaling, duplicate handling and activation; it cannot establish parity. |

## First implementation and validation

First implement Pain Offering's actual damage contribution to the declared
ordinary minion recipient domain, together with the shared application contract.
Its strength comes from injected level/stat data and real producer inputs. Keep
speed, duration, range, spike survival, reservation and additional recipient
families explicitly incomplete until implemented; partial delivery is not complete
Skill coverage. Standard/alternate quality and supported final level retain their
own producer chains.

Reference controls must include disabled/removed Offering, different levels,
duplicate copies with equal and unequal strengths, reversed source order,
recipient/source effect scaling, combat/unbuffed modes and duplicate minion
occurrences. Observe actual parsed inputs, selected source modifiers and final
damage while preserving functions, source state and output across both JIT modes.

The first game-data publication includes reference controls with two Sniper
recipients of different quality selected independently in MAIN/CALCS. Exact
loaded occurrence/actor correspondence proves recipient isolation and local
quality, independently of the duplicate-Offering controls for source stacking.

Native tests must exercise source occurrence → application → recipient → damage,
including wrong recipients, missing/inactive/unknown producers, duplicate
stacking, independent occurrences, bounded planning, scratch reuse and Rayon
workers. Preserve the five original selections and 110 queries. The first complete
component does not close the real raw-input, usage or readiness inventories.

This decision is independent of the now-accepted raw SkillUse input, usage-preset
and preparation-readiness contracts. Implement their explicit joins where
Offering depends on them; contract acceptance does not supply missing final-input
producers. The later requested-participation proposal remains a separate pending
decision. The [current implementation plan](implementation.md) controls that work.

## Historical first publication and integration gates

The receipt and issue counts in this section describe the first application
publication. Later publications preserve this component and advance its input
and topology dependencies; use the current implementation plan for the baseline.

The checked first endpoint is `runs/owned-pain-offering-01/package`, input
`922ddfc5b0767d1a7938df807fe0680cfeb661cb84844dccae394ed30f6d32ec`:
18 files/60,136,032 bytes, 52 provenance records, thirteen new definitions
`3221`–`322d` and an explicit Partial V15 application registry. The default alias
and predecessor migration contracts remain V14. No evaluation/stage bundle is
included; a later staged evaluator must classify every application explicitly.

Gem `086b` now has authored supply/grant declarations and a pre-support input
program. Its previously Unmapped Skill `02a2` is Known with Partial declarations.
The injected forty-row level table and scaling/Maximum program target the exact
Sniper actor slot. The global minion-level/preparation channels `30ab`–`30ad`
are bound through the existing physical-Gem compiler. That path uses physical
level, quality and corruption and needs no Direct SkillUse raw-input extension.

The fresh source witness passed with 37 cases/38 loads per JIT mode and identical
19,612,949-byte canonical evidence, SHA256
`030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935`.
It validates the full finite table, distinct recipients, duplicate sources,
parsed source/recipient scaling and exact rounding. Eight native tests passed,
including an authenticated twelve-case/fifteen-candidate application join;
two publication tests verify rebuilding and original-data preservation.

Remaining work is explicit:

1. Produce final supported level/quality `3225/3226`, effect activation `3227`,
   and resolved source/recipient scaling `3228`–`322c` from complete, admitted
   facts. Required generated parameters `3223/3224` retain their ordinary gates.
   Integrate the accepted usage and preparation-readiness contracts, preserving
   the actual Prolonged Duration support assignment. Requested participation and
   action-duration transfer to application lifetime retain their separate gates.
2. Connect received buff Damage INC `322d` to the physical-damage consumer with
   authenticated passive increases, quality, Gigantic and intrinsic attack
   inputs. Preserve applicability and source-store rounding boundaries; no
   measured aggregate becomes a production input.
3. Complete the relevant coverage/stage declarations and validate the resulting
   actual physical range. Re-finalize all five unchanged selections and their
   110 queries to identify the next blocker. Broader Offering mechanics and
   recipient families remain Partial until separately implemented.

At that historical publication, selected issue counts were **116 / 116 / 108 / 121 / 19**, all requests
Pending/not run and complete native whole-build coverage **0/5**. The passing
native application fixture supplies labelled final/scaling/activation boundaries;
it proves the authored component, not unchanged Original05 evaluation. Detailed
checkpoint receipts and resume work remain in the [implementation log](implementation.md).
