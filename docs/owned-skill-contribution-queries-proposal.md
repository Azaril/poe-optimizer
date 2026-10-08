# Checked contribution queries for exact Skill occurrences

**Status:** Proposed; not accepted or implemented.
**Date:** 2026-10-07.
**Decider:** Project owner.
**Scope:** Extend the existing contribution graph's ownership and membership
authority so real Skill scaling producers can replace resolved test inputs.

## Why this is the next boundary

The selected Original05 chain now connects actual item preparation, the Sniper
population and intrinsic attack/accuracy. Its selected ordinary minion-damage
passives can supply the known 68% increase. Pain Offering is the next substantial
missing numerical connection: the application and final source inputs exist,
but these five required scaling channels still have no published producer:

| Stat suffix | Owner and meaning |
| --- | --- |
| `3228` | Exact supplied Skill: source buff-effect increase |
| `3229` | Exact supplied Skill: source buff-effect multiplier |
| `322a` | Exact supplied Skill: combined magnitude factor |
| `322b` | Exact recipient Actor: buff-effect-on-self increase |
| `322c` | Exact recipient Actor: buff-effect-on-self multiplier |

The source is Skill `02a2`, supplied by physical Gem `086b`; it is not every use
of that definition, its owning Player, or the physical Gem's input/property
owner. The existing application binds that exact source to each eligible Actor.
Its source and recipient values must remain independently owned.

The retained source witness observes empty incoming scaling lists in Original05:
the resulting values are 0, 1, 1, 0 and 1. Together with final Offering level 22,
these produce the real 62% application contribution. Those observations are
evidence for a bounded empty domain, not permission to install five literals.
Closing this connection would let the real application join the 68% passive
increase to reach 130%. Final physical endpoints, speed, critical hits,
mitigation and complete request coverage still need their own checked consumers.

## What exists and what does not

Core already represents exact authored/generated `SkillTarget`s, provider roots,
declared grant paths and effect occurrences. Its `ContributionQuery` partitions
one recipient's typed channel into named groups; numeric groups have explicit
order, and Boolean `Any` groups are unordered. The binder verifies every
potential matching contribution, including groups a particular consumer does
not read. Existing arithmetic can sum increases, convert percentages to factors,
multiply factors and apply explicit rounding. No new interpreter is needed.

There are deliberate authority limits:

- Data's [`owned_rules/ordered.rs`](../crates/poe-optimizer-data/src/owned_rules/ordered.rs)
  `read` rejects a query on `Current` in Skill context. `order` admits only direct
  Class, Ascendancy, PassiveNode, ItemTemplate and Modifier owners with reviewed
  Character, Allocation or equipment origins.
- Engine's [`owned_plan/compile/ordered.rs`](../crates/poe-optimizer-engine/src/owned_plan/compile/ordered.rs)
  `Sources::provider` rejects generated grant paths and other invocation origins.
  The existing numeric rank tuple does not define order between repeated Skill
  occurrences or support assignments.
- A plain numeric `Contributions(Current)` read can address Skill channels, but
  using it here would omit the checked query's explicit membership and grouping
  proof. Reading a Player query as a proxy would change the recipient.

The existing Actor query contract can express guarded recipient producers for
`322b`/`322c`. It cannot express the corresponding checked source-Skill producers
merely by adding data. That is the design decision, rather than a missing
arithmetic operation or a need for another stat system.

## Recommended decision

Extend the **existing checked contribution graph to exact Skill occurrences**.
Reuse native identities and the existing compiler, reductions and execution
plan. Add reviewed read and member-origin authority; do not create a parallel
Skill contribution pipeline or persist source-runtime objects.

1. **Exact read scope.** Admit `Current` Skill query reads only when the declared
   stat permits Skill recipients and the invocation binds an exact existing
   Skill target. Initially admit only the relative scopes needed by authored
   producers. This does not automatically authorize arbitrary related Skills,
   source-property aggregation, Actor-to-Enemy reads or a new cross-owner write.
2. **Exact member authority.** Extend membership declarations to bind the real
   definition/program/effect to its checked Skill/provider occurrence, including
   the declared supply/grant route where required. Authored copies, generated
   copies and repeated uses remain distinct. A Gem/Skill definition ID alone,
   shared ancestor, display name or first discovered occurrence is insufficient.
   A producer keeps its original read context; its destination needs independent
   authority. Player, item, support and application contributions must not become
   eligible merely because their roots fit `ProviderKey`. Admit each origin
   family only with its reviewed source-to-recipient relation. Unsupported
   families remain rejected or explicitly incomplete.
3. **Ownership before grouping.** Named groups describe owned mechanical
   domains, such as a Skill's own adjustments and explicitly applicable Actor
   adjustments, only after their membership and applicability are established.
   An Actor's contributions do not flow into every descendant automatically.
   Use checked reads or delivery relations for inherited applicability; retain
   the source's identity and avoid counting the same occurrence twice. Data
   defines which groups combine and where any rounding occurs.
4. **Order is semantic authority.** Numeric folds retain explicit ordering and
   complete membership. Do not use opaque IDs, discovery order, thread order or
   source-store depth to break ties. A new positive origin needs a reviewed order
   or a proof permitting its existing numeric fold for the admitted domain.
   Duplicate definitions are not deduplicated. Boolean `Any` keeps its existing
   unordered semantics and still requires complete, resolved active membership.
