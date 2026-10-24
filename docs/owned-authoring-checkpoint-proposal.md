# Persisting named allocation with a compiled release

Status: accepted by the owner and implemented, 2026-10-23. This decides the offline
publication workflow; it does not change the native build, schema, rule or
evaluation model. Mutable-head promotion remains deferred. The owner reaffirmed
this choice on 2026-10-24; an independent implementation review found the current
CLI, checkpoint loader and Host publisher consistent with the accepted contract.

The release builder publishes a name ledger and a separate closed runtime
directory as one immutable checkpoint. `NamedIdRegistry` stages its successor in
memory with an exact predecessor check. Persisting that history ensures that
adding an earlier-sorting declaration does not shift existing identities.
Ordinary alias files do not preserve allocation history.

## Accepted workflow

Publish one immutable **authoring checkpoint** containing the registry-bound name
ledger and a separate runtime-release directory. Construct and validate the whole
release before committing either. Publish the outer checkpoint atomically into a
new directory; runtime consumers receive its runtime directory and retain their
existing strict inventory. Advancing a mutable authoring head requires the planned
compare-and-swap check against the actual stored predecessor. Failed compilation,
stale predecessors and failed publication leave the previous checkpoint intact.

The next compile reads an explicit checkpoint as its allocation predecessor.
Reproducing a checkpoint uses the same source snapshot and the same predecessor
as its original build. The receipt records source hashes, not the source files;
retain the manifest and input files separately. Historical runtime releases remain
immutable. No runtime dependency on authoring names is introduced.

This keeps allocation and successful release construction in one transaction.
Its cost is an explicit authoring output layout and predecessor selection. Host's
bounded two-level artifact publisher validates file/directory basenames and
collisions, stages privately, and performs one atomic no-replace rename. This
guarantees atomic visibility; it does not guarantee durability after sudden power
loss. No mutable head is introduced by this implementation.

## Current commands and layout

For the maintained production history, first extract and verify the snapshot in
the [canonical reference](../data/owned/poe2/3887ae68/authoring-genesis.md), then
build against that exact predecessor:

```powershell
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --predecessor canonical-checkpoint --expected-predecessor 71ff4429b7fcfb4f75e45d9c58df878453cb25428542d3531074fc08fccbba91 --output next-checkpoint
```

Use `--bootstrap` only to start an independent allocation history. It must not
reconstruct the maintained ledger from evolving source. Every release build
requires either explicit bootstrap or a predecessor path and
its expected checkpoint identity. It emits receipt JSON on stdout and the new
checkpoint identity on stderr after successful publication. The output contains
only `checkpoint.json`, `names.json`, and `runtime/`; runtime consumers receive the
last directory. A failed predecessor load never falls back to bootstrap.

To validate a transported checkpoint independently of its sources and ancestors:

```powershell
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release verify canonical-checkpoint --expected 71ff4429b7fcfb4f75e45d9c58df878453cb25428542d3531074fc08fccbba91
```

Supply the expected identity of the directory being verified from a maintained
reference. The command uses the same bounded loader as `release build` and emits
compact JSON containing `checkpoint`, `names`, `runtime_input` and
`runtime_registry` only after full validation. It writes no files. A valid receipt
alone is insufficient: the exact ledger and complete runtime inventory are required.
This verifies a snapshot, not its source reproducibility or any build's coverage.
An archive transports the unchanged directory. Each new canonical allocation
generation must publish another complete checked snapshot; the current snapshot
includes 73 names, preserving the prior 70 and appending three revival channels.

An optional `stat-declarations "definitions.kdl" names="declaration-names.kdl"`
manifest record uses the existing bounded source decoder, KDL declaration parser
and typed name loader. Declaration lookup aliases remain local; packets receive
only current declared Stat names alongside their own aliases. Packet conflicts
and declaration failures include the relevant source paths. No declaration record
means an empty declaration set with retained allocation history, including reserved
names. It does not discard the predecessor registry or export every persisted name.

The library's `ReleaseAllocationBase` distinguishes `Bootstrap` from
`Predecessor { names, expected_names }`. Every successful `BuiltRelease` returns a
`ReleaseNameAllocation`; its predecessor name identity is absent only for bootstrap.
`build_authoring_checkpoint` accepts an explicit `CheckpointBase`, records exact
source commitments, and returns the staged receipt, ledger and runtime artifacts.
`load_authoring_checkpoint_artifacts` requires the expected checkpoint identity,
checks bounded canonical receipts and names, and revalidates the complete runtime.
The runtime registry is the only registry snapshot: `NamesInput.registry` must
equal its identity. The ledger identity is a separate commitment.

## Canonical allocation snapshot

