# Flat resource reward producers

This packet adds complete numerical owner inventories for four already modeled rewards selected in Original05. It adds no definitions, parameter slots, selection recipes, reducers, receivers or metrics.

| Reward | Source control | Contribution |
|---|---|---|
| `0028` | Act 1 Freythorn / King In The Mists | Player Spirit Add 30, Stat `3166`, unit `0004` |
| `0029` | Act 1 Ogham Manor / Candlemass | Player Life Add 20, Stat `311a`, unit `3119` |
| `0030` | Act 3 Azak Bog / Ignagduk | Player Spirit Add 30, Stat `3166`, unit `0004` |
| `0063` | Interlude 3 Kriar Village / Lythara | Player Spirit Add 40, Stat `3166`, unit `0004` |

Each reward already has a Known schema with seven complete empty declaration inventories. Its numerical owner was absent. The packet adds one unconditional Actor-context program per exact Reward owner, contributing to Player through the existing flat-resource channel. The three Spirit contributions total 100 only when those three reward occurrences are present in an explicitly closed component; 100 is not a runtime default. Candlemass remains on the distinct Life-points channel, with no percentage-Life conversion.

`closure.json` contains only the four new owners. `dependencies.json` captures their prior absence and the exact Reward, Stat and Unit descriptors. `bindings.json` identifies the reviewed source controls, values, owned targets and numerical channels. `source-vectors.json` preserves exact source callback and selection observations. `authoring.json` binds this packet to `runs/owned-plain-minion-life-passives-01/package` input `ac2cab1855e111822d6c85b1ab4a674f7a65ecfbc3f9a2fee6553ec9fbb0491a` and commits each artifact's bytes.

The existing `owned_configuration_reward_source` witness provides independent source evidence. Both `runs/owned-configuration-reward-source-01/source-jit-{off,on}.json` reports are 6,246,344 bytes with SHA256 `4fa239bb7f72b936b522a84fb6adb0139cdddb2a4b593ce69a64bb722f7a2e9d`. They cover 30 complete loads and 39 finite callback cases per JIT mode. The bounded extraction includes all five originals and relevant absent, disabled and archived/selected configuration controls. It preserves complete recorded modifier lists rather than discarding other reward outputs.

Each owner is justified by an exact join: configuration option key and Boolean value, existing reward policy outcome, exact owned mapping, source QuestRewards row, original callback result, and the corresponding saved MAIN/CALCS records. The source rows at indices 3, 5, 11 and 27 each produce one untagged BASE modifier, with zero flags and keyword flags. The proof does not match by Quest source label alone. Existing input/default aliases, missing-value behavior, false-to-no-reward policy and reward selection IDs remain unchanged.

These configuration effects are distinct from the source's `useConfig=false` weapon-set-point quest records. This packet neither reimplements point grants nor treats selected numerical rewards as empty behavior.

The publication helper authenticates the prior input, source checkout pins and whole source reports, then uses the existing checked release assembler. It reverses exactly the four appended owner rows and the new provenance entry and compares the entire prior input. All existing schema/registry/routing identities, numerical bodies, import policies, preset instances and 110 queries must remain identical. No schema release rename or dependency rebinding is performed.

No full Spirit/Life result, complete incoming contribution inventory, resource availability/reservation result, or final metric is claimed. Other selected missing/Partial owners remain coverage blockers and the published release has no evaluation bundle. Native component tests must distinguish the actual contribution programs from any test-only aggregate reducer.

## Verification status

Central validation passed: four native reward tests, three existing Spirit
regressions and both authored/publication tests (nine total). The new package is
`runs/owned-flat-resource-rewards-01/package`, input
`0ab77c776cdf17d2e93d04f25d06280165a82b724604fddfa16c752fe1b42f20`.
All five original imports, selections and 110 queries are preserved without
rebinding; all 18 artifacts rebuild byte-identically. Strict workspace/all-features
and native-only Clippy and formatting pass. Receipts:
`runs/owned-flat-resource-rewards-validation-{02,03}.json` and
`runs/owned-flat-resource-rewards-01/validation.json`. Earlier failed test/helper
attempts remain recorded; no production runtime changed.

Normal authored checks use tracked packet files and the source manifest only.
Optional publication also requires the pinned checkout and both existing source
reports. The output directory must be fresh and nonexistent; choose another
suffix when reproducing the already completed command below. The source witness
was reused unchanged, not rerun. Whole-build results remain 0/5.

```powershell
cargo test --test owned_flat_resource_rewards_cli flat_resource_reward_packet_has_complete_single_effect_source_proofs
cargo test --test owned_flat_resource_rewards_native

$env:POE_OPTIMIZER_TEST_RESOURCE_REWARDS_PRIOR = 'runs/owned-plain-minion-life-passives-01/package'
$env:POE_OPTIMIZER_TEST_RESOURCE_REWARDS_OUTPUT = 'runs/owned-flat-resource-rewards-01'
cargo test --test owned_flat_resource_rewards_cli publish_flat_resource_rewards_preserving_all_five_originals -- --include-ignored --nocapture
```

The CLI gate asserts exact schema, registry, routing and import/query identities, preserves selected issue counts `113/116/108/121/11`, verifies each of these four rewards occurs exactly once in Original05's selected 17-reward ChoicePreset, and compares all 18 rebuilt release files byte for byte. Shared normalization and preservation modules are reused.
