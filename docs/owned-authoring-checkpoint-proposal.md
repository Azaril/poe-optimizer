# Persisting named allocation with a compiled release

Status: accepted by the owner, 2026-10-23. This decides the offline publication workflow;
it does not change the native build, schema, rule or evaluation model.

The current release builder publishes a closed runtime directory. Runtime loading
rejects extra files. `NamedIdRegistry` stages a successor in memory, with an exact
predecessor check, but its name ledger is not yet persisted by that builder.
New declarations need stable names across later compilations. Recreating an empty
ledger on each build would let a newly added, earlier-sorting name shift existing
identities. Ordinary alias files do not preserve allocation history.

## Accepted workflow

Publish one immutable **authoring checkpoint** containing the registry-bound name
ledger and a separate runtime-release directory. Construct and validate the whole
release before committing either. Publish the outer checkpoint atomically into a
new directory; runtime consumers receive its runtime directory and retain their
existing strict inventory. Advancing a mutable authoring head requires the planned
compare-and-swap check against the actual stored predecessor. Failed compilation,
stale predecessors and failed publication leave the previous checkpoint intact.

The next compile reads that explicit checkpoint as its allocation predecessor.
Reproducing a release uses its recorded source inputs and predecessor. Historical
runtime releases remain immutable. No runtime dependency on authoring names is
introduced, and no compatibility wrapper is needed for the unshipped CLI.

This keeps allocation and successful release construction in one transaction.
Its cost is an explicit authoring output layout and head-promotion operation.
The host needs a checked nested publication primitive; the existing basename-only
artifact publisher must not be bypassed with unchecked paths.

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
  remains an immutable compiler operation and can proceed before this decision.
- Check incoming bootstrap commitments before constructing new dependencies.
  The current release assembler checks final bindings; it does not repair them.
  The old revision helper assumes a checked predecessor and cannot safely be
  called on raw source constituents or revived as a parallel publication path.
- Reuse shared semantic validation and dependency construction in the maintained
  Authoring pipeline. Do not add an independent list that silently repairs stale
  or corrupt runtime seals. Evaluation-artifact dependencies need explicit support.
- Test stable IDs after adding earlier-sorting names, stale/mismatched snapshots,
  malformed bootstrap commitments, late compilation failure, source/symbol
  conflicts, no-clobber publication and interrupted/failed head promotion.

Neither publication workflow supplies missing gameplay facts, promotes partial
coverage, or establishes a complete original build.

Implementation checkpoint, 2026-10-23: the shared original-source dependency check
is implemented and used before DSL lowering. It now retains the checked source
registry/schema, routing and Import owners for consuming content extraction, plus
the bounded rule payload with its checked original header. The maintained compiler
stages declarations from explicit persisted names, reconstructs rule/routing and
Import dependencies, then performs mandatory final rule/evaluation validation.
Successful in-memory results retain successor names and their expected predecessor;
the CLI still accepts no allocating declaration context. The compiler also
constructs evaluation dependencies from explicit authored content against the
final checked recipe. Sealed runtime fields are rejected in those source files;
inherited content must come from the actual fully checked evaluation owner.
The workflow is approved; atomic nested publication, loading the explicit ledger
predecessor and CLI integration remain implementation work. Include packet/source
context in declaration diagnostics before exposing allocating CLI inputs.
