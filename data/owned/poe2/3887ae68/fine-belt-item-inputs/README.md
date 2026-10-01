# Fine Belt physical inputs

This profile adds six typed item parameters for Fine Belt `1e84` and an opt-in saved member census of two implicit modifiers and one explicit modifier. It starts from the exact checked `runs/owned-fine-belt-modifiers-02/package` input `9b9f25511f05f2963c5f46cd0cea0b921ab2c1335407a95b0045453183dbc074`.

The new slots are rarity `31a7`, fresh corruption state `31a8`, raw LevelReq override `31a9`, empty augment socket capacity `31aa`, catalyst selection `31ab` and catalyst amount `31ac`. The raw LevelReq slot remains `OptionalOnce`; this first construction proof requires exactly one explicit valid header. The other five slots are `RequiredOnce`. Actual Original05 Item27/source590 supplies Rare, false, 62, 0, None and 20 respectively. The catalyst transport program is the existing equipment-use transport with only its two declared input bindings changed.

`membership.json` injects the exact 2+1 category counts into `PobFreshOrdinaryMemberCensusV3`. The publication preserves all prior singleton and paired template rows. The new proof requires a complete attributed layout, retained single emissions with complete roll inputs, exact category ordinals and source identities, and the existing fresh empty-augment proof. Its private ordered identities determine the canonical order; source text position alone is not authority. The three existing modifiers and all 73 rolls remain unchanged: CharmLimit has 25, FlaskChargesGenerated has 24 and Life has 24.

`FreshRareSavedCategoryCensusV3` also binds the exact `observed-charm-slots` rule and `amount` capture as a derived observation. That header does not create a parameter and never supplies raw socket capacity. A missing socket header with independently proved zero capacity uses `AbsentSocketHeader` evidence. Explicit `Sockets: S` plus `Rune: None` uses the actual header and capacity one. The authored Charm Slots observation can be absent or contain another valid nonnegative integer without replacing the separately derived CharmLimit semantics. Duplicate or malformed observations withhold physical input completion.

The complete original-source witness is `crates/poe-optimizer-pob/tests/owned_fine_belt_source.rs`, with both JIT-mode outputs under `runs/owned-fine-belt-source-01`. `authoring.json` pins its immutable source revision, manifest and individual source files. These observations distinguish physical member arrays from locally consumed CharmLimit, derived charm capacity and later delivery. Item-level and quality absence were already proved by the predecessor and are unchanged here. No source implementation is copied into native calculations.

The new opt-in profile emits sidecar version 15. Existing V1/V2 and omitted-policy behavior remain separate. Exact checked schema, item-line and source-policy commitments are required; stale commitments and cross-owner slot bindings are rejected.

Reproduce the data publication and real preservation checks from the repository root with an explicit fresh output directory:

```powershell
$env:POE_OPTIMIZER_TEST_FINE_BELT_ITEM_INPUTS_PRIOR = 'runs/owned-fine-belt-modifiers-02/package'
$env:POE_OPTIMIZER_TEST_FINE_BELT_ITEM_INPUTS_OUTPUT = 'runs/owned-fine-belt-item-inputs-01'
cargo test --locked -p poe-optimizer-cli --test owned_fine_belt_item_inputs -- --include-ignored
```

The default and explicit real publication tests passed (2/2). The checked output is `runs/owned-fine-belt-item-inputs-01/package`, input `a1931151bd06b9950816b052ef7915f7f6fbe89db2b2503d579f9989407bbfe8`, schema content `240af94f13e52c8dba6aa8544178c6569fe130d95e8df229a987b6188169f599`, registry `8fa34d32023e411393579360d17c23b026550c5b998bb0e4141b51c3c418a07e`. Its 18 files total 59,808,922 bytes and retain all 31 prior provenance entries plus this explicit authoring entry.

`validation.json` records whole predecessor restoration, byte-identical package rebuild, all five unchanged saved selections, all 110 queries preserved byte-for-byte, and exact source/draft correspondence. Selected issue counts are 127/128/120/158/29. Only the three Fine Belt physical-list obligations retire, from Original05's prior 32; four saved receiving uses and one selected use remain intact. The three removed issue allocations shift later local IDs, so the test checks injective identity correspondence and every surviving relationship. All three modifier records and 73 rolls are preserved.

The real test also passes 34 source/header probes, six stale-commitment rejections and two cross-owner rejection checks. These results concern publication and canonical inputs. No native or CI result is claimed here.

Template parameter declarations, static memberships, rule owners, numerical contributors, derived charm capacity, final requirements and final metrics retain their existing incomplete coverage. No original build is claimed numerically complete, and no readiness behavior changes.

The normalization target passes 127 tests, including 14 new census/raw-input tests. Three release-policy tests exercise V1/V2/V3 stale bindings, successors and schema revisions. All four Fine Belt native component tests pass; the added physical-input case uses actual typed assignments and the unchanged catalyst transport program, then checks parallel plans, reused scratch and missing inputs. The finite native fixture does not provide a final character aggregate or close release coverage. Strict workspace Clippy, compiled dependency-boundary checks, both WASM configurations and formatting checks pass. Full workspace runtime tests were not repeated; CI is not claimed green. Detailed commands and profiles are recorded in `runs/owned-fine-belt-item-inputs-01/execution-receipt.json`.
