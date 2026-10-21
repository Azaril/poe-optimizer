# Reverse-primary rebuild audit — 2026-10-21

This is a read-only artifact audit plus the separately authorized external Rust test constructor edits. No Cargo, source execution, network, Git mutation, JSON mutation, or public API workaround was performed. The census implementation and actual regeneration belong to the root/compiler owner. No generated output or completed-build claim is implied.

## Checked starting point

- Main checkpoint: `32d48986992c252938231269b8df5a56e8ca4d69`.
- Current published package: `runs/owned-generated-enabled-publication-01/package`.
- Current input: `348f0c7588e915f7b7ea6c2de9b1d2ce5089ebdb4393f5822ed818537a4c7560`.
- Its endpoint: `runs/owned-generated-enabled-publication-01/endpoint.json`, 65,672,791 bytes, SHA-256 `ee20e4b2255de4b2ba092c7127a8816e821836e47e44839d43f6d743695d5dba`.
- Its release receipt: 57,925 bytes, SHA-256 `860994818079984781b93c742a16351819601be0693aa89796049cec7ba244bf`.
- Its validation: SHA-256 `ce23983ca89dbdb744c8b7013d0c8220e10f8a87a2ae5669a3d3497e0f5d3a28`.
- Current role identity: `2c50f2cdbb205112f338c8da05dd6c568e2fe65ca6cdc44eb0859fd5649e744d`; raw `roles.json` SHA-256 `d97e06756726fdc9956f5b15cd58488930b3cea497756db078d7a2f0dd2ea85f`.
- Five selected issue counts are `[123, 137, 127, 144, 5]`; 110 queries in five sets; 17 payload files plus `release.json`; no complete native original build.

The endpoint is only 1,436,073 bytes below the existing 64 MiB limit. Measure the actual census and endpoint sizes before publication; do not silently change the limit.

## Minimal authoring route

Reuse the exact production compiler and the existing private checked dependency rebinder. A schema revision or numerical migration is inappropriate: these APIs require a real schema/rule change and correctly reject a no-op. Do not add a descriptor merely to pass that gate, relax it, or expose the rebinder publicly.

1. Authenticate the old complete package/endpoint and original compiled/current seed artifacts as retained JSON bytes and receipts. The new required field means old role bytes cannot be directly deserialized into the new runtime type. This is a one-off authoring inverse, not a runtime compatibility reader.
2. Recompile from the five existing maintained inputs using `owned_catalog_recipe::extend_owned_catalog_recipe`: `import/{recipe-seed.json,mapping-seed.json,skill-identities.json,source-pin.json,skill-catalog-policy.json}`. The compiler already calls full catalog reproduction. Require all existing role rows, receipt claims, mapping, recipe, registry, definitions, slots, rules and routing to survive exactly; the only role JSON addition is the required top-level `reverse_primary`.
3. Rebuild the maintained tree seed through the existing `transition_owned_bundle` → `compile_owned_tree_catalog_extension` → `transition_owned_catalog_with_tree` sequence. Keep `CatalogItemPolicyMode::RebindPrior` and the actual tree policy installation. Require 18 of its 20 files to remain byte-identical, with only `roles.json` and `transition.json` changed.
4. For the current published endpoint, install the compiler-derived census only after proving exact Gem/Skill IDs and old construction rows match. Keep all existing native/catalog and Import facts. Convert this completed current wire object to `OwnedReleaseInput`, then call crate-private `owned_release_revision::rebind_release_dependencies` in a temporary internal Rust authoring test. Assert the exact changed digest-leaf set below and a complete inverse. Add one explicit authoring provenance entry.
5. Call `assemble_owned_release`, then independently `verify_owned_skill_catalog_reproduction` against the complete pinned catalog. Repeat assembly and compare the complete artifact/receipt set byte-for-byte. Require all native artifacts, mapping, items, item source, rewards and five query files unchanged.
6. Run fresh CLI normalization/replay for all five originals against the new package and compare to authenticated saved prior outputs. The old package cannot be newly loaded by the new required-field runtime. Preserve full drafts, allocators, origins, selected requests and pending/completion state; only exact Import role/policy commitments may rebind. A census is not permission to resolve generated skills yet.

The most relevant previous **Rust** authoring sources are:

- `runs/owned_source_construction_cutover.rs`: exact raw-endpoint authentication, one-off wire insertion, private dependency rebinder, assembly/reproduction, provenance and whole inverse. Its prior input/endpoint/counts are obsolete: adapt to the explicit current pins above, and replace per-row construction insertion with this single census insertion.
- `runs/owned_construction_seed_rebuild.rs`: production rebuilding of the 20-file maintained tree seed. Its old transition SHA must be replaced with the authenticated current SHA below.
- `runs/owned_construction_fixture_rebuild.rs`: useful inventory only; **do not repeat** its old row-construction/direct-input migrations for this top-level addition.

The similarly named `runs/owned-source-role-migration-01/author.rs` and maintained `owned_source_role_migration` helper concern item parser source roles. They are unrelated to this Gem census.

Root can temporarily register the adapted authorer as an internal ignored Rust test, run it serially, retain exact executed source plus receipt, and remove the temporary registration. No new CLI publisher or compatibility layer is needed.

## Maintained seed files and pin closure

Only two maintained JSON artifacts contain a complete `OwnedSkillRolePackageInput`:

| Path below `data/owned/poe2/3887ae68` | Old SHA-256 | Bytes |
| --- | --- | ---: |
| `import/compiled/roles.json` | `7a2191cdc79594cf25955a1204aadd7287766fafa323ac48c662cd553f6e0d1f` | 1232706 |
| `import/compiled/transition.json` | `5ca97989545ed0ef5441d580dca09d9f6bf9dbe39531ee9f59f2b14b780093a0` | 5918 |
| `current/roles.json` | `611713865bbf4ab237dd132ee79c232c6e81eb76c35dbf2e998233121874e04b` | 1232706 |
| `current/transition.json` | `a84e9c57648d6d8108d3291120d0b84e649632d4ae597e63a14aaae818233a7e` | 4502 |

Additional direct receipt references:

- `import/policies/transition.json:/compiled_transition_sha256` points to the compiled transition above. Its old raw SHA is `c0e958785176c256794547ce400e4980f5e24b9ef178bf43e981b8ff7c87c4db`.
- `allocations/source-facts.json:/bundle_transition_sha256` points to the current transition above. Its old raw SHA is `e92aecb56d7674a11f15c3e6df28ffdb618829b63c94332df2116d473d741c23`. Root clarified that the protected scope is allocation code/tests, not this direct receipt reference. A checked digest-only candidate is appropriate; no allocation rules or tests change.
- No further maintained `data`/`tests`/`crates` JSON/Rust literal consumers of these two raw receipt hashes were found. The two original roles identities occur in their transition receipts; no additional live exact-hash consumers were found. Keep immutable historical evidence unchanged.

The three row-only construction snapshots (`generated-preset-inputs-v1/dependencies.json`, `ice-nova-count-inputs/dependencies.json`, `warrior-generated-correspondence/dependencies.json`) contain unchanged `OwnedGemRoleRow`s, not the top-level role package. They need no JSON/authoring pin rewrite. Their isolated runtime test packages must explicitly declare an unavailable census, since a handful of copied rows is not a full catalog.

The ignored, unexecuted Rust helper `runs/owned_reverse_primary_receipt_rebind.rs` exposes `rebind_receipts(root: &Path, output: &Path, compiled_candidate: &Path, current_candidate: &Path) -> serde_json::Value`. Call only after production compilation/reproduction. It authenticates the old role/transition bytes, checks the candidate whole role/transition inverses and new typed role identity, then replaces only these two 64-character receipt references with exact whole-byte inverses. It writes fresh `before/`, `files/`, and `report.json` beneath the supplied fresh output; it does not install data. It rereads every captured input before/after output and records its own source hash. No code was executed beyond scoped rustfmt parsing.

## Current published dependency closure

`rebind_release_dependencies` already rebuilds these in dependency order: role index → support/payload → direct inputs and direct support → generated inputs → skill inventory → usage → Gem inventory → configuration dispositions → tree normalization. Registry/schema/mapping stay identical for this Import-only change.

The current package has 22 role/direct/scalar/usage digest leaves. These **19 normalization leaves** should change; add `/tree/normalization` for 20 dependent changes total:

