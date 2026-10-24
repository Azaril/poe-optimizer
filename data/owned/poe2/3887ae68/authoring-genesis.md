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
tranches preserve this entire 73-name ledger and registry. The singleton tranche
published verified checkpoint
`2be706a87dd1d7ab916565121422cb6fd16e7d0a04268a590d3887c01901c3f6`, with runtime
input `58aa33797506bbd1c49988059756f04749a46207a8d85b569683a0b66f07e8dc`.
Compared with the preceding skeletal checkpoint, exactly four inventory
destinations and their derived dependency commitments change; the only changed
runtime files are `normalization.json`, `tree-normalization.json` and `release.json`.
Historical restoration restores those four destinations through checked content
constructors, alongside the independently bounded earlier content deltas.

The subsequent tracked source enabled the independently reviewed
[default Timeless search-preference family](timeless-search-defaults/README.md).
It publishes verified checkpoint
`54496d58233f41b8ef15cab738edf764ffc7d08fd1d97d69e3c53684f9c8eac8`, runtime input
`5145862bfbaf30279b430888df2b69258e097d242ee05be81b198ecfafe1edcd` and release
receipt SHA-256 `afaf818acf734cdfe56a2aa41878312e0f620b315b597ef7a93e3c80e82a89fd`.
The name ledger and registry are unchanged. Only the presentation flag and its
constructed tree normalization binding change; the same three runtime files
listed above have new bytes. No numerical or completion state changes.

The preceding tracked sources extend the same intrinsic revival body to the exact
Arsonist, Frost Mage and Reaver Skills and their physical Gem grants. This adds
three program occurrences and six query members with all Partial gaps retained.
The verified checkpoint is
`82677b8da2f00142276ba573854f00242dde4f9448246baca804524fed4d0b9a`, runtime input
`cc25c852ad3918a6fa610fe423890bb7d89e3c2fe03f1be264fb3a4ecd0004ea`, with release
receipt SHA-256 `d14bc0e44c04c64c7ab7ed22f0f8211d5ad3d5b30a6f7a2c8bf5f7f7d0b19af1`.
Compared with the Timeless checkpoint, only `rules.json`, `manifest.json` and
`release.json` change. Schema, ledger, registry, Import data and queries retain
their bytes. No new allocation or complete-build admission is introduced.

Current tracked sources additionally declare the intrinsic support-source domain
of the independently reviewed [fixed-Life modifier](flat-life-support-domain/README.md).
Its six numerical programs and all four owner gaps remain unchanged. The verified
checkpoint is `5f3ea0e0f5a7a91207827cdb5effe16230df509f077a278fa802c982ed4299ba`,
runtime input `de3998a03a2aa932a8a5766c37a63c9ee4eedd2c974d1ecf5730272345ca596a`,
with release receipt SHA-256
`6b6b654eaeb73e2c629db7a6e4e8a1ac8244c6c0bef9b6f352573942a4517b92`.
Only `rules.json`, `manifest.json` and `release.json` change from the preceding
revival publication. All other runtime artifacts, the ledger and registry remain
byte-identical; no new allocation or complete-build admission is introduced.

| Current tracked-source build | Count or bytes |
| --- | ---: |
| Declared source files | 2,802 |
| Encoded source bytes | 6,731,170 |
| Decoded source bytes | 46,030,483 |
| Root manifest bytes | 184,376 |
| Runtime files | 18 |
| Runtime bytes | 53,665,744 |
| Schema artifact bytes | 19,774,085 |
| Checkpoint and name-ledger bytes | 455,218 |

The saved verifier accepts the current checkpoint, and an independent build
reproduces all twenty files byte-for-byte: eighteen runtime files, the name ledger
and checkpoint receipt. Evidence is in
`runs/flat-life-domain-checkpoint/release-verified-02.json` and
`release-equivalence.json`; these local records are not build inputs. The canonical
`71ff4429` snapshot above remains the allocation predecessor, not the latest
content. Build the tracked source to obtain the current release. Validation and
the actual remaining first-build blockers are recorded at each checkpoint in
the [living implementation plan](../../../../docs/implementation.md). Hosted
confirmation remains separate from local tests and immutable publication checks.

Archive transport is not a second semantic format, a source-reproduction proof
or the planned indexed binary runtime pack. Future canonical allocation
predecessors must likewise be available as one complete checked snapshot; do not
grow a chain of historical source rebuilds or compatibility readers. No complete
build admission or numerical parity is added here. See the
[checkpoint design](../../../../docs/owned-authoring-checkpoint-proposal.md) and
[implementation plan](../../../../docs/implementation.md).
