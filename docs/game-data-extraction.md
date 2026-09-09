# Pinned native game-data extraction

`extract-game-data` generates the current native JSON package from the pinned PoB source.
It is optional development tooling: native evaluation loads the exported package and has
no Lua or worker-process dependency. No website is scraped or downloaded.

The exporter covers the versioned sections of the
[native data package](native-data.md): tree, character, actor, receiving defences, movement, action speed, direct timing, quests, Spark, Mace, supports, weapons,
item modifier rules, item source formatting, jewellery and fixed armour bases, defence, monsters, encounters, typed owned passive
effects, explicit passive exclusions, configuration metadata, constructed skill/gem identities,
item definitions and general item scalability. Numerical mechanic coverage remains partial.
Whole ordinary structure is separate from
capability admission: 1,282 complete source views are supported, including the four
admitted ascendancy resistance nodes; 3,476 source views are explicitly excluded. It does not infer arbitrary build mechanics
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
direct source-file entries cover extraction and retained provenance reads. Additional
tree/loader/spec evidence remains in the package's `tree.source` record.
It describes how this artifact was produced. Native loading continues to use explicit
host trust and actual content identity; a sidecar claim does not grant trust or establish
numerical parity. The normal package loader validates records and section digests.

The configuration extractor constructs all original `ConfigOptions.lua` rows, including
generated quest choices, using the original global/misc definitions and boss-data tooltip
producer. `Bosses.lua` and `BossSkills.lua` are authenticated construction dependencies.
It preserves ordered duplicate keys, original typed values and recursive metadata; callback
source locations/hashes are inert descriptors. It does not execute effect or UI callbacks.
Independent original `ConfigTab` default-state oracles remain separate from the exporter.

The skill identity extractor executes the original skill/gem assembly in `Data.lua`, its
nine actual skill modules, real level data and construction helpers. It preserves ordered
raw declarations separately from final constructed identities, generated additional effects,
display ordering and absent references. Seventeen source files bind this section to its
construction dependencies. Independent original-source tests compare every declaration,
constructed row, source span and lookup behavior. Identity membership does not admit native
mechanics; see [skill source and identities](skill-source-and-identities.md).

## Isolation and limits

Extraction runs in a fresh supervised process with no inherited standard I/O and a hidden
window on Windows. Lua libraries/module access and source chunks are restricted. LuaJIT
is disabled. The numerical extractor VM has a 128 MiB allocation limit and an instruction
hook checking every 10,000 instructions, rejecting after approximately 200 million
instructions. The separate configuration-construction VM has a 64 MiB limit and an
approximately 100-million-instruction bound. The separate skill/gem construction VM has a
256 MiB limit and an approximately one-billion-instruction bound, checked every 100,000
instructions, with JIT disabled. These are per-VM bounds, not a total process memory limit.
Direct consumed source text is limited to 64 MiB; full-inventory verification and tree
extraction retain their separate read/record limits. This is offline extraction of reviewed source, not a general-purpose Lua sandbox.
The parent bounds the private package/evidence envelope to 17 MiB and error output to
64 KiB, checks ordinary-file status and rejects noncanonical or malformed envelopes.
The package retains its separate 16 MiB portable-loader limit and one-million-value bound. The deadline includes the
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
schema 15 and `poe2-native-profiles-v15`, including source-keyed passive/actor effects,
structural attribute/replacement metadata, jewellery and four fixed armour slots, movement
formula/penalty data, shared action-speed/direct-timing parameters and explicit excluded views. Item penalty absence and zero remain
distinct; actual parser checks exclude unsupported conditional special phrases. It
retains normalized actor modifiers, actor rules,
the full source precision table and Spirit quest records, alongside the existing item rules,
critical-chance cap, requirements, support-color costs and passive effects. Signed actor
numeric values and unconditional player BASE resistance values retain their distinct typed
operation and scope validation. The structural selection policy in
`crates/poe-optimizer-data/data/class-tree-policy.json` identifies the four admitted ascendancy source nodes; complete ordinary structure is
converted separately and admitted by whole-effect capability rather than an ID allowlist;
Rust does not contain their numeric values. Regenerate older packages rather than silently filling missing
records. Native controlled search consumes the same selected data; new tree revisions and
arbitrary operation versions still require compatibility review.


The `item_loading` section executes authenticated source construction for the full item base,
modifier, raw unique and jewel-radius definitions. It retains hidden/unknown metadata and
source locations for inert callbacks. Parsing those callbacks or constructing the parsed
unique database remains a separate consumer; catalog extraction does not silently replace
missing modifier effects. Independent item-loading oracles execute the original parser and
assembly methods separately from this exporter. Regeneration preserves all preceding section
values and the pinned source/tree; see [item loading](item-source-and-loading.md).


The `item_scalability` section preserves complete exact-case keys and ordered capture
records from the original `Data/ModScalability.lua` table. Format dispatch is recorded as partial
assignments, retaining raw ignored labels; numeric defaults, antonyms and catalyst policy
come from their original source definitions. Existing actor precision and catalyst tables
are reused. Extraction does not turn label names into behavior or admit unknown effects.
Independent runtime comparisons validate the complete catalog and exercise original
formatting separately from the exporter; see [general item formatting](item-formatting.md).

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


The actor converter observes the actual source parser's form, special-modifier and
condition tables, then invokes the original parser with distinct signed, fractional or
integer operands as appropriate. The original 116 templates cover seven individual
attribute/resource/accuracy targets with BASE, increased/reduced and more/less forms,
including two attribute-comparison suffixes, plus fixed bonus/override phrases and the
Dexterity accuracy override. It retains every emitted target, effect, source, scope field
and ordered supported condition tag. Unconsumed bookkeeping, unknown targets/conditions,
extra fields, scoped flags and unsupported numerical transforms reject. This is a reviewed
grammar subset, not a general transcription of `ModParser`.

Actor constants come from the original setup, attribute-bonus and maximum-resource code.
Level-based initialization is checked as a complete source modifier, including its Level
multiplier tag and offset. The converter retains the full `data.highPrecisionMods` table,
including BASE entries; MORE consumers select their explicit operation from the package.
Three actual Spirit quest callbacks supply keys, defaults, values and `Quest:` source
strings. No existing character or quest balance table is duplicated in the actor section.

The receiving extension adds 204 templates, preserving exact Global tags, complete paired
outputs and source receiver query groups. All defensive passive effects are ordered actor
records; the extractor refuses an incomplete or mixed conversion.

Earlier validation recorded 3,750 pinned-parser inputs in cold and warmed modes, compared
all 40 precision records and exercised the original quest callbacks and actor constant
branches. These historical counts precede movement; see the
[living implementation checkpoint](implementation.md) for current validation totals.
Separate extraction mutation tests reject discarded fields, changed
source scope, unsupported tags and mismatched rule operands. These checks prove the stated
data/input boundaries. Native build parity, resource reservation, conversion receivers,
additional skills and minion actors still require their own differential validation.
