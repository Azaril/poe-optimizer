# Reward recovery and capacity inputs

This packet completes three existing Reward producer inventories selected by
Original05. It appends four Actor contribution channels and three ordinary rule
programs. The two Charm effects remain together under their actual Reward owner.

| Reward | Source record | Owned Player contribution |
| --- | --- | --- |
| `0036`, Venom Draught | 30% increased Elemental Ailment Threshold | Increase `32f0`, percentage points `0002` |
| `003b`, Goddess of Justice | 30% increased Life Recovery from Flasks | Increase `32f1`, percentage points `0002` |
| `002d`, Medallion | 30% increased Charm Charges Gained | Increase `32f2`, percentage points `0002` |
| `002d`, Medallion | +1 Charm Slot | Add `32f3`, Count `295a` |

These are source contributions. The packet contains no reducer, receiver, final
threshold, flask recovery formula, charm charge rate, or final charm capacity.
The last channel is distinct from the existing item-local Charm capacity channel.
Conditional thresholds, flask interactions, charm capacity caps and other selected
owners remain separate obligations. A complete Reward producer inventory does
not make the selected build or its incoming contribution inventory complete.

## Data and source evidence

The exact predecessor is `runs/owned-source-presentation-03/package`, input
`07dc985c86fc2dc2467281e920b902b21f99d7fbedc720105c550f177b631b51`.
Schema V6 and operation V19 stay unchanged. The existing checked successor extends
the registry from `32ef` through `32f3` and changes the schema release to
`pob-3887ae68-reward-recovery-inputs-v1`. Every prior descriptor, rule body, input
policy and query is retained; only authenticated dependency identities rebind.

`closure.json` contains four new Stat descriptors and three complete Reward
owners. `dependencies.json` records their exact existing Reward schemas, units and
absent numerical owners. `bindings.json` joins each exact source option and
selected value to its Reward and contribution channels. `source-vectors.json`
retains bounded exact observations from the independently authenticated reports,
including the source reward census, original callback output, typed selection
policy, mapping and selected Original05 MAIN/CALCS quest records. Shared quest
labels alone are never used as reward identity.

The existing complete-source reports are
`runs/owned-configuration-reward-source-01/source-jit-off.json` and
`source-jit-on.json`. They are byte-identical: 6,246,344 bytes each, SHA256
`4fa239bb7f72b936b522a84fb6adb0139cdddb2a4b593ce69a64bb722f7a2e9d`.
They cover 30 complete loads and 39 finite callback cases per mode, with original
source methods and no business-method replacement. The four relevant callback
records have zero flags, zero keyword flags and no recipient tags or enemy
records, matching Player contributions. Publication verifies the pinned source
files and exact report bytes; native evaluation does not load PoB.

## Validation and reproduction

Nineteen focused authoring/native checks and the five-original publication pass.
The new packet has five native tests; prior flat/permanent reward tests remain
active. Ordinary authoring checks use only tracked artifacts and the pinned source
manifest. The optional publication gate also needs
the exact predecessor, source checkout and authenticated reports. It reverses only
the four descriptor additions, three numerical owners, registry extension and
explicit dependency rebinding, then compares the complete prior release. All five
original imports, selected rewards and 110 queries must survive; a second package
publication must reproduce all eighteen files byte-identically.

Checked output is `runs/owned-reward-recovery-inputs-02/package`, input
`f4ee5d5682ab009c024e5fcf57950cbab41640f72a3d83a25cef49be972b1fc8`.
Its eighteen files total 60,841,623 bytes, reproduce byte-identically and retain
all five selected issue sets (114/117/109/122/11). The publication uses each
endpoint's own definitions during intent validation. Evidence is in
`runs/owned-reward-recovery-inputs-02/validation.json`; logs are
`runs/owned-reward-recovery-{focused,publication,clippy}-02.log`.
Strict workspace/all-feature/all-target Clippy passes. No evaluation bundle is
added; complete native builds remain 0/5.
The historical permanent-reward publication also passes through the extracted
helper, preserving its exact endpoint and earlier issue counts. Its evidence is
`runs/owned-permanent-reward-shared-replay-01/validation.json` and the log is
`runs/owned-reward-recovery-historical-01.log`.

The finite native tests use the exact packet programs and the existing item/reward
fixture. They check the four contributions, both Charm records, independent Reward
origins and coverage failures. The fixture's existing Spirit scalar is only an
unchanged sentinel; it does not supply a reducer for the four new channels.

```powershell
cargo test --test owned_reward_recovery_inputs_native
cargo test --test owned_flat_resource_rewards_cli recovery_reward_packet_has_all_four_source_effects

$env:POE_OPTIMIZER_TEST_RECOVERY_REWARDS_PRIOR = 'runs/owned-source-presentation-03/package'
$env:POE_OPTIMIZER_TEST_RECOVERY_REWARDS_OUTPUT = 'runs/owned-reward-recovery-inputs-reproduction'
cargo test --test owned_flat_resource_rewards_cli publish_reward_recovery_inputs_preserving_all_five_originals -- --include-ignored --nocapture
```

The output directory must not already exist. Historical reward publication gates
continue to use their exact earlier predecessors; the shared helper does not
rewrite or reinterpret those endpoints.
