# Canonical first-allocation checkpoint

This is the reproducible predecessor for the maintained authoring history. It
contains the first 69 named Stat allocations, alongside the existing registry.
It is outside the compiled manifest inventory to avoid a self-referential receipt.

| Commitment | Value |
| --- | --- |
| Git source revision | `caca40a55885e5c8612e7d9b47c1691f8ea321a9` |
| Manifest within that revision | `data/owned/poe2/3887ae68/migration-source/release.kdl` |
| Checkpoint identity | `e89ae92ac3895a3e9f38591452d340b8c640d4af737a544cc18759de4d98b109` |
| Name-ledger identity | `dabf9a92dffb7e9ec36ee587e1dd377bc9aaffa096373c32062ac4428dadabd2` |
| Runtime input identity | `802bf2eccbdf83814dbb34f0745a2006c75ec4fdcd80e4ee8fc80e053852cdcb` |

Use the maintained compiler, not a compiler checked out from the source revision.
The source revision supplies only data. A full clone already contains it; shallow
clones and CI must fetch that exact revision before reproduction. No prior local
`runs/` directory, PoB checkout, old migration helper or separate ledger importer
is required.

From the repository root, with fresh output paths:

```powershell
$genesisRevision = "caca40a55885e5c8612e7d9b47c1691f8ea321a9"
git archive --format=zip --output=genesis-source.zip $genesisRevision data/owned/poe2/3887ae68/migration-source
Expand-Archive -LiteralPath genesis-source.zip -DestinationPath genesis-source
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build genesis-source/data/owned/poe2/3887ae68/migration-source/release.kdl --bootstrap --output genesis-checkpoint
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --predecessor genesis-checkpoint --expected-predecessor e89ae92ac3895a3e9f38591452d340b8c640d4af737a544cc18759de4d98b109 --output rebuilt-checkpoint
```

The second command to `poe-author` verifies the exact expected genesis identity,
canonical ledger and complete runtime through the normal checked loader before
using it. The final runtime is `rebuilt-checkpoint/runtime`. Bootstrap only the
fixed source above; do not bootstrap evolving current sources to reconstruct IDs.
The same-source successor has a different checkpoint identity because its receipt
records a predecessor, while names and runtime bytes remain identical.

Verified on October 23, 2026: an archive of the committed revision reproduced the
exact checkpoint identity with the maintained CLI. Loading that fresh checkpoint
and compiling current source produced successor
`d15cabc5774b85149fa98d7672257e8b9188fa586c9fee8c598cdf732d8c4622`; the name ledger
and all eighteen runtime files matched byte for byte. Local evidence is in
`runs/initial-types-archived-{genesis,successor}.{stdout,stderr}.log`, not required
inputs. The Rust authoring/host suites pass 245 tests; this source adds no complete
build admission or numerical parity claim.

This is one bounded bridge to R3 storage/distribution. Before a second allocation
generation needs another canonical predecessor, archive or distribute one full
checked snapshot that can be loaded directly. Do not extend this into a chain of
historical source rebuilds. Compiler/source-format cutovers deliberately regenerate
the canonical checkpoint/reference rather than keeping compatibility readers.
See the [checkpoint design](../../../../docs/owned-authoring-checkpoint-proposal.md)
and [implementation plan](../../../../docs/implementation.md).
