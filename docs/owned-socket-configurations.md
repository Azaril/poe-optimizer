# Proposed owned socket configurations

**Status: proposed; awaiting the user's design choice.** This model is neither accepted nor implemented. The current checkpoint only makes unmaterialized rune membership explicit and prepares source descriptions. No Core wire or evaluator authority changes follow from this document.

## Observed gap

`ItemRecord` is a rolled specification occurrence, not a physical inventory copy. Its fields contain a template, parameters, quality, level and modifiers; they contain no socket contents. `InventoryItem { id, item }` establishes stock supply, but likewise stores no contents. `EquipmentDestination::ItemSocket { container, slot }` attaches a child **use** to a host **use**. Structural validation checks references and use-containment cycles; it does not establish a persistent item configuration or require two uses of one item record to have the same child setup.

A socketed weapon in an unused source Item entry has no host equipment use. We can preserve its rune headers as source evidence, but the present owned model cannot retain their host/content relationship independently of an equipped use. Composition currently retains item records directly referenced by selected equipment uses, without a separate socket configuration closure. Availability checks copy-to-record correspondence and reports overlapping copy uses; it has no copy-level socket contents.

The earlier [augment-binding proposal](owned-augment-binding.md) assumed that existing `ItemRecord` and per-use `ItemSocket` edges alone could preserve augment ownership. That assumption is incomplete. Its authored layout/capacity, schema binding, provenance and reconciliation requirements still apply, but persistent configuration and physical supply need separate treatment.

## Recommended separation

| Layer | Meaning | Does not establish |
| --- | --- | --- |
| `ItemRecord` | Rolled descriptor/specification, reusable by multiple configurations and uses | Physical copy supply or socket contents |
| `SocketConfiguration` | An authored ordered socket setup for one host record, retained even when unused | Ownership of the host/children or permission to modify owned stock |
| `EquipmentUse` | One receiving occurrence selecting the host record and a configuration | A second independent definition of its children |
| `InventoryItem` and configuration evidence | Exact physical copies and their observed current setup | Automatic legality, zero-cost modification or availability of a hypothetical setup |

Different hypothetical configurations may reuse the same host `ItemRecord`. Different saved presets can select those configurations. Changing a rune setup need not clone unchanged weapon rolls or falsely create another physical weapon. Numerical evaluation remains possible with unspecified supply; stock constraints and modification feasibility are separately bound facts.

The alternative under discussion is to make each configured item a distinct configured-record occurrence. That is valid if configuration is explicitly part of that record's identity and copy binding can express transformations between configured records. It should not quietly reinterpret every existing `ItemRecordId` as a unique physical copy.

## Minimal proposed authored records

Illustrative names, not a finalized API:

```rust
SocketConfiguration {
    id: SocketConfigurationId,
    host: ItemRecordId,
    sockets: Vec<ConfiguredSocket>, // authored order; includes explicit empty slots
}
ConfiguredSocket {
    slot: SocketSlotDefId,
    content: Option<SocketedSelection>,
}
SocketedSelection {
    id: SocketedSelectionId,
    item: ItemRecordId,
}
```

The configuration table belongs in owned project/build/inventory documents as required by each document's self-contained references. An unused imported item can retain its configuration without an `EquipmentUse` or an invented `InventoryItem`. Multiple alternative configurations can reference the same descriptor. Source correspondence identifies what was imported; the resulting configuration is owned runtime data.

The separate socketed-selection identity distinguishes two selections of an identical rolled descriptor and makes replacement explicit. It is **not** a physical-copy ID. Editing the same selected item's rolls or quality follows the existing same-occurrence policy. Replacing the socketed supply allocates a new selection identity; exact queries/locks are not silently retargeted. The exact ID API must be settled with existing edit/provider identity contracts before implementation.

A draft uses explicit Pending fields and collection closure. `Known(None)` means an empty slot; an absent configuration or unresolved content does not. A complete ordered socket list establishes its authored slot count, including empty slots. This count comes from explicit owned structure, not the number of child equipment uses. Do not also store an independently authoritative count that can disagree. Intrinsic maximum capacity and any mechanic that changes usable capacity remain separately declared data/rules. Import requires an explicit layout mapping before source socket positions become typed slot IDs.

The concrete requirement is a host plus ordered socketed item selections. Do not introduce an arbitrary recursive object graph merely to solve these rune cases. If supported content requires configurations nested inside socketed items, add an explicit child-configuration relation and bounded cycle/depth/expansion checks; never flatten such legacy/source data into this flat shape. That representability decision must be made before declaring those inputs complete.

