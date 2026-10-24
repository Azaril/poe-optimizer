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

The subsequent item/skeletal support-domain and generated-singleton accounting
tranches preserve this entire 73-name ledger and registry. Current tracked source
publishes verified checkpoint
`2be706a87dd1d7ab916565121422cb6fd16e7d0a04268a590d3887c01901c3f6`, with runtime
input `58aa33797506bbd1c49988059756f04749a46207a8d85b569683a0b66f07e8dc`.
Compared with the preceding skeletal checkpoint, exactly four inventory
destinations and their derived dependency commitments change; the only changed
runtime files are `normalization.json`, `tree-normalization.json` and `release.json`.
Historical restoration restores those four destinations through checked content
constructors, alongside the independently bounded earlier content deltas.

| Current tracked-source build | Count or bytes |
| --- | ---: |
| Declared source files | 2,802 |
| Encoded source bytes | 6,728,208 |
| Decoded source bytes | 46,027,282 |
| Root manifest bytes | 184,374 |
| Runtime files | 18 |
| Runtime bytes | 53,659,348 |
| Schema artifact bytes | 19,774,085 |
| Checkpoint and name-ledger bytes | 455,214 |

The saved verifier accepts the current checkpoint, and replay reproduces all
eighteen runtime files and the name ledger without allocation. Evidence is in
`runs/generated-singleton-usage/verified.json`, `equivalence.json` and
`inventory.json`; these local records are not build inputs. The canonical
`71ff4429` snapshot above remains the allocation predecessor, not the latest
content. Build the tracked source to obtain the current release. Current accounting
validation passes all 391 Authoring tests (175 library, eight CLI and 208 release)
without optimization, including the three singleton laws. Strict Import/Authoring
package lint also passes. The separate historical-witness repair passes both
CLI feature profiles; hosted confirmation remains separate from these local checks.

Archive transport is not a second semantic format, a source-reproduction proof
or the planned indexed binary runtime pack. Future canonical allocation
predecessors must likewise be available as one complete checked snapshot; do not
grow a chain of historical source rebuilds or compatibility readers. No complete
build admission or numerical parity is added here. See the
[checkpoint design](../../../../docs/owned-authoring-checkpoint-proposal.md) and
[implementation plan](../../../../docs/implementation.md).
