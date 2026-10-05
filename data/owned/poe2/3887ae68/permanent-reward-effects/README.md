# Permanent reward contributions

This packet completes eight existing Reward owners selected by Original05. It
adds eight ordinary programs with twelve numerical effects, two Actor resistance
Stats, and no new evaluator operation, input parameter, reducer or receiver.

| Rewards | Complete effects | Owned Player contribution |
| --- | --- | --- |
| `0031`, `0048` | Fire resistance +10, +5 | Add to new Stat `32e6`, percentage points `0002` |
| `002a`, `0052` | Lightning resistance +10, +5 | Add to new Stat `32e7`, percentage points `0002` |
| `003e` | 5% increased Mana | Increase to existing Mana `29f9`, unit `0002` |
| `0053` | 5% increased Life | Increase to existing Life `311a`, unit `0002` |
| `0041` | 30% increased global Armour, Evasion and Energy Shield | Three Increase contributions to `29f2`, `29f1`, `29f0` |
| `005a` | 15% increased global Armour, Evasion and Energy Shield | Three Increase contributions to the same separate channels |

Increase's percentage-point unit is independent of a Stat's base quantity unit.
The 5% Life effect therefore shares the Life stat with Candlemass's flat 20 while
remaining a distinct contribution kind. The source's Global tags justify Actor
contributions; these do not modify individual items' local defense values.
Each three-effect defense reward has one complete owner program. All effects
retain the actual selected Reward occurrence as their source.

The Fire and Lightning channels are separate from Cold resistance `09d4`,
all-elemental resistance `09d5`, item-local prepared magnitudes and final metrics.
No cap, override, resource-pool formula or full defense calculation is supplied.

## Evidence and publication

Authoring binds the exact prior package `runs/owned-flat-resource-rewards-01/package`,
input `0ab77c776cdf17d2e93d04f25d06280165a82b724604fddfa16c752fe1b42f20`.
The reviewed allocation extends registry `32e5` by exactly two Stats, through
`32e7`. Existing Reward schemas have complete empty declaration inventories;
their numerical owners were absent. The successor uses the existing catalog and
tree-policy transition, preserving every prior descriptor and rule and rebinding
only authenticated dependency identities. No source-selection policy changes.

`bindings.json` names the source option/value, parsed effects and owned channels.
`dependencies.json` captures the exact prior definitions and missing owners;
`closure.json` contains the two new Stats and eight new owners. `source-vectors.json`
preserves exact configuration census, callback and full-load observations, joined
through the existing input/default mappings and typed reward policy. Shared Quest
labels alone are insufficient identity; Boolean controls and exact option strings
are checked independently. All three Global-tagged records are retained for each
defense reward.

Both independent reports under `runs/owned-configuration-reward-source-01/` are
6,246,344 bytes with SHA256
`4fa239bb7f72b936b522a84fb6adb0139cdddb2a4b593ce69a64bb722f7a2e9d`.
They cover 30 complete loads and 39 finite original callback cases per JIT mode.
Optional publication authenticates each report and the pinned source files.
The source runtime is reused as evidence, not called by native evaluation.

The helper reverses exactly the two schema additions, eight owners, registry
extension and dependency rebinding and compares the previous release. All five
original inputs and 110 queries must survive. Fresh imports have new lineages and
derived draft digests; preservation compares their exact occurrence correspondence.
Published package reconstruction must be byte-identical across all 18 files.

## Verification

Seven native tests, both authored checks and the exact publication gate pass.
The publication preserves all five original inputs/110 queries and reconstructs
all eighteen artifacts byte-identically. Central receipts are
`runs/owned-permanent-reward-effects-validation-{02,04}.json`; strict workspace
and native-only lint plus supplemental source-stability validation also pass.
Normal authored checks only need tracked data and the source manifest. The ignored publication gate additionally
requires the exact prior package and pinned source reports. Choose a fresh output
directory when reproducing publication.

```powershell
cargo test --test owned_permanent_reward_effects_native
cargo test --test owned_flat_resource_rewards_cli permanent_reward_packet_has_complete_twelve_effect_source_proofs

$env:POE_OPTIMIZER_TEST_PERMANENT_REWARDS_PRIOR = 'runs/owned-flat-resource-rewards-01/package'
$env:POE_OPTIMIZER_TEST_PERMANENT_REWARDS_OUTPUT = 'runs/owned-permanent-reward-effects-01'
cargo test --test owned_flat_resource_rewards_cli publish_permanent_reward_effects_preserving_all_five_originals -- --include-ignored --nocapture
```

The remaining selected reward families (ailment threshold, flask Life recovery
and Charm charges/capacity), other selected Partial owners, and final metric
consumers remain separate obligations. No whole-build result or complete incoming
contribution inventory is claimed by these source producers.
