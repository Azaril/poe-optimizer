# Support-source domains for complete numeric rewards

This packet adds explicit support-source certificates for 18 already-complete
numeric Reward owners. All 17 rewards selected by unchanged Original05 are among
them. This removes their missing support-capability declarations from native
composition; it does not complete that build or retire an imported-input issue.

The accepted [composed discovery model](../../../../../docs/owned-support-origin-composition-proposal.md)
requires a reviewed capability domain for each selected owner. These rewards
have known schemas with seven complete empty declaration sets, complete nonempty
program inventories, no reads, literal numeric nodes and unconditional numeric
contributions to Player. Their existing closed owned semantics admit no extra,
linked or additional-effect support origin. The packet explicitly declares
`AuthoredAssignmentsOnly` through the existing recipe-extension API. Runtime
code does not infer this from a reward label or an empty observed PoB query.

`dependencies.json` freezes all 31 Reward definitions and 19 Reward rule owners,
plus operations V25. Publication checks the exact predecessor and every frozen
definition/body. Eighteen owners satisfy the reviewed restriction; one Partial
owner and 12 definitions without reviewed owners remain unknown. The extension
changes no schema, formula, table, receiver, import policy or runtime contract.
An exact inverse check permits only support-domain rows and provenance to change.

## Validation and reproduction

Ordinary Rust tests check the admission restriction and reject Partial rules,
conditional effects and incomplete grant declarations. A native replay removes
Reward support domains and becomes unavailable, then restores two selected real
packet rows and reproduces the original component report. The fixture's reward
schemas and bodies must equal the frozen publication dependencies. An Unmapped
replacement blocks again. The replay remains explicitly finite component data.

```powershell
cargo test --locked -p poe-optimizer-cli --test owned_reward_support_domains
$env:POE_OPTIMIZER_TEST_REWARD_SUPPORT_PRIOR = 'runs/owned-mana-adjustments-publication-01/package'
$env:POE_OPTIMIZER_TEST_REWARD_SUPPORT_OUTPUT = 'runs/owned-reward-support-publication-01'
cargo test --locked -p poe-optimizer-cli --test owned_reward_support_domains -- --include-ignored
```

The all-five publication run passes three tests in 24.73s. It preserves original
local IDs, inputs, policies and provenance, and rebuilds 18 artifacts identically.
There remain 110 metric queries, selected input obligations 107/117/109/123/4,
no evaluation bundle and **0/5 complete native builds**. Strict all-feature Clippy
passes for this target. Evidence is in
`runs/owned-reward-support-native-publication-02.log`,
`runs/owned-reward-support-clippy-01.log` and
`runs/owned-reward-support-publication-01/validation.json`.

The successor package is `runs/owned-reward-support-publication-01/package`:

- Input: `aada9fe41ef814d85dca1a4e20bfbcb3d1c3c12bea1bebcc8352cc6b8252ecc3`.
- Schema: `407ace9074f4a4917d0d23617a631f9844b54799c3a9260b7a0a9a9461f0e438`.
- Rules: `ef7a509a288385042d15f90ee2a67188b6f88d512e3b4962c1c3e25b2b17806e`.
- Compiled: `9f0bcfaf1a3e4a4ce3f9a005af0dba0d94296f0c8960ae45490540bae9ae7435`.

Operations remain V25; no definition IDs are allocated. Other provider families,
exact generated-provider exclusions and the 13 unknown Reward domains remain
separate work. Neither a closed reward formula nor this support certificate
establishes coverage for another owner or for the complete build.