## Use projection

An equipped host selects exactly one configuration whose `host` equals its item record. The native preparation boundary validates the configuration and constructs occurrence-specific child provider paths. Conceptually their key includes the exact root equipment-use identity and socketed-selection identity; the same configuration reused by two host uses yields distinct effect/activation paths.

Keep one authored source of socket contents. Existing child `EquipmentUse` records may be retained temporarily as a checked projection, but every edge must match the selected configuration; they cannot independently select different rune content. Missing, extra, duplicate or mismatched projection edges are unresolved/invalid as appropriate. A published or deserialized preview DTO cannot construct a bound projection.

Projection must not depend on an Import sidecar or PoB text. It is deterministic from owned configuration, host use, validated definitions and relevant selection/scope. If prepared keys replace authored child-use IDs, introduce that identity change explicitly. Do not silently allocate IDs in a supposedly allocation-free composition operation. If the compatibility step retains authored IDs, allocate them in authoring/import and validate them during composition.

Composition retains the selected host/configuration and all referenced child descriptors. It must preserve independent presets and unused project configurations without including unrelated stash data in a numerical plan digest. Whether presets explicitly list projected child uses or composition selects an already-authored projection closure is an API choice that must be documented and tested; it cannot be accidental auto-completion of missing inputs.

## Physical supply and changes

A configuration expresses a desired setup. An inventory copy can have separately recorded, complete or partial evidence of its **current** setup. That evidence must name actual child `InventoryItemId` values where known, bound to the relevant socketed selections/slots. Descriptor references alone cannot establish that two copies of the same rune exist.

Binding a hypothetical configuration to inventory therefore checks two independent things: which exact copies supply the host/children, and whether the desired setup matches or requires changes to observed stock. Do not mutate the inventory snapshot during numerical evaluation. Removal/replacement restrictions, destructive changes, costs, budget, trade availability and user locks belong in explicit feasibility/edit policies. Unspecified or partial stock evidence is unknown, not free modification. Reusing a physical copy in mutually exclusive alternatives is distinguishable from simultaneous reuse.

Lock targets need to distinguish host rolled values, a selected configuration, a particular socketed selection, an empty slot and physical-copy availability. Configuration edits invalidate exact content bindings and prepared plans while retaining the identities allowed by the existing same-occurrence/replacement policy.

## Validation requirements

- Typed identity/domain, allocator watermark, namespace and revision checks; all host/child/configuration references self-contained; bounded collections and work.
- No duplicate slot in a configuration; preserve authored ordering and explicit empty/unknown distinction. Every slot must be declared by the host template with the correct kind and scope; the child template must permit the destination.
- Exact host/configuration correspondence and complete per-use projection. Reuse across alternatives does not imply equal configurations, while reuse of one configuration cannot acquire contradictory child sets.
- Exact definition-package and configuration content bindings before preparation. Unknown schema membership remains Pending; it is never an inferred empty list or a wildcard.
- Independent inventory-copy correspondence, current-configuration evidence, multiplicity/overlap and modification feasibility. These checks do not invent copies or erase hypothetical numerical results.
- Unused stash weapon retains its setup; two hypothetical rune setups share unchanged weapon rolls; repeated identical rune descriptors retain distinct selection/provider identities; the same setup reused by two host uses has separate local effects.
- Saved rune lines reconcile with reconstructed configuration-derived effects exactly once. Source names, line positions and saved display values remain provenance rather than runtime authority.

## Compatibility and migration

This would be a material owned-model/API change, with a versioned document/codec migration and updates to project composition, drafts/finalization, inventory binding, authoring edits, provider identities and plan digests. It should be discussed and accepted before dependent materialization work starts.

Existing per-use socket edges cannot always be lifted to a unique persistent configuration. A migration may group an explicitly complete, consistent host-use/child set into a configuration, retaining correspondence to its old identities. Conflicting setups for the same descriptor should become explicit alternative configurations when their membership is proved. Absent, partial or inconsistent edges—including unused items whose contents exist only in source text—remain unresolved; absence must not synthesize an empty configuration.

Older source imports can be re-normalized through reviewed header/catalog/layout conversion. The sidecar records provenance and migration correspondence, but it never supplies a hidden ownership edge at runtime. Keep the existing RuneLifecycle and semantic collection gaps until configuration, occurrence projection, reconciliation, activation and effect coverage are proved at their respective boundaries.
