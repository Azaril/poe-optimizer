# Socket occurrence identity and projection

**Status: proposed for owner review, 2026-10-06.** This refines the accepted
[socket configuration separation](owned-socket-configurations.md); it does not
reopen the choice to separate rolls, configurations and physical inventory.

## Decision

Recommend deterministic structural identities for evaluated socket contents,
derived from the host equipment-use identity, configuration identity, declared
socket slot and saved socket-selection identity. Allocate configuration and
selection IDs during authoring or Import. Preparation derives occurrence keys without
allocating new document IDs or storing a second editable child-use inventory.
This is an identity and migration change, not a different evaluation backend.

These keys are stable symbolic addresses usable in saved selectors, not ephemeral
plan indices or hashes manufactured as authored IDs. A shared equipment-address
type would serve Core binding, exact selectors, inventory claims and the existing
Engine plan. Validated keys may then be interned to compact plan-local indices.

| Approach | Benefit | Cost |
| --- | --- | --- |
| Derived socket occurrence keys (recommended) | One authoritative configuration; reuse across hosts has distinct receiving identities; candidate composition needs no child-ID allocation or persisted projection synchronization. | Broader changes now to provider identities, exact selectors, inventory bindings, serialization and preparation. |
| Stored child equipment-use IDs as a checked projection | Smaller initial change because current consumers already accept those IDs. | Every configuration/preset edit must maintain and validate a second set of records; search must manage their allocation and replacement. Later removal would require another identity migration. |

Both approaches can be correct and deterministic. The recommendation favors the
requested end-state structure over the smallest first patch. No performance
speedup is claimed without measurement; the structural benefit is avoiding
unnecessary mutable projection state in candidate creation.

## Current constraint

`ProviderRoot::EquipmentUse`, `ProviderRoot::ItemModifier`, equipment-owned
choices and the Engine's `ConcreteEntity::EquipmentUse` currently refer to
`ItemSlotUseId`. `EquipmentDestination::ItemSocket` connects stored child and
host uses. It does not retain a configuration for an unused item. Meanwhile,
project composition explicitly allocates no occurrence and advances no revision.
Generating and persisting extra child IDs during composition would violate that
contract. Existing generated skill/actor keys provide a structural-identity
precedent, but they are not an implementation of this equipment extension.

## Proposed invariants

- Direct equipment uses retain their existing authored identity. Socket content
  receives a distinct typed structural identity rooted in that direct use and
  its selected configuration. Exact public type names are implementation details.
- Reusing one configuration on two host uses yields distinct receiving keys.
  Repeated identical rune descriptors have different saved selection IDs.
- An explicit empty slot creates no child occurrence. Missing or unknown content
  remains unresolved. An unused item retains its configuration without a fake
  equipment use or physical copy.
- Replacing a socket selection changes its identity. Editing rolls under the
  existing same-occurrence policy need not change that selection identity, but
  content bindings invalidate prepared plans. Locks and exact queries are never
  silently retargeted after replacement.
- Moving a selection to another declared socket changes its receiving key.
  Selecting the same earlier saved configuration restores its symbolic addresses,
  with content and availability checked again. Presets select the root use and
  configuration; they need not enumerate derived children independently.
- Preparation validates the exact host/configuration/slot relation and compiles
  the receiving graph once. Workers use immutable prepared keys and private
  scratch; neither source text nor mutable UI state participates.
- Physical inventory correspondence remains separate. A structural key does not
  prove stock availability, permission to replace a rune, or the mechanic's
  numerical completeness.

This first contract covers the accepted host plus ordered socket selections.
Do not silently flatten nested configurations into this representation; any
required nesting needs explicit representability and bounded expansion rules.

## Migration and acceptance

Version the affected document, draft and provider/plan identities explicitly.
Preserve direct-use IDs. Existing child-use references may migrate only when a
complete, unambiguous configuration correspondence is proved; record the exact
old-to-new map and update all dependent selectors, choices and bindings together.
Retire old child-use rows from the canonical document, retaining migration
correspondence as diagnostics. Never recycle their allocated IDs; preserve the
allocator watermark.
Partial or conflicting evidence remains unresolved. Historical packages remain
reference artifacts, not an alternate live runtime mode.

Start with the actual selected Crown/Leggings configurations, each containing
three explicit empty slots, and contrasting occupied/unused hosts from the other
originals. Test round-trip, deterministic re-import, same configuration on two
hosts, alternative presets, repeated runes, replacement invalidation, unknown
contents and no allocation during composition. Preserve all five original
requests and their 110 query identities except any explicitly reviewed identity
migration, which must retain exact correspondence and query meaning.

Socket layout mapping, local rune-to-host delivery, grouped rounding and physical
modification feasibility retain their separate implementation gates. Empty
configuration support cannot certify occupied rune mechanics. Parameter-only
declaration closure and precise stale modifier-obligation cleanup can continue
independently while this public identity choice is reviewed.
