# Canonical authoring checkpoint

This complete immutable snapshot preserves all 73 named Stat allocations
alongside their checked runtime. It is outside the compiled manifest inventory
to avoid a self-referential receipt. A clean clone can load it directly; no
historical source rebuild or prior local output is required.

| Commitment | Value |
| --- | --- |
| Archive | `authoring-checkpoints/71ff4429b7fcfb4f75e45d9c58df878453cb25428542d3531074fc08fccbba91.zip` |
| Archive bytes | 1,442,323 |
| Archive SHA-256 | `fc1f557b37c1318b5a89842755fa1faf2052d34e24405dcd20f4289caeeccd6f` |
| Checkpoint identity | `71ff4429b7fcfb4f75e45d9c58df878453cb25428542d3531074fc08fccbba91` |
| Name-ledger identity | `9e2c57cce54eb76422fd49cf3d259b51e0477a47212ed1a29817ebe7345cebc1` |
| Registry identity | `2ba7a660227ce8c10acfa8e1194c4ff08078367290313862e0037b544adbbd80` |
| Runtime input identity | `f94b745619f5a1b4fd40339289b5a62edfe3d103ff4bca69d0ca7393557a9d66` |

From the repository root, with fresh output paths:

```powershell
Expand-Archive -LiteralPath data/owned/poe2/3887ae68/authoring-checkpoints/71ff4429b7fcfb4f75e45d9c58df878453cb25428542d3531074fc08fccbba91.zip -DestinationPath canonical-checkpoint
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release verify canonical-checkpoint --expected 71ff4429b7fcfb4f75e45d9c58df878453cb25428542d3531074fc08fccbba91
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --predecessor canonical-checkpoint --expected-predecessor 71ff4429b7fcfb4f75e45d9c58df878453cb25428542d3531074fc08fccbba91 --output rebuilt-checkpoint
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

The revival checkpoint preserves the prior 70 bindings and appends exactly three
generic Skill/Actor Stats. The earlier support-prohibition allocation likewise
preserved the original 69 bindings despite its earlier-sorting name. The saved
ledger determines identity history. This snapshot replaces the previous archive;
its recorded predecessor digest is provenance and does not require an ancestor.
The [revival producers](revival-producers/README.md) add component contributions,
while source census, final duration and complete-build coverage remain unresolved.

Verified for revival checkpoint `bf533382` on October 24, 2026: the extracted
predecessor passed complete checkpoint validation, then that source built and
independently verified successor
`dd2d337ab5ce1f0970a0644491636aa6d710895e3beb804dce5f8368da63a5f8` with the
same semantic runtime input, ledger and registry. Its eighteen runtime files
total 53,655,992 bytes. All eighteen runtime files and the name ledger replay
byte-for-byte, with no further allocation. Evidence is recorded in
`runs/revival-producers-extracted-verified.json`,
`runs/revival-producers-replay-verified.json`,
`runs/revival-producers-replay-equivalence.json` and
`runs/revival-producers-archive.json`; those local files are not inputs.
Replay changes the checkpoint's predecessor commitment without allocating more
names or changing runtime bytes. The snapshot above remains the explicit predecessor.

The subsequent item and skeletal support-domain tranches preserve this entire
name ledger and registry. Current tracked source publishes verified checkpoint
`9c052dd7909f65b7654f40892859d82b55cbb79cc11ffcc208b07e41c5cf7762`, with runtime
input `baa13efffdac5ca8b784b53e83254c5fc46ba9a0cdc142de8c2e392f8a768ba6`.
The current runtime differs from the allocation snapshot only in rule metadata
and the corresponding manifest/receipt commitments. This snapshot remains the
allocation predecessor; it is not advertised as the latest evaluated content.
Build the tracked source to obtain the current release. Local verification:
`runs/skeletal-support-verified.json` and `runs/skeletal-support-inventory.json`.

Archive transport is not a second semantic format, a source-reproduction proof
or the planned indexed binary runtime pack. Future canonical allocation
predecessors must likewise be available as one complete checked snapshot; do not
grow a chain of historical source rebuilds or compatibility readers. No complete
build admission or numerical parity is added here. See the
[checkpoint design](../../../../docs/owned-authoring-checkpoint-proposal.md) and
[implementation plan](../../../../docs/implementation.md).
