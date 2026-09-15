# Owned import inputs for source 3887ae68

These are production inputs for the offline catalog extension and `normalize-owned`. The original component recipe, tables, and 38 IDs in the parent directory remain unchanged. The new seed adds 31 fixed reward outcomes, 30 option values, and 20 equipment slots, giving 119 allocations. The production extension reuses the six reviewed Gem/Skill mappings and appends 2,396 missing catalog identities, giving a final registry watermark of 2,515.

The appended catalog descriptors are explicitly Unmapped input schemas. They do not inherit the executable Twister/Sniper schemas or claim numerical coverage. The original nine Partial rule owners and four Partial routing entries remain Partial. The reward declarations describe fixed authored input outcomes only; their stat text is evidence, not compiled effects.

## Artifact roles

| Files | Role |
|---|---|
| `authoring.json` | Explicit source selectors, equipment relationships, policy versions, reused quality IDs, and additional file pins. This is offline authoring input. |
| `recipe-seed.json`, `mapping-seed.json` | New seed retaining all original entries, descriptors, slots, and rule bodies. The mapping includes the six exact existing Gem/Skill targets and reviewed reward/equipment targets. |
| `skill-identities.json` | Standalone `SkillIdentityData`: 966 Gems, 1,436 Skills, their declarations and all 16 missing references. No legacy game-data package is needed by its consumer. |
| `source-pin.json`, `skill-catalog-policy.json` | Exact source revision with 29 LF-hashed `src/...` file paths, and explicit absent-support/materialization policies. |
| `allocations.json`, `reward-source-facts.json`, `provenance.json` | Tail allocation ledger, reviewed 17-rule reward metadata, and reproducible artifact/source hashes. |
| `*-policy-seed.json` | Policies bound specifically to the seed schema/mapping. They are not valid against the compiled successor. |
| `compiled/` | Production CLI output: immutable extended recipe, registry, schema, rules, routing, mapping, roles, manifest, and transition receipt. |
| `policies/` | Normalization, reward, item-line, and item-source policies explicitly bound to the compiled successor. `transition.json` records the checked dependency change. |
| `queries/original-01.json` through `original-05.json` | All 22 requested query identities per original, in their original order. Only query identity and actor category were projected; no measured result was consumed. |

Twister's exact external Gem ID is `Metadata/Items/Gem/SkillGemTwister`, distinct from its catalog key with `Gems`. The two Sniper actor-skill mappings use the actual `MinionMeleeBow` and `GasShotSkeletonSniperMinion` identities. No display-name fallback or missing command replacement is supplied. The two missing Sniper command references remain in the catalog and the existing Partial declarations.

The source pin uses `src/Classes/SkillsTab.lua` with its LF-normalized hash. It does not retain the former test fixture's unrooted alias and CRLF hash. Item source policy uses the same real pin; its empty layouts do not grant facts or range attribution.

## Reproduce the offline seed

From the repository root, run this with a new output directory:

```text
python scripts/export-owned-import-data.py --snapshot crates/poe-optimizer-data/data/game-data.json --base-recipe data/owned/poe2/3887ae68/recipe.json --base-ids data/owned/poe2/3887ae68/ids.json --mechanics-manifest data/owned/poe2/3887ae68/source-manifest.json --mechanics-facts data/owned/poe2/3887ae68/mechanics-facts.json --reward-facts crates/poe-optimizer-import/tests/fixtures/owned-quest-rewards-v1.json --query-manifest tests/fixtures/breadth-expectations/originals-v1.json --authoring data/owned/poe2/3887ae68/import/authoring.json --source-root vendor/path-of-building-poe2 --output-dir runs/my-import-seed
```

The exporter projects the injected structured identity catalog unchanged, verifies the source file pins, and emits reviewed finite policy data. It does not execute Lua, evaluate source programs, parse stat text into effects, or inspect original-build results. Only this optional offline step reads the legacy snapshot and vendor source files. Its output directory is reserved exclusively. An I/O failure leaves an explicitly reported incomplete new directory; it does not overwrite an existing directory or claim atomic runtime publication.

