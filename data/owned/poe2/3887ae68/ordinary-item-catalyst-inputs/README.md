# Ordinary item catalyst inputs

This adds the existing catalyst input contract to Rope Cuffs (`2007`) and Tattered Robe (`238c`). It supplies a missing input dependency of the flat-Life `3100` programs. It does not close either item's overall parameter inventory, template rules, aggregate contributions, or final Life calculation.

The checked predecessor is `runs/owned-item-modifier-membership-02/package`, input `288592e1cf04639d14adde8db0a37f91ecdc1437bf2ade94a307c83f4a46c6ec`. The publication contains four new slots:

| Template | Catalyst selection | Catalyst amount |
| --- | --- | --- |
| Rope Cuffs `2007` | `311b` | `311c` |
| Tattered Robe `238c` | `311d` | `311e` |

Both slots are required item parameters. Selection uses the existing finite Options `09eb`–`09f8`; amount uses percentage-points unit `0002` and the existing computational bounds. Each template receives one `catalyst-inputs` program that transports those values to its exact EquipmentUse stats `0a19` and `0a1a`. All previous programs, declarations, members, and Partial closure evidence remain intact.

The shared `Catalyst` and `CatalystQuality` header rules gain two exact template bindings each (338 to 340), without grammar or codec changes. The existing source-default rows gain independently guarded assignments: absent kind becomes None `09eb`; absent amount becomes the source helper's fallback20. This does not mean the source stored20. Explicit0 stays0, selected-kind-only uses20, and amount-only is inert. Malformed, duplicate, unknown, or unproven source context does not gain absence authority. Existing item-level and ordinary-quality facts are unchanged.

`authoring.json` pins the complete source manifest and the exact Item, ItemsTab, Build, ItemTools, ModParser, body, and glove files. The optional Rust witness `owned_simple_item_catalysts.rs` executes unchanged source methods and the authenticated `getCatalystScalar` upvalue. Both JIT modes agreed: two unchanged original build loads in total, 30 fresh controls and two reparse/fresh contrasts per mode. Actual original05 Life17/16 lines have no source life tag; this data does not infer tags from English text or the saved affix catalogue.

The shared Rust authoring helper first uses the existing checked recipe extension and membership transition with unchanged source interpretation. It then explicitly authors the new item/source commitments, preserves the singleton proof's domain and allowlist, and binds unchanged tree content. The complete endpoint is validated again. A supplied successor never silently repairs stale singleton commitments. The authoring digest includes the extension, bindings, default assignments, dependencies, and source evidence.

Validation published `runs/owned-simple-item-catalyst-inputs-01/package`:

- Input `21d2bf11de0819d9d86eb5040b59684d145d431b8d3384d24e7b27ced0349efa`.
- Schema `b1ce01d1af27aaa68de7bc6668e2a3efe5063f20de2abf708e6d6d5b531f719d`.
- Registry `e96ccaa232e0c2d5016a29f73777c9acc23c927256033f9100ecc6dc179d1193`, last key `311e`.
- 18 files, 58,584,671 bytes, 18 provenance entries; rebuild byte-identical and predecessor unchanged.
- All five original saved selections and all 110 queries preserved. Exactly four Known catalyst assignments and their source-default evidence were added. Local instance IDs, allocator watermarks, issue links, other input values, and singleton modifier order remained unchanged across fresh imports (only import lineage is canonicalized for comparison).
- Selected issue counts remain `[317,322,314,380,144]`. The two overall item parameter issues remain Pending. Complete numerical build coverage remains **0/5**.
- 20 actual-base header probes and four stale-binding/changed-semantics rejection checks passed. Detailed reports are in `validation.json` and the five selected-report files beside the package.

Run the default data check:

```powershell
cargo test --locked --test owned_simple_item_catalyst_cli
```

Reproduce the explicit full publication in a new, nonexistent output directory:

```powershell
$env:CARGO_PROFILE_TEST_OPT_LEVEL = '2'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG_ASSERTIONS = 'true'
$env:CARGO_PROFILE_TEST_OVERFLOW_CHECKS = 'true'
$env:POE_OPTIMIZER_TEST_SIMPLE_CATALYST_PRIOR = 'runs/owned-item-modifier-membership-02/package'
$env:POE_OPTIMIZER_TEST_SIMPLE_CATALYST_OUTPUT = 'runs/owned-simple-item-catalyst-inputs-reproduction'
cargo test --locked --test owned_simple_item_catalyst_cli -- --ignored --exact real_catalyst_inputs_preserve_all_saved_requests_and_partial_coverage --nocapture
```

The test invokes `assemble-owned-release` for publication and rebuild, then normalizes and finalizes all five original saved selections. Its complete predecessor-restoration check permits only the four slots, two producers, explicit header/default patches, exact dependency bindings, and one appended provenance entry. New tests are Rust. Runtime source parsing, Lua, and fixture-specific numerical dispatch are not introduced.