The first production declaration slice established one genesis checkpoint with
69 allocated Skill Stats. The support-prohibition checkpoint preserves those
bindings and appends the seventieth. The current revival checkpoint preserves all
70 and appends three generic Skill/Actor Stats. The same `mechanics.kdl`
declaration component and atomic publication transaction own these allocations;
no parallel ledger or allocation step is introduced. The earlier empty-ledger checkpoint is
verification evidence, not allocation authority. The [canonical reference](../data/owned/poe2/3887ae68/authoring-genesis.md)
pins a complete immutable snapshot and its checkpoint/archive identities outside
the compiled manifest inventory, avoiding a self-referential receipt.

The initial fixed-source rebuild bridge has been replaced by that snapshot.
A clean clone extracts it and validates the exact expected identity through the
normal checked checkpoint loader. No historical compiler, ancestor checkpoint,
source checkout or machine-local ledger is needed. Standard archive transport
wraps the existing complete checkpoint directory and adds no semantic format.
The current snapshot and its checked source replay preserve all eighteen runtime
files and the name ledger byte-for-byte. The [revival component evidence](../data/owned/poe2/3887ae68/revival-producers/README.md)
keeps incoming coverage and final duration unresolved; publication success does
not promote those partial mechanics or complete any original build (0/5).

Current source builds use the checked snapshot as their explicit predecessor.
Never bootstrap evolving source to reconstruct allocation history or substitute
an empty ledger: earlier-sorting additions could then change assigned IDs.
Identical source can preserve runtime and name identities while its checkpoint
identity changes to record a predecessor. A storage-only cutover must preserve
name allocations and semantic runtime commitments while deliberately regenerating
artifact hashes and the canonical snapshot. Old readers and compatibility
compilers are not retained to reproduce superseded storage.

Do not grow a recursive source-replay chain or a separate ledger importer. Each
new canonical allocation predecessor must be available as one complete checked
snapshot. Mutable-head promotion remains a separate deferred task.

## Considered alternative: explicit source-ledger allocation

Use a separate command to allocate and atomically update one registry/name ledger
beside the authoring sources. Release builds only read that ledger and fail if a
declaration is not allocated. This keeps the existing release output shape and
makes ordinary builds read-only. It adds an allocation step before compilation;
the allocation command must define how much semantic validation precedes a ledger
commit, and failed later compilation can leave reserved but unused identities.
Both workflows can preserve stable IDs and deterministic reproduction.
The owner selected atomic authoring checkpoints; do not implement the separate
allocation command as another supported path.

## Shared requirements and separate implementation work

- Use the existing allocator and generic typed Stat schema. Declaration staging
  remains an immutable compiler operation.
- Check incoming bootstrap commitments before constructing new dependencies.
  The current release assembler checks final bindings; it does not repair them.
  The old revision helper assumes a checked predecessor and cannot safely be
  called on raw source constituents or revived as a parallel publication path.
- Reuse shared semantic validation and dependency construction in the maintained
  Authoring pipeline. Do not add an independent list that silently repairs stale
  or corrupt runtime seals. Evaluation-artifact dependencies are constructed from
  authored content against the final checked recipe.
- Test stable IDs after adding earlier-sorting names, stale/mismatched snapshots,
  malformed bootstrap commitments, late compilation failure, source/symbol
  conflicts, zero-declaration history, same-limit publication/loading, closed
  inventories and no-clobber publication. Before adding a mutable head, implement
  actual stored-predecessor compare-and-swap and interruption/concurrency tests;
  read-compare-rename alone is insufficient.

Neither publication workflow supplies missing gameplay facts, promotes partial
coverage, or establishes a complete original build.

Implementation checkpoint, 2026-10-23: the shared original-source dependency check
is implemented and used before DSL lowering. It now retains the checked source
registry/schema, routing and Import owners for consuming content extraction, plus
the bounded rule payload with its checked original header. The maintained compiler
stages declarations from explicit persisted names, reconstructs rule/routing and
Import dependencies, then performs mandatory final rule/evaluation validation.
Successful in-memory results retain successor names and their expected predecessor,
including when there are no new declarations. The compiler also
constructs evaluation dependencies from explicit authored content against the
final checked recipe. Sealed runtime fields are rejected in those source files;
inherited content must come from the actual fully checked evaluation owner.
Atomic nested publication, checked explicit-predecessor loading and allocating CLI
integration are implemented. Mutable-head promotion is separate future work.
That compiler checkpoint did not add production gameplay declarations or complete
the first original build. Subsequent declarations add initial Skill types, support
prohibition and revival producers, while retaining Partial rule coverage and the
same incomplete-build admission result. The three revival queries resolve their
new Stat names within that same compile; their source census and final consumer
remain follow-up work.
