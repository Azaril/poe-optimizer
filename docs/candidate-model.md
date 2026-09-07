# Canonical candidates and finite validation

The candidate boundary represents a build's choices independently of Lua, a process, XML,
or the evaluator implementation. `Candidate` contains class, optional ascendancy, paid
passives, equipment instances, active skill instances and their support instances. All six
search dimensions remain explicit. A finite catalog supplies allowed choices; a lock fixes
only the field the user requests. Catalog breadth and search strategy are separate concerns.

## Identity and exact choices

`CatalogIdentity` contains the candidate schema version, game, rules revision and SHA-256
fingerprint of a canonical source manifest. Every candidate carries that identity. A domain
rejects a candidate from any other catalog, even when some IDs happen to match.

Candidates use ordered sets and maps and implement `Clone`, `Eq` and `Ord`. Equivalent
insertion orders produce equal values and deterministic serialization, which supports
search deduplication. Slot IDs are stable within a catalog. An equipment value is an exact
physical instance ID, and active/support values are exact gem or granted-skill instance IDs.
Two physically distinct copies with identical stats need different instance IDs. A candidate
cannot equip the same item twice or assign the same gem instance to several groups.

`ExactPayload` preserves the complete source content, its format and its SHA-256. This keeps
rolls, variants, quality, gem configuration and other producer-resolved detail available to
a materializer without copying it into each candidate. **The catalog producer must verify
payload and manifest hashes.** The portable core checks their syntax and catalog references;
it has no hashing or source-parser dependency and does not authenticate caller-provided
content. `CandidateDomain` owns a validated catalog and exposes it immutably. Production
adapters must bind the identity to all entries and rules, not just selected build numbers.

The representation treats support assignment as a set. Producers must explicitly reject or
report order-sensitive support behavior that cannot be captured by immutable instance
configuration; they must not assume it commutes. Stable skill-group slot IDs retain group
placement, while disabled source groups are not enabled candidate assignments. The original
whole-build source remains a materializer concern until its complete projection is covered.

## Shared physical roots and effective overrides

Passive node ownership uses nonempty sets: `ClassStart.class_ids`,
`AscendancyStart.ascendancy_ids`, and `Ascendancy.ascendancy_ids`. Each definition points to
a physical start ID, and that root's owner set must point back to the same definition.
This supports several classes or ascendancies at one actual graph node without inventing
new IDs. A shared root does not relax class/ascendancy pairing or merge candidate identities.

The pinned data requires this distinction. Node **54447** belongs to Witch and Sorceress,
and **50459** belongs to Ranger and Huntress; these are the same physical IDs, not separate
nodes at coincident coordinates. `PassiveTree` assigns every listed class the same `node.id`
in [PassiveTree.lua](../vendor/path-of-building-poe2/src/Classes/PassiveTree.lua), lines
228-237. The source records are in [tree.lua](../vendor/path-of-building-poe2/src/TreeData/0_5/tree.lua)
at lines 118094 and 111082. Its shared ascendancy handling, lines 238-248 of `PassiveTree.lua`,
also assigns Lich and Abyssal Lich the same start **23710**. Switched paid nodes such as
**58751** likewise retain a physical graph identity with different effective ascendancy data.

Class and ascendancy IDs determine which automatic override is selected. Base **4739** has
a Witch override with effective ID 17306 and spell/minion stats; base **56651** has a Huntress
override with effective ID 39263 and attack stats. `PassiveSpec` selects class overrides
first, then ascendancy overrides, in
[PassiveSpec.lua](../vendor/path-of-building-poe2/src/Classes/PassiveSpec.lua), lines 1485-1493.
The candidate retains the physical allocation IDs. A producer must also preserve and verify
the effective override payload and provenance before evaluating freely mutated builds.
Ownership sets model graph membership; they do not implement these changed calculations.
Unresolved effective overrides remain explicit unsupported mechanics until data extraction
and requested-versus-realized checks cover them.

## Requirements, locks and available choices

`CandidateConstraints.required_skill_ids` preserves one or more active skill definitions
while allowing alternative configured instances of those skills. Required supporting active
skills use the same mechanism. `required_item_instance_ids` preserves one or more exact
items while allowing placement in compatible unlocked slots. Empty requirement sets remain
valid for unconstrained research cases.

