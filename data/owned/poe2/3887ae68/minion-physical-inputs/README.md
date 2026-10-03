# Minion physical Gem inputs

This authored family admits reviewed physical scalar inputs for Skeletal Arsonist, Skeletal Frost Mage and Skeletal Reaver. It uses the existing V4 Gem schema migration and shared normalization codecs. The ordinary catalog compiler still rejects their unresolved additional Command references; this explicit refinement preserves that missing knowledge.

| Existing Gem | Known primary | Corruption flag slot | Corruption delta slot |
| --- | --- | --- | --- |
| 0914 Arsonist | 0323 | 325b | 325c |
| 0917 Frost Mage | 0326 | 325d | 325e |
| 0918 Reaver | 0327 | 325f | 3260 |

All IDs use the `def.000000000000` prefix in the `poe2/owned-mechanics-v1` namespace. Only these six parameter slots are allocated. Both slots on each Gem are required and belong to the physical Gem parameter site. There is no new calculation program or runtime operation.

## Publication

`migration.json` is a complete `GemSchemaMigrationInput` bound to the checked `owned-skill-usage-inputs-02` predecessor recorded in `authoring.json`. It promotes exactly three Unmapped Gem descriptors, preserving all other definitions, registry assignments, rules, routing and prior content. The three Known primary memberships remain Partial, as do parameters, quality-kind membership and every unreviewed provider port. The unresolved Reaver reference is spelled `CommandSkeletalReaversPlayer` in the source catalog.

`inputs.json` is a predecessor-bound `GemInputPolicy` **template containing only the three new rows**, not a replacement for the existing complete policy. After checked V4 migration, append these rows to the migrated normalization and use its new definitions identity. Recompute `gem_inventory_scalar_inputs_identity` and explicitly set both the inherited Gem inventory policy and Pain Offering usage policy to that new scalar-input commitment before checked normalization publication. Their rows and catalog commitments are preserved. Explicit replacement does not silently repair stale bindings.

`bindings.json` records the authenticated physical identities, existing primary IDs and allocated slots. Its Sniper entry is a preservation control, not another migration subject. Sniper's existing schema, level envelope 1..65535, quality closure, slots 30aa/30b1, supply and missing-Command gaps stay unchanged.

## Scalar contract

- New physical levels use 1..40, the contiguous source primary and first-stat-set table domain. Natural maximum 20 is a separate source fallback fact. Normalization preserves parsed u16 values without clamping; 0/41 can remain Known draft inputs but fail later schema binding. Fractional, missing and unparseable levels remain Pending.
- Quality uses the existing Optional quality kind 0006 and its nonnegative 0..1,000,000 quantity schema. The new Gems' allowed-kind inventories remain Partial. Missing/malformed quality is not an invented zero.
- The corruption flag accepts exact `true`, `false`, and the observed literal `nil` as false. Missing and unknown strings remain Pending.
- Corruption level delta reuses the shared finite scientific-number codec in Count unit 295a, with only explicit `nil` mapped to 0. It remains independent of both the flag and physical level. Missing, malformed and non-finite values remain unresolved.

The full finite delta range is a direct-storage contract justified by the source `tonumber` assignment and the existing shared codec. The finite witness does not exhaustively measure all floating-point values or establish gameplay legality. Source fallback behavior for malformed inputs does not become an owned default.

## Source evidence and compact vectors

The optional `owned_minion_physical_gem_inputs` witness loaded the complete pinned original build with original source methods, then observed 35 fresh cases for each of four Gems, including Sniper as a preservation control. Each JIT lane contains 128 successful LoadSkill/repeated ProcessSocketGroup cases and 12 expected missing/unparseable-level failures. Full original output, actor output, saved Gems, source catalog and independent MAIN/CALCS selections were preserved by the witness.

`authoring.json` records exact byte hashes for the two full evidence files in `runs/owned-minion-physical-gem-inputs-02`. The witness omits wall-clock timing, so the reports are identical and reproducible. Publication authenticates both complete reports and their exact projection into the compact vectors. It also pins the source manifest, reviewed source files, source catalog, original XML and all authored JSON artifacts. These are authoring/test commitments; runtime imports do not load ignored `runs/` evidence files or the reference vectors. The earlier `-01` reports retain historical timing and remain separate evidence.

`reference-vectors.json` is copied from the passing JIT-off evidence. Its reproduction is structural, with no inferred values:

1. Copy top-level source hash and evidence XML/catalog identities; use the authenticated manifest revision.
2. Copy `additional_observation.catalog` in full, retaining both level-key arrays and missing-Command observations.
3. For each `additional_observation.load_cases` row in source order, copy existing `id`, `label`, `attributes`, `ok`, `physical_gems`, `published_groups`, and `error` fields.
4. If `after` or `reprocessed` exists, copy only its existing `level`, `quality`, `corrupted`, and `corrupt_level` properties. Missing fields stay absent.

Ordinary tests can use this compact authenticated measurement without Lua or a local package. Optional full-evidence validation proves this exact projection for both JIT lanes. The vectors are source input observations, not native whole-build or numerical parity.

## Remaining gates

Partial physical parameter and quality-kind inventories remain open. Missing Command Skills are not manufactured. This family does not establish minion actor/action supply, population, reservation, minion selection, final level/quality producers or complete native builds. The prior Pain Offering usage policy and Sniper's existing supply remain intact.

Corpus validation should find 15 newly admitted physical occurrences across Originals01 and05, with 30 known scalar assignments and their existing Pending inventories. Preserve selected Original01 physical levels 17/12/19 and distinct unselected Original05 alternatives, including literal nil corruption. Reassess the actual selected blockers after publication; schema admission alone is not whole-build progress.
