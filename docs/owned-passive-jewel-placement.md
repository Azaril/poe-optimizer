# ADR: Ordinary passive-jewel placement

**Status:** Implemented within the existing import boundary. Complete-source,
whole-corpus publication and finite native receiving tests pass.
**Date:** 2026-10-01
**Scope:** Import and injected data. Core's existing equipment destination and
allocation models remain unchanged. This does not accept the separate skill-input,
skill-usage or preparation-readiness proposals.

## Context

The checked Ruby input package completes one item's physical inventories, but
the item still has no resolved receiving destination or loadout scope. The four
originals that use passive jewels have 15 selected assignments across 12 ordinary
nodes and six jewel bases. The fifth original has no equipment blockers.

PoB retains saved socket assignments separately from effective eligibility.
Loading a saved assignment does not prove that its node is allocated, its item
exists uniquely, or its item type is valid for that socket. Later calculations
also apply mechanics such as jewel limits and radius effects. Recognizing
placement must not silently claim those mechanics are implemented.

Core already expresses `PassiveSocket { allocation, slot }`. Native provider
availability follows the exact receiving use to its allocation. The missing
parts are injected socket definitions/memberships and a checked source-to-
occurrence join. Reconstructing an allocation from a node number would lose the
owning Spec and conflate repeated occurrences of the same node.

## Decision

Add an opt-in, versioned ordinary shared Spec-socket policy. Keep the existing
V1 explicit-empty policy's representation and behavior. The new pass runs after
tree allocation conversion and joins each source assignment to one physical
item, one receiving equipment use and one already converted allocation in that
same Spec. It never creates another allocation or changes saved selections.
The allocation must already have independently proved ordinary access as well
as Shared scope, and its preset's character must have Known class and ascendancy
fields. A parsed node token alone is insufficient: the source loader can reject
invalid character fields before it loads any allocations. The occupied-placement
lane also requires every direct Spec to use the checked tree version: an unknown
sibling version can stop the enclosing source loader before the target exists.

Inject node-token/owned-node/socket bindings and reviewed template eligibility.
Each socket definition has its exact passive-node owner, Passive kind and the
source-proved scope domain. Append corresponding node socket and item destination
memberships while retaining all wider Partial closures. The initial proposed
domain is ordinary shared placement; weapon-specific or special socket behavior
must be demonstrated before it can broaden admission.

An independent bounded item-base proof establishes constructor framing and
source base identity without requiring conversion of every modifier. Its base
names and exact header rules are injected. It reuses the bound source base-name
inventory to reject competing bases and unsupported control syntax. A Known
template from an unrelated line alone is insufficient. Unique and radius jewels
may have proved ordinary placement while their effects remain unimplemented.

Bind the policy to the definitions, mapping/source, item-line/source policies,
equipment policy and checked tree **content**. The complete tree package already
binds normalization, so using its whole identity inside normalization would be
circular. A domain-separated content digest supplies the independent commitment;
the existing package binding still checks all surrounding dependencies. Checked
schema/catalog revisions must validate the old commitments before rebinding.

Unproved source shapes retain Pending facts. A duplicate node assignment,
ambiguous item ID, absent allocation, foreign Spec, unsupported scope or socket
kind cannot borrow another occurrence's proof. Complete Spec equipment membership
requires accounting for every direct assignment, independently of whether each
item's mechanics are complete. RuneSlot rows and item-contained sockets are
separate source domains.

## Alternatives and consequences

| Approach | Benefit | Cost or limitation |
| --- | --- | --- |
| Exact post-allocation join with injected socket data | Reuses canonical identities, native ancestry and existing data seams. | Requires source eligibility, binding and per-Spec membership proofs. |
| Fill destination from the saved node number | Small immediate adapter change. | Can bind another Spec's allocation and lacks owned socket/eligibility authority. |
| Add a separate jewel runtime/model | Could emulate the source loader directly. | Duplicates topology and moves external-format behavior into evaluation. |

This is cold import work. Reuse the checked equipment inventory and memoize the
independent base proof by exact source Item occurrence across Specs. Reuse the
immutable Items-container census and the compiled rule indexes as well. Joins remain
separate and bounded; caching a physical proof does not merge receiving uses.
Native search remains independent of XML, Lua and UI
state, with immutable definitions and worker-owned evaluation scratch. The change
does not require editing the protected allocation engine or its tests.

## Acceptance

1. Authenticate complete original loaders, selections, allocations and calculation
   consumers in both JIT modes for all five originals. Keep saved assignment,
   slot eligibility and actual modifier delivery distinct.
2. Exercise the eligible base/socket cross-product and controls for missing,
   duplicate, malformed, unallocated, wrong-kind, cross-Spec, weapon-set and
   reused-object cases. Admit only the demonstrated source domain.
3. Validate injected declarations, stale dependencies, bounded work, checked
   revisions and historical V1/omitted-policy behavior in Rust.
4. Publish from the exact Ruby input predecessor, compare every old canonical
   fact and source link, preserve all selections and 110 queries, and rebuild
   byte-identically. Measure actual selected issue changes in all five builds.
5. Use an unpublished finite native fixture to exercise the published receiving
   relation, allocation ancestry, repeated uses, scope isolation and scratch
   reuse. Retain the real package's unresolved mechanics and coverage.

The [implementation log](implementation.md) remains the authoritative resume
point. Initial dependency evidence is `runs/owned-ruby-equipment-next-blocker.md`.