The seed recipe's canonical schema identity is `d61c98e791687ccd879ab763bf91b1ec4264a6be747b8c25f54f4f77c07d5667`. Production Rust assembly validated it before extension. For a newly exported seed, substitute its paths below and choose another new output directory:

```text
poe-optimizer extend-owned-skill-catalog data/owned/poe2/3887ae68/import/recipe-seed.json --catalog data/owned/poe2/3887ae68/import/skill-identities.json --mapping data/owned/poe2/3887ae68/import/mapping-seed.json --source data/owned/poe2/3887ae68/import/source-pin.json --policy data/owned/poe2/3887ae68/import/skill-catalog-policy.json --output runs/my-import-compiled
```

The extension validates the seed, appends only missing identities, preserves prior descriptors and mappings, and explicitly creates new schema/rule/routing bindings. It does not silently repair stale runtime artifacts. Its published successor schema is `be845a5de89f06eba928c4c285678cbc177b0ff4293a9a69528138db1d50f87b` and mapping identity is `3794c5e01ddcf7495fb130bfc6524e0fa256149bccec3e7b5f4da89efd24a3e7`.

To reproduce the final policy files, add `--compiled runs/my-import-compiled` to the exporter command and use another new output directory. Before changing a policy binding, that mode checks the compiled publication hashes, the exact seed/successor identities, the preserved 119 registry entries, all 105 seed definitions and 14 slots, previous source mappings, and unchanged rule/routing content. Only then does it emit policy copies bound to the successor. Runtime loaders continue to reject stale bindings.

To verify the checked-in seed and final policies without writing, use the same exporter inputs with:

```text
--compiled data/owned/poe2/3887ae68/import/compiled --check-dir data/owned/poe2/3887ae68/import
```

The Python regression suite is:

```text
python scripts/tests/test_export_owned_import_data.py -v
```

It checks exact reproduction, ID/descriptor preservation, external selector spelling, canonical bindings, explicit reward defaults, equipment aliases, quality absence/zero conventions, stale transition rejection, source hashes, shared bounds, and independence from reference numerical payloads. The production Rust constructors remain the semantic validation gate.

## Normalize with the persisted artifacts

Supply the compiled bundle and final policies explicitly; no game-data or source checkout is loaded:

```text
poe-optimizer normalize-owned tests/fixtures/builds/breadth-20260908/build-02.xml --registry data/owned/poe2/3887ae68/import/compiled/registry.json --definitions data/owned/poe2/3887ae68/import/compiled/schema.json --mapping data/owned/poe2/3887ae68/import/compiled/mapping.json --roles data/owned/poe2/3887ae68/import/compiled/roles.json --policy data/owned/poe2/3887ae68/import/policies/normalization.json --rewards data/owned/poe2/3887ae68/import/policies/rewards.json --items data/owned/poe2/3887ae68/import/policies/items.json --item-source data/owned/poe2/3887ae68/import/policies/item-source.json --queries data/owned/poe2/3887ae68/import/queries/original-02.json --output runs/my-original-02
```

The coordinated all-five production CLI checks and retained breadth tests preserve 478 physical Gems, 140 authored Skills, 338 Supports, 81 selected reward occurrences, 448 explicit zero-quality amounts, and all 110 ordered query rows. Separate source presets and equipment/loadout relationships remain independent. The same standard quality ID and percentage-points unit from the original recipe are reused.

Item-line and layout policies are explicitly empty, as in the preceding breadth coverage. Unknown item semantics, classes, point pools, metrics, generated providers, and other unresolved facts stay Pending. Selected-minion query templates have no fabricated actor; projection-ledger handling of recorded unavailable rows remains separate. Every original still produces a partial draft. Full-original native evaluation remains **0/5**.