```text
/normalization/direct_skill_inputs/roles
/normalization/direct_skill_inputs/dispositions/0/reference_action/roles
/normalization/direct_skill_inputs/dispositions/1/reference_action/roles
/normalization/direct_support_targets/direct_inputs
/normalization/generated_skill_inputs/roles
/normalization/gem_inventory/roles
/normalization/gem_inventory/usage_inputs
/normalization/gem_inventory/primary_dispositions/0/reference_action/roles
/normalization/gem_inventory/primary_dispositions/1/reference_action/roles
/normalization/gem_inventory/primary_dispositions/2/reference_action/roles
/normalization/gem_inventory/primary_dispositions/3/reference_action/roles
/normalization/gem_inventory/primary_dispositions/4/reference_action/roles
/normalization/gem_inventory/primary_dispositions/5/reference_action/roles
/normalization/usage_inputs/roles
/normalization/support_origin_order/roles
/normalization/payload_inventory/roles
/normalization/skill_inventory/roles
/normalization/skill_inventory/direct_inputs
/normalization/configuration_dispositions/dependencies/usage_inputs
```

The three `scalar_inputs` leaves in Gem inventory, usage and configuration dependencies must remain unchanged. The generator must measure and assert the actual exact set rather than accepting a broad recursive digest substitution. `roles.json`, `normalization.json`, `tree-normalization.json`, release receipt/endpoint and the explicit provenance record change. All native artifact files and query files remain exact.

## Existing executable entry points and validation targets

The maintained compiler command is:

```text
poe-optimizer extend-owned-skill-catalog data/owned/poe2/3887ae68/import/recipe-seed.json --mapping data/owned/poe2/3887ae68/import/mapping-seed.json --catalog data/owned/poe2/3887ae68/import/skill-identities.json --source data/owned/poe2/3887ae68/import/source-pin.json --policy data/owned/poe2/3887ae68/import/skill-catalog-policy.json --output <fresh-output>
```

`tests/owned_tree_cli.rs` provides the existing `extend-owned-tree-catalog` and 20-file seed reproduction, with prior setup in `tests/support/owned_bundle_cli.rs`. Use those same APIs in the authorer, rather than a new production command.

After regeneration, useful serial checks are Import targets `owned_skill_catalog`, `owned_skill_catalog_extension`, `owned_catalog_recipe`, `owned_normalize`, `owned_normalize_tree`, `owned_reference_projection`; root CLI targets `owned_catalog_cli`, `owned_tree_cli`, `owned_normalize_cli`, `owned_source_construction_cli`. Tests using the isolated source-action/item/evaluation/Ice Nova/scenario fixtures should also compile through the normal root target set.

`tests/owned_source_construction_cli.rs` and its preservation helper contain an explicitly ignored historical cutover with old input/counts. Its ordinary substituted-role refusal remains useful and reproduces current compiler output. Reuse its preservation pattern for the new current transition, not its old hardcoded input/counts. `owned_release_migration_preservation.rs` expects typed current endpoints; it cannot directly deserialize the old missing-field package. Keep old bytes authenticated as evidence in the one-off inverse.

## Authorized external test changes completed

Each directly authored synthetic package now contains `reverse_primary: OwnedReversePrimaryCensus::Unavailable`:

- `crates/poe-optimizer-import/tests/owned_normalize.rs`
- `crates/poe-optimizer-import/tests/owned_normalize_tree.rs`
- `crates/poe-optimizer-import/tests/owned_reference_projection.rs`
- `crates/poe-optimizer-import/tests/support/owned_item_normalization.rs`
- `crates/poe-optimizer-import/tests/support/source_actions_fixture.rs`
- `tests/owned_normalize_cli.rs`
- `tests/support/owned_evaluation_release_fixture.rs`
- `tests/support/owned_ice_nova_count_inputs.rs`
- `tests/support/owned_scenario_stage_inputs.rs`

`crates/poe-optimizer-import/tests/owned_skill_catalog_extension.rs` propagates `result.reverse_primary.clone()` from the actual compiler and compares the second compilation census to the first. All ten edited test files passed scoped rustfmt; no Cargo was run. Compiler-owner `tests/owned_skill_catalog.rs` and the internal unit-test constructor in `src/owned_normalize/gem_inventory/dispositions.rs` remain outside this edit scope.
