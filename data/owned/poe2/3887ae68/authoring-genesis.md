# Canonical authoring checkpoint

This complete immutable snapshot preserves all 70 named Stat allocations
alongside their checked runtime. It is outside the compiled manifest inventory
to avoid a self-referential receipt. A clean clone can load it directly; no
historical source rebuild or prior local output is required.

| Commitment | Value |
| --- | --- |
| Archive | `authoring-checkpoints/5f77a8f22888bd946d273a7376c92fe9838535b770dff5a58d6996aa04662a37.zip` |
| Archive bytes | 1,441,115 |
| Archive SHA-256 | `7618ec1fd10f7e7e4b46a2865d988ce37cba829f42047a37f863a3ef4ceef72f` |
| Checkpoint identity | `5f77a8f22888bd946d273a7376c92fe9838535b770dff5a58d6996aa04662a37` |
| Name-ledger identity | `c1eb716472a8af62a998ddf1319fd9bff580e0558b3b95f14f302b9b6f83375c` |
| Registry identity | `e997d7222de1e153706c9fd9c1003568f8c7a3b766112c250f3c8fc677dde8d1` |
| Runtime input identity | `9d9e3326f0a6fdf211f48df8d2813e61f8550012c5ed131ab7e3242d38fae838` |

From the repository root, with fresh output paths:

```powershell
Expand-Archive -LiteralPath data/owned/poe2/3887ae68/authoring-checkpoints/5f77a8f22888bd946d273a7376c92fe9838535b770dff5a58d6996aa04662a37.zip -DestinationPath canonical-checkpoint
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release verify canonical-checkpoint --expected 5f77a8f22888bd946d273a7376c92fe9838535b770dff5a58d6996aa04662a37
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --predecessor canonical-checkpoint --expected-predecessor 5f77a8f22888bd946d273a7376c92fe9838535b770dff5a58d6996aa04662a37 --output rebuilt-checkpoint
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

The support-prohibition checkpoint extends the prior 69-name ledger by exactly
one generic Boolean Skill Stat. The new name sorts before existing names but its
ID appends after the saved registry history. The original 69 bindings remain
unchanged. This snapshot replaces the previous archive; its recorded predecessor
digest is provenance and does not require loading an ancestor.

Verified on October 23, 2026: a fresh extraction passed complete checkpoint
validation, then current tracked source built successor
`0f7962e4163e8e90f424a91176181d1df2160d0d38b0351f081cf43d52aa73a0` with the
same semantic runtime input, ledger and registry. Its eighteen runtime files
total 53,647,754 bytes. All eighteen runtime files and the name ledger replay
byte-for-byte, with no further allocation. Evidence is recorded in
`runs/support-prohibition-extracted-verified.json`,
`runs/support-prohibition-archive.json` and
`runs/support-prohibition-replay-equivalence.json`; those local files are not inputs.

Archive transport is not a second semantic format, a source-reproduction proof
or the planned indexed binary runtime pack. Future canonical allocation
predecessors must likewise be available as one complete checked snapshot; do not
grow a chain of historical source rebuilds or compatibility readers. No complete
build admission or numerical parity is added here. See the
[checkpoint design](../../../../docs/owned-authoring-checkpoint-proposal.md) and
[implementation plan](../../../../docs/implementation.md).