Independent `CandidateLocks` can fix class, an ascendancy or explicitly no ascendancy,
allocated and unallocated passives, exact equipment placements or empty equipment slots,
active instances within groups, required/forbidden/exact support sets, and empty skill groups.
An absent ascendancy lock differs from an explicit no-ascendancy lock in JSON and Rust.
Required skills/items are not automatically placement locks. Supports can change while the
active is fixed, and supporting active skills can change while required skills remain.

The validator never repairs a candidate or relaxes a lock. A coordinated class change must
supply a compatible ascendancy and reachable passives while preserving all requirements.
If a locked passive or ascendancy makes the change impossible, validation rejects it. A
future repair operator must return an explicit failure under these same constraints rather
than quietly dropping a locked component.

## Finite validation contract

`CandidateDomain::new(catalog, constraints)` validates source identity, references, root
ownership, edge targets, instance payload metadata, support compatibility references and
obvious contradictory locks once. It derives bidirectional passive adjacency even when the
source lists an edge only from child to parent. Dangling edges are rejected at this boundary;
a producer that excludes a missing edge must retain explicit coverage evidence and report
any resulting unverified mechanic. The current PoB tree investigation remains relevant when
building a full catalog.

`CandidateDomain::validate(candidate)` returns structured violations and a separate ordered
set of unsupported mechanics. It checks:

- Class/ascendancy membership and availability restrictions supplied for items and skills.
- Connected ordinary and ascendancy allocations with independent point budgets. Class and
  ascendancy roots are implicit zero-cost nodes; candidates cannot allocate them as paid
  nodes. Ordinary paths cannot traverse foreign class or ascendancy roots. Foreign
  ascendancy nodes cannot consume another point category's budget.
- Known equipment/skill slots, item slot compatibility, exact item/gem multiplicity,
  explicitly resolved active/support compatibility, configured global support-definition
  limits, active-group count and supports per group.
- Additive resource costs attached to selected items, active skills and supports. Named
  capacities are explicit per run; a selected cost without a corresponding budget fails
  closed. Derived or conditional costs require additional resolved rules and coverage.
- Required skill definitions and exact equipped instances, all independent locks, and
  granted-skill dependencies on exact items or paid passive owners.

`is_valid_within_catalog()` means that supplied finite rules pass. `is_searchable()` also
requires no explicit unsupported mechanics. Neither method certifies complete game
legality. Global unknown mechanics and unknown mechanics attached to selected nodes, roots,
items, active skills or supports are retained. An unused item's unknown mechanic does not
block unrelated candidates. Consumers must preserve this distinction in reports and avoid
turning a passing finite contract into a universal legality claim.

## Scope still requiring extraction or richer rules

This model does not yet extract a full PoB catalog or implement arbitrary XML mutations.
Its connectivity is an ordinary rooted graph, not the complete rules for weapon-set passive
allocations, alternate starts, jewels/subgraphs, granted allocations, annoints, or refunded
travel paths. These need resolved data and specialized validation before search can use them.

Equipment interactions such as two-handed/offhand conflicts, dual-wield restrictions,
uniqueness limits, attributes, requirements granted by other items, crafting alternatives,
and inventory/trade cost are not inferred from item text. Active/support tag parsing,
triggered or granted skill ownership beyond explicit dependencies, mutually exclusive
supports, order-sensitive effects, reservation modifiers, dynamically derived resource
capacity, loadouts, spectres, minion limits and rotation feasibility are not inferred either.
A producer must represent resolved checks or mark the affected mechanics unsupported.

A narrowly allowlisted materializer may use a finite set of fully specified source builds
with exact requested-versus-realized projections. That is useful evidence for search and
evaluator integration, but does not establish free mutation or complete six-dimension game
legality. Broader catalogs and mutation operators remain implementation work; no dimension
is removed from the intended optimizer scope.

## Validation evidence

The core contract tests change all six dimensions simultaneously while preserving two
required skills and two exact items, demonstrate order-independent deduplication, reject
cross-class changes that cannot preserve locks, and check separate active/support locks.
They also exercise reverse graph edges, independent point budgets, exact-copy multiplicity,
compatibility and resource limits, ownership dependencies, unsupported-mechanic propagation,
invalid source references and distinct no-ascendancy serialization. Shared physical class and
ascendancy roots/paid nodes, empty/unknown owner sets and forward/reverse root membership are
also checked without claiming effective override calculation coverage. These fixtures specify
finite rules to test the contract; independent PoB fixtures validate source-backed behavior.