5. **Unknown stays unknown.** Preserve candidate-wide potential-contributor
   validation, stage/cycle checks, activation and competing-final-write errors.
   Partial source discovery, missing producers and ambiguous recipients cannot
   yield neutral factors. A complete empty group may yield its declared 0/1
   identity only after all relevant provider coverage is accounted for. Every
   potential matching effect must belong to a declared group even if inactive,
   identity-valued or unread by the selected consumer.
6. **Cold binding, native execution.** Resolve identities, membership and order
   once in checked planning with existing work limits. Workers consume immutable
   indices and local scratch. Plan identity includes the new checked authority;
   changing source, recipient, provider path, group or order invalidates reuse.
   Rebuild artifacts for the current contract rather than maintaining an older
   runtime interpretation.

These principles need owner approval. The exact minimal wire declarations and
admitted origin families must then be specified and tested before implementation;
this proposal does not approve arbitrary provider paths or settle all support
origin semantics. The separate
[support-origin composition proposal](owned-support-origin-composition-proposal.md)
remains open.

## Grouped multipliers and source evidence

Pinned `CalcPerform.lua:2126–2149` separately reads source buff effect, recipient
buff effect and source magnitude before application. `CalcTools.lua:16–45`
combines magnitude's increase and multiplier. `ModList.lua:164–224` and
`ModDB.lua:214–296` multiply local matching MORE factors, round that product
(or floor at a configured precision), then multiply the parent's result. Their
multi-name variants also retain separate per-name products. Flattening all
records into one generic Product can therefore change results.

Existing named queries and numeric rounding operations can express a **known
finite owned partition** and its final combination. They cannot establish that
partition's meaning or prove arbitrary parent chains equivalent. PoB's parent
stores, bucket names and traversal depth are offline evidence, not runtime
ownership concepts. The exact relevance of this rounding to game mechanics also
needs review; reproducing it for a documented parity domain is not proof of game
intent. Unproven nonempty grouping remains unsupported.

The retained `owned-minion-physical-damage-source-02` JIT reports are identical
and cover 37 cases. The observer's `scalarChannel` in
[`owned_minion_physical_damage_source.lua`](../crates/poe-optimizer-pob/tests/support/owned_minion_physical_damage_source.lua)
records flattened matching modifiers and actual scalar returns, with exact
source/recipient observations. It proves baseline absence and positive scaling
controls, but does not by itself authenticate reusable inherited group boundaries.
Source buff increase 50, recipient increase 25, combined MORE factors 1.25/1.5,
and magnitude increase 25 with MORE 20 are useful nonneutral controls. Their
custom-source origin is not proof that production custom-modifier import or
every real support family is already admitted.

## Alternatives and first delivery

| Option | Benefit | Limitation |
| --- | --- | --- |
| **Extend exact Skill reads and occurrence membership — recommended** | Establishes one reusable structural contract for real nonempty producers and repeated sources. | Needs origin, recipient and order validation plus stronger grouping evidence; positive origins can be admitted incrementally. |
| Add only complete-empty Skill query reads | Smaller initial change; can replace neutral fixture inputs with checked absence, following the attribute empty-MORE pattern. | Still changes public read authority, rejects all nonempty incoming domains and leaves the next real Skill producer blocked. It must not be presented as general Skill aggregation. |

Approve the general ownership boundary first. A first admitted **empty incoming
domain** can then be a bounded subset of that design while positive origin and
group proofs are completed. It must use real published query-backed writers,
not observed scalars, literal neutral producers or a hidden fallback.

The immediate data packet should author the five writers `3228`–`322c` and their
actual incoming queries on the existing source Skill and recipient Actor. Its
magnitude writer combines independently checked INC and MORE domains using
existing operations. Query/group memberships remain Partial wherever discovery
or semantics are unproved. A checked complete-empty group may be published only
for its explicitly admitted domain; it does not close the global query registry,
unconverted owners or the complete support inventory.

Validate the packet by extending the existing actual-item Sniper/Offering graph,
removing its supplied resolved-scaling boundary. First establish real level 22
to Offering 62%, then combine the selected passive 68% using authored consumers.
Add the noncritical physical endpoint consumer only once its addition,
conversion, multiplier-group and rounding domains have explicit coverage;
baseline endpoints alone cannot justify discarding currently zero mechanics.

## Acceptance gates

- Exact authored/generated Skill and recipient bindings; two equal definitions
  with different levels, qualities or providers cannot leak contributions. Wrong
  parent, supply slot, grant path, recipient and producer context are rejected.
- Complete-empty queries derive identities; Partial or missing inventories do
  not. An additional potential contribution, including disabled or neutral ones,
  invalidates an empty-domain claim rather than silently changing its result.
- Real nonempty admitted members exercise duplicate source occurrences and
  explicit order. Positive tests use authenticated published programs. Numeric
  type mixing, ambiguous order, duplicate membership and undeclared origins fail.
- Group-local rounding and final combination have discriminating fractional and
  negative controls, with provenance-to-owned-group evidence. Reordering input
  containers must not change the semantic result or diagnostics.
- Dependencies remain explicit: preparation/execution separation, disabled
  roots, missing scaling writers, unresolved support origins and cycles stay
  unavailable. Fresh, reused-scratch and Rayon evaluation agree exactly.
- Publication preserves all five normalized inputs and 110 queries. Retain
  source-backed application/stacking controls and scope every parity claim;
  this work alone does not produce a complete build or authorize optimizer use.
