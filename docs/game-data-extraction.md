# Pinned native game-data extraction

`extract-game-data` generates the current native JSON package from the pinned PoB source.
It is optional development tooling: native evaluation loads the exported package and has
no Lua or worker-process dependency. No website is scraped or downloaded.

The exporter covers the same twelve explicitly partial sections as the
[native data package](native-data.md): tree, character, quests, Spark, Mace, supports, weapons,
item modifier rules, defence, monsters, encounters and typed owned passive effects. The current version includes the four admitted ascendancy
resistance nodes. It does not infer arbitrary build mechanics
or broaden accepted source revisions. Progress and validation evidence belong in the
[living implementation record](implementation.md).

## Run the exporter

Use the default development build with its `pob` feature and initialized pinned submodule.
The output directory must already exist; both output paths must be new.

```powershell
cargo run --locked -- extract-game-data --output runs/extracted-game-data.json
cargo run --no-default-features --locked -- evaluate examples/native-witch-entrance.xml --data runs/extracted-game-data.json --raw
```

The first command writes canonical package bytes and a companion:
`runs/extracted-game-data.json.extraction.json`. JSON printed to stdout includes the
package digest, byte count, data identity, section digests, output paths and evidence.
`--pob <directory>` selects a local copy of the same verified source inventory.
`--timeout-seconds <positive integer>` defaults to 30 and covers worker startup, source
verification, extraction, artifact decoding and parent validation. A final deadline check
runs after host validation/serialization and before file publication. Filesystem publication
and stdout delivery are outside the extraction deadline. Every worker is reaped.
Native-only builds omit this command.

Existing package or evidence destinations reject before extraction. Files are published
without overwriting existing entries. The two-file publication is not a filesystem
transaction; an I/O error during publication can leave one successfully written file.
A nonzero exit is not a completed export.

## Source, policy and evidence

The exporter constructs records from source rather than using the bundled package as a
template. Existing authenticated tree extraction supplies topology and attributes; numeric
records come from verified source tables, functions and expressions. Source verification
uses the committed full inventory and normalizes CRLF to LF. The exact source bytes used
for extraction are checked again before execution.

Conversion policy selects the supported profiles, weapon slots, quest positions and typed
operations. It also defines deterministic ordering and the treatment of absent optional
values. This policy is versioned extraction logic; it is separate from patch-dependent
numeric values. Typed modifier conversion must account for complete parser output, flags,
actor scope and values. Unconsumed or ambiguous modifiers reject.

The companion retains source revision/inventory identity, consumed-file hashes,
extractor/policy identity, package schema/semantics and the resulting package digest. Its
26 direct source-file entries cover extraction and retained provenance reads. Additional
tree/loader/spec evidence remains in the package's `tree.source` record.
It describes how this artifact was produced. Native loading continues to use explicit
host trust and actual content identity; a sidecar claim does not grant trust or establish
numerical parity. The normal package loader validates records and section digests.

## Isolation and limits

Extraction runs in a fresh supervised process with no inherited standard I/O and a hidden
window on Windows. Lua libraries/module access and source chunks are restricted. LuaJIT
is disabled; Lua allocations are limited to 128 MiB, with an instruction hook checking
every 10,000 instructions and rejecting after approximately 200 million instructions.
Direct consumed source text is limited to 64 MiB; full-inventory verification and tree
extraction retain their separate read/record limits. This is offline extraction of reviewed source, not a general-purpose Lua sandbox.
The parent bounds the private package/evidence envelope to 4 MiB and error output to
64 KiB, checks ordinary-file status and rejects noncanonical or malformed envelopes.
The package retains its separate 2 MiB portable-loader limit. The deadline includes the
parent's decoding and evidence validation; Rust conversion and serialization are covered
by process supervision.

The worker's private canonical envelope is separate from the public package format. Public
package loading still accepts valid formatting choices and identifies the actual bytes.

## Reproduction and further updates

For the current source, extracted package bytes should exactly reproduce the reviewed
artifact. Repeated fresh processes must produce the same package and evidence on Windows
and Linux. Existing independent source-oracle and full-build parity tests stay separate
from the exporter so that reproduction does not become its own only correctness check.

```powershell
cargo test -p poe-optimizer-pob --lib game_data --locked
cargo test -p poe-optimizer-pob --test game_data --locked
cargo test -p poe-optimizer-cli --test game_data_extraction_cli --locked
```

The source revision remains `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. The exporter now emits
schema 5 and `poe2-native-profiles-v5`, including source-derived item grammar/mappings,
global critical-chance cap, level/attribute requirements,
support-color costs and owned passive effects. Signed values are allowed only for the
new unconditional player BASE resistance operations. The structural selection policy in
`crates/poe-optimizer-data/data/class-tree-policy.json` identifies the four source nodes;
Rust does not contain their numeric values. Regenerate older packages rather than silently filling missing
records. Native controlled search consumes the same selected data; new tree revisions and
arbitrary operation versions still require compatibility review.


When intentionally changing the retained policy or package schema, maintainers can prepare
new artifacts before changing the compiled reviewed digests:

```powershell
cargo run -p poe-optimizer-pob --example regenerate_game_data --locked -- runs/reviewed-migration
```

This explicit source-review helper requires a new directory, verifies the same pinned full
source, and writes package/tree bytes, their digests and extraction evidence. It never edits
the committed artifacts or grants runtime trust. Review policy/data changes and independent
parity first, then deliberately update the reviewed artifacts. Ordinary `extract-game-data`
continues to authenticate its output against the committed reviewed tree/package scope.

Support identities, eligibility, scoped modifiers and damage flags are extracted into the
`supports` section; see [support data and migration](support-loadouts.md). The retained
tree bytes and upstream revision do not change for this support extension.

The schema-5 item-rule converter invokes original `ModParser` with distinct numeric sentinels
to retain the complete local stat, operation, flags, keyword flags and capture mapping. The
reviewed five templates use source-verified integer capture grammar. Source critical-cap
extraction requires the exact unconditional `BASE` modifier shape before taking its value;
scoped, tagged or changed-operation records reject. Cold/warm parser and local-assembly
oracles remain independent checks. See [local weapon configuration](local-weapons.md).
