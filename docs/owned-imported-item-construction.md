# ADR: Shared imported-item construction proof

**Status:** Implemented and validated within the accepted import boundary.
Complete-source, import, publication, finite native, lint and portability checks
pass. The implementation log tracks the next selected-build blocker.
**Date:** 2026-10-01
**Scope:** Import adapter and injected conversion policy. Core and Engine remain
unchanged. This does not accept the separate skill-input, usage or readiness proposals.

## Context

The checked Fire Damage endpoint admits Original04's Ruby modifier and ordinary
quality absence. The selected item still has unresolved physical parameter,
modifier-membership and modifier-order inventories. Existing conversion profiles
require crafted-affix headers that are absent from this saved imported item.
The augment proof also rejects its identity header. Adding unrelated modifier
families cannot resolve this construction boundary.

Source construction has semantic effects. A recognized `Crafted` header sets
crafted state even when its value reads false. Range tags set advanced-copy state;
the legacy XML ModRange overlay does not. Unique ID can affect quality on some
bases and can survive parsing into a reused object. Those facts belong in the
external-format adapter's proof; they must not become fixture-specific conditions
or PoB UI state in the native evaluator.

## Decision

Add an opt-in V2 equipment-membership policy carrying the old template/base-name
inventories plus exact item-line and source-policy commitments and imported-item
construction profiles. Each profile binds a template and bounded header rules,
captures, lexical constraints and cardinalities. Reuse the existing checked
source grammar and typed value codecs. Definitions and actual item values remain
injected; production Rust contains no fixture names, source ordinals or item IDs.

One private construction proof establishes the fresh source occurrence, exact
template and policy context, admitted headers and empty augment capacity. Pass
that proof through modifier-inventory admission to physical-input conversion.
The existing V3 category census supplies ordered modifier membership; a new V4
physical-construction selector consumes the imported proof. The materialized
canonical item uses the existing Core fields and native programs.

The first source domain is fresh imported rare construction on reviewed bases
without weapon/armour socket defaults. Its header rules exclude crafted-affix,
advanced-copy, occupied-augment and unknown-setter forms. Identity metadata is
not globally whitelisted. Future domains extend the versioned policy only after
their source behavior and canonical inputs are demonstrated.

Every rule/capture and metadata interpretation must be validated against the
bound conversion artifacts. Cardinalities and work limits are checked before
allocation. Wrong bindings are policy errors; unsupported or ambiguous source
forms retain Pending obligations. XML overlays require their own bounded shape,
identity and range checks; a successful raw-text scan alone is insufficient.

V1–V3 policies retain their wire representation, identity and behavior. Omission
of the new policy preserves existing admission. Imported evidence must not
silently relax an old crafted profile. Static definition and rule coverage remain
independent of a complete concrete item's physical inventory.

## Alternatives and consequences

| Approach | Benefit | Cost or limitation |
| --- | --- | --- |
| Shared versioned proof with injected profiles | One construction interpretation feeds all three consumers; preserves the native model and historical policy semantics. | Requires explicit artifact binding, proof transport and compatibility tests. |
| Separate imported scanners in each consumer | Each consumer can evolve locally. | Repeats header and lifecycle interpretation, allowing the consumers to disagree. |
| Broaden the existing crafted profile | Smaller immediate code change. | Changes historical admission and conflates imported and crafted state that the source treats differently. |

The shared proof is cold import work. Native evaluation continues to read owned,
typed values without parsing source headers or loading Lua. No database, second
interpreter, readiness shortcut or new calculation backend is introduced.

## Delivery and validation

1. Extend complete-source evidence across all five original builds and targeted
   fresh/reused, header, state and overlay contrasts, in both JIT modes.
2. Implement the shared adapter proof and preserve historical serialization,
   admission, diagnostics and bounded-work behavior in Rust regressions.
3. Inject the first template's raw inputs and existing catalyst transport while
   retaining Partial template and program coverage.
4. Publish from the exact checked predecessor; preserve every old fact under an
   injective occurrence mapping, every saved selection and all 110 queries.
5. Rebuild byte-identically, re-finalize all five originals and record the next
   actual blocker. Closing the three Ruby inventories is not whole-build parity.

The [implementation document](implementation.md) is the authoritative progress
and resume point. The supporting initial audit is
`runs/owned-ruby-physical-next-blocker.md`; current publication commitments are in
`runs/owned-fire-damage-modifier-04/next-blocker-review.md`.
