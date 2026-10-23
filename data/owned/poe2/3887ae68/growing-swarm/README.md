# Growing Swarm: complete default effects and receiving

This packet records a historical publication checkpoint. Its predecessor-specific
publisher has been retired. The maintained Authoring release tests validate the
current source domains, both producer effects and their separate receiving paths.
Publication uses the [current source workflow](../migration-source/README.md).
The release identities and validation results below describe the original checkpoint.

This packet completes the existing default Passive owner `0bc0` (source node
14945, Growing Swarm). Its two effects are 20% increased minion Area of Effect
and 20% increased minion Cooldown Recovery Rate. Neither effect has a Commandable
condition. Both remain tied to the selected Allocation occurrence.

The four new percentage-point Actor channels separate the player's bonuses to
minions from each supplied minion's received values:

| Channel | Purpose |
| --- | --- |
| `32f4` | Player's unconditional minion cooldown Increase contributions |
| `32f5` | Exact Sniper actor's received unconditional cooldown increase |
| `32f6` | Player's minion Area Increase contributions |
| `32f7` | Exact Sniper actor's received Area increase |

Ordinary reducers and stat receivers use the existing Sniper actor slot
`Skill0012/ActorSlot001f`. They neither discover new actors nor bypass parent
inputs, grants, activation or contributor coverage. Area stops at the Actor
input; no radius or final area formula is added.

The existing Action contribution channel `32ea` accepts the received cooldown
increase. Two appended ordinary programs read `Actor32f5` for the actual Basic
and Gas outputs. Existing conditional programs, the `Commandable` fact, and the
upstream conditional channels `32e8`/`32e9` remain exact. In a finite component,
these distinct contributions total Gas 72+20=92 and Basic 0+20=20. The Basic20 is
a receiving amount, not an observed Basic cooldown calculation. All three Gas
stat sets and every existing query/routing record remain unchanged.

## Complete default owner proof

The pinned `src/TreeData/0_5/tree.lua:49470` node body contains precisely the two
stat lines, ordinary Notable identity, connections and presentation/anointment
recipe metadata. There is no mastery, choice, skill grant or socket declaration.
The tracked tree catalog confirms the default view, ordinary point pool, no
attached choices and no class-start adjacency. The pinned PassiveTree constructor
adds connected-to-class flags only to adjacent Normal nodes; this source node is
Notable. The constructed default `modList` also contains exactly the two expected
untagged inner MinionModifier records and no injected flag.

Seven empty declaration inventories and the one full default numerical owner are
closed. Pool, adjacency and every other descriptor field stay exact. The source
records are from `spec.tree.nodes[id].modList`. The observed
`effective_same_definition=false` denotes the separate PassiveSpec wrapper; it
must not be interpreted as a proof that radius jewels, alternative views or other
external transformations have no effect. Those are separate provider obligations.

## Source and publication boundaries

The predecessor is `runs/owned-reward-recovery-inputs-02/package`, input
`f4ee5d5682ab009c024e5fcf57950cbab41640f72a3d83a25cef49be972b1fc8`.
The registry extends from `32f3` through `32f7`. The explicit V4 migration retains
schema V6 and operations V19, then the existing checked passive-refinement API
closes only the reviewed default definition. No public evaluator API changes.

Existing reports `runs/owned-minion-physical-damage-source-04/source-jit-off.json`
and `source-jit-on.json` are byte-identical, 590,791 bytes each, SHA256
`cfd7a36757dc59171b44ec9f6e8bba8960467d26cf02e5cd0d58e8926b075652`.
They include the original selection, real Gas selection and restored original,
with four complete loads per mode. Source definitions, actual cooldown modifier
ownership and original receiving observations are authenticated.

The report does **not** contain an independent Area receiving Sum result. Its
exact Area producer is observed; the generic original
`CalcPerform.addMinionModifiers` transfer is authenticated from the pinned body.
The native Area20 receiver check therefore proves that explicit data transport,
not numerical parity for the full source area calculation. No new source runtime
or copied final number is required or claimed.

`dependencies.json` stores exact predecessor definitions, programs and slots;
`closure.json` contains the single refined default Passive; `migration.json`
contains four new channels, their four receiver programs and two Action appends.
`source-vectors.json` preserves bounded exact source observations and complete
static-node evidence. The retired shared passive publisher preserved the earlier
Command packet's provenance domain and reconstructed the exact expected successor
recipe. It compared every unrelated schema, rule, routing, import and query field.

## Verification

At that checkpoint, twelve combined Command/Growing Swarm native tests and four
fast authoring checks passed. The checked publication at
`runs/owned-growing-swarm-02/validation.json` passed, with endpoint
`caa08fd53b2902cbf6b6362daa478bba843aad00944a173fe3a8a4a85fc5cc58`.
The historical Command replay at `runs/owned-command-shared-replay-01/validation.json`
also passed with the extracted publisher. Logs are
`runs/owned-growing-swarm-{focused-02,publication-02,historical-01}.log`.
Strict workspace/all-feature/all-target Clippy and package formatting passed;
lint log: `runs/owned-growing-swarm-clippy-01.log`.

The historical publication normalized all five original builds against both
endpoints, preserved 110 ordered queries and selected issue counts
`114/117/109/122/11`, and required all eighteen rebuilt files to match. The
maintained ordinary proof authenticates tracked source archives and packet
evidence without a generated predecessor package, local reports or Lua execution.
Source-domain coverage does not complete the selected builds or add an evaluation bundle.

The reused native Command fixture supplies two distinct Sniper populations and
explicitly finite parent inputs and contributor coverage. New checks retain exact
Allocation and generated Action identities, independent conditional/unconditional
contributions, removal behavior, missing-input and coverage failures, and scratch
reuse. They do not authorize those finite closures in the real package.

```powershell
cargo test -p poe-optimizer-authoring --test release minion_support_domains
cargo test -p poe-optimizer-cli --test owned_command_cooldown_native
```

This packet does not implement final cooldown duration, tick rounding, base uses,
final Area/radius, other minion
families' receiving declarations or complete incoming contributor inventories.
