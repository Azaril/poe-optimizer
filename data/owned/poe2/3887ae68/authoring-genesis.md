# Canonical first-allocation checkpoint

This complete immutable snapshot preserves the first 69 named Stat allocations
alongside their checked runtime. It is outside the compiled manifest inventory
to avoid a self-referential receipt. A clean clone can load it directly; no
historical source rebuild or prior local output is required.

| Commitment | Value |
| --- | --- |
| Archive | `authoring-checkpoints/c588b782e7683f74f4e3ead45360364a7e6f9c69e3e4430f0783ebf4f45a6124.zip` |
| Archive bytes | 1,439,333 |
| Archive SHA-256 | `3dec46a43add68b63b095f07e58cd0c3275dc204ff286726175e5fe918170254` |
| Checkpoint identity | `c588b782e7683f74f4e3ead45360364a7e6f9c69e3e4430f0783ebf4f45a6124` |
| Name-ledger identity | `dabf9a92dffb7e9ec36ee587e1dd377bc9aaffa096373c32062ac4428dadabd2` |
| Registry identity | `003d28c7f38b4b3f72e3c39b2f25462d216cbf3fc7c6f67c393073d127ddfcb8` |
| Genesis runtime input identity | `802bf2eccbdf83814dbb34f0745a2006c75ec4fdcd80e4ee8fc80e053852cdcb` |

From the repository root, with fresh output paths:

```powershell
Expand-Archive -LiteralPath data/owned/poe2/3887ae68/authoring-checkpoints/c588b782e7683f74f4e3ead45360364a7e6f9c69e3e4430f0783ebf4f45a6124.zip -DestinationPath genesis-checkpoint
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release verify genesis-checkpoint --expected c588b782e7683f74f4e3ead45360364a7e6f9c69e3e4430f0783ebf4f45a6124
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --predecessor genesis-checkpoint --expected-predecessor c588b782e7683f74f4e3ead45360364a7e6f9c69e3e4430f0783ebf4f45a6124 --output rebuilt-checkpoint
```

The archive contains `checkpoint.json`, `names.json` and the eighteen files in
`runtime/`. Verification uses the ordinary bounded loader to check the complete
ledger/runtime closure against the independently supplied identity. It needs no
source files or ancestor checkpoints. Current builds consume this checked ledger
as an explicit predecessor; never bootstrap evolving sources to reconstruct IDs.
Runtime consumers use `rebuilt-checkpoint/runtime`.

The maintained Rust corpus and release tests consume this same persisted
predecessor. Their shared test-only transport authenticates the archive, reads
its bounded inventory in memory and calls the ordinary checkpoint validator.
Tracked-source arithmetic mutations and new declarations preserve that ledger;
synthetic bootstrap tests separately exercise first allocation. The transport
library is not a production or native evaluation dependency.

The October 23 compact schema cutover regenerated storage from the original
first-allocation source at Git revision
`caca40a55885e5c8612e7d9b47c1691f8ea321a9`, replacing only its bootstrap schema
with the new canonical encoding before using the maintained compiler. Name
allocations, registry identity and semantic runtime input are unchanged. Schema
artifact, manifest, source and checkpoint transport commitments deliberately
change; the replaced expanded-artifact reader and one-time converter are gone.
The original source revision alone is no longer a current-format reproduction
command. The snapshot replaces that temporary source-rebuild bridge.

Verified on October 23, 2026: a fresh extraction passed complete checkpoint
validation, then current tracked source built successor
`3872b395afc00c61f75e11804fdab4b7b91993806711817f17f03a861657bf31` with semantic
runtime input
`3ede2f3da137c16ccf3a3d3cbb914f9b1a65db717d86edd339346ccdbdd22d43` and the same
ledger/registry. Its eighteen runtime files total 53,647,002 bytes. Evidence is
recorded in `runs/compact-schema-*-verified.json` and
`runs/compact-schema-checkpoint-inventory.json`; those local files are not inputs.

Archive transport is not a second semantic format, a source-reproduction proof
or the planned indexed binary runtime pack. Future canonical allocation
predecessors must likewise be available as one complete checked snapshot; do not
grow a chain of historical source rebuilds or compatibility readers. No complete
build admission or numerical parity is added here. See the
[checkpoint design](../../../../docs/owned-authoring-checkpoint-proposal.md) and
[implementation plan](../../../../docs/implementation.md).
