# Persisting named allocation with a compiled release

Status: accepted by the owner and implemented, 2026-10-23. This decides the offline
publication workflow; it does not change the native build, schema, rule or
evaluation model. Mutable-head promotion remains deferred.

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

The bootstrap command illustrates starting a new history. For the maintained
production history, run it only against the fixed genesis source described below;
use the predecessor form for evolving source.

```powershell
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --bootstrap --output first-checkpoint
$checkpointIdentity = "<checkpoint identity printed by the previous build>"
cargo run --locked -p poe-optimizer-authoring --bin poe-author -- release build data/owned/poe2/3887ae68/migration-source/release.kdl --predecessor first-checkpoint --expected-predecessor $checkpointIdentity --output next-checkpoint
```

Every release build requires either explicit bootstrap or a predecessor path and
its expected checkpoint identity. It emits receipt JSON on stdout and the new
checkpoint identity on stderr after successful publication. The output contains
only `checkpoint.json`, `names.json`, and `runtime/`; runtime consumers receive the
last directory. A failed predecessor load never falls back to bootstrap.

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

## Canonical first-allocation genesis

The first production declaration slice establishes one genesis checkpoint
containing its 69 allocated Skill Stats. The earlier empty-ledger checkpoint is
verification evidence, not that genesis. Record the exact Git source revision,
manifest path and expected checkpoint identity in a maintained reference outside
the compiled manifest's source inventory. Record those values after the validated
content commit, in a follow-up reference/documentation commit; the manifest must
not refer to its own future Git commit or checkpoint identity. The exact reference
is pending validation and publication of the first declaration slice.

A clean clone reproduces genesis by obtaining that fixed source revision and
running the maintained compiler with explicit `--bootstrap` against its manifest.
Check the resulting checkpoint identity against the reference, then pass the
artifacts through the normal checked checkpoint loader. The referenced source
revision supplies data; it does not select a historical compiler or migration
helper. CI must make that exact revision available even when its initial checkout
is shallow. No pre-existing `runs/` output or machine-local ledger is required.

Current source builds use the fully checked reproduced genesis as their explicit
predecessor. Never bootstrap the evolving source to reconstruct its allocation
history, and never substitute the earlier empty ledger: either would permit
earlier-sorting additions to change assigned IDs. Rebuilding identical source with
genesis as predecessor can preserve runtime and name identities while changing the
checkpoint identity, because the receipt now records that predecessor. Reproducing
the genesis receipt itself still requires the pinned source and its original
explicit bootstrap operation.

This source reference is a bounded bridge to R3 distribution. Before a second
allocation generation needs a new canonical predecessor, archive or distribute
one full, checked immutable checkpoint snapshot through R3. Do not extend this
recipe into recursive historical source builds, retain a growing chain of duplicate
working-tree snapshots, or introduce a separate names-ledger importer. The next
build must be able to load its allocation predecessor directly. If compiler or
artifact formats change, deliberately regenerate or migrate the canonical snapshot
and its reference; do not add compatibility compilers or fallback loaders to keep
the genesis recipe alive. Mutable-head promotion remains a separate deferred task.

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
the first original build. The subsequent first-genesis slice adds the initial
Skill type declarations described above, while retaining Partial rule coverage
and the same incomplete-build admission result.
