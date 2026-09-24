# Owned data release assembly

`assemble-owned-release` builds one complete, immutable data package in Rust. It is an
offline authoring operation: no character build, PoB checkout, Lua runtime or numerical
evaluation is required. Native evaluation keeps consuming the existing owned runtime
artifacts and receives no new Core model or operation.

## Release and successor contracts

A full release validates the authored endpoint independently. It can correct an earlier
schema declaration or contain explicitly authored rule changes; it makes no compatibility
claim for builds pinned to older data. The separate successor APIs retain their stricter
preservation rules. V3 cannot change a Complete set, and V4 cannot rewrite a Known Gem.
There is no switch to disable those checks.

`OwnedReleaseInput` combines the runtime recipe, mapping, roles, normalization, rewards,
item-line/source policies, optional tree policy and ordered named query sets. Every supplied
dependency binding must already match the supplied endpoint. Normal assembly rejects stale
bindings rather than repairing them. Existing constituent constructors validate and compile
all content, and aggregate limits bound bytes, entries, queries and repeated policy/query
commitments before publication. Source footprints must have the same system/revision and
matching hashes wherever they overlap; the receipt records their deterministic union.

## CLI

The input can be a complete authored JSON document, a checked V1/V2 successor directory,
or an existing release directory. Publish to a new destination:

```text
poe-optimizer assemble-owned-release path/to/authored-release.json --output runs/my-release
poe-optimizer assemble-owned-release runs/prior-package --output runs/rebuilt-release
```

The host validates directory membership, safe artifact names, sizes and SHA-256 hashes,
then reconstructs the input and reproduces every canonical artifact byte. A malformed
`release.json` cannot cause fallback to an older package parser. Supplied JSON may use
noncanonical descriptor order; the output uses the constituent constructors' canonical
representation. Ordered query sets and query rows retain their authored order in every
input format. Filesystem publication uses the existing atomic no-clobber operation.

Runtime files remain `registry.json`, `schema.json`, `rules.json`, `routing.json` and their
recipe manifest. Separate files carry import policies, source metadata and queries. The
new `release.json` commits the canonical full input, all constituent identities, source
footprints, ordered query counts and every emitted artifact's hash/size except itself.
It contains no duplicated full recipe and establishes no numerical parity. A directory
reader verifies the receipt itself by reproducing its exact bytes after validation.

Existing offline compiler commands accept full releases through the shared checked loader.
They may still emit successor publications with their existing contracts; full-release
assembly can consume those outputs. The loader distinguishes release V1 from legacy
successor V1, so the new format never selects legacy duplicated-recipe behavior by accident.

## Explicit schema corrections

A reviewed correction can be applied while assembling a new release:

```text
poe-optimizer assemble-owned-release runs/prior-package --revision path/to/revision.json --output runs/corrected-release
```

`OwnedReleaseRevisionInput` contains an exact prior canonical input commitment, a new
release name, a reason, and complete replacement descriptors for existing definition/slot
addresses. Targets are unique and canonical. Empty changes, unchanged targets, unknown
addresses, stale endpoints and reuse of the old release name reject. The correction does
not allocate, retire or reuse identities, alter the registry watermark, or change unrelated
rules and input policies. New allocations continue through the existing compiler APIs or
an explicitly authored complete input.

The compiler takes a fully validated prior release, checks the correction and tighter
resource limits before cloning/indexing, applies the named data changes, recompiles the
runtime recipe, and rebinds dependent artifacts in dependency order. It then runs the same
full-release validation. Query content and source evidence are preserved. Invalid revised
schemas or policies fail before any output is published; old packages remain untouched.

A provenance record commits the prior canonical input and exact revision policy. Standalone
authored provenance is a declaration, not authenticated ancestry. The correction compiler
establishes its record by checking the actual prior input; consumers must not infer stronger
history or numerical coverage merely from a manifest's presence. Historical successor
formats keep their existing metadata contract; retain the authored correction and immutable
parent publications for reproducibility.

The first correction reopens the prematurely Complete-empty direct parameter and choice
collections of the early Twister/Skeletal Sniper data. It preserves all known members,
grants and missing-reference gaps. This restores accurate Partial knowledge; it does not
supply those remaining inputs, activate a support, or complete a build evaluation. See the
[implementation resume](implementation.md) for actual publication and validation evidence,
and the [migration plan](architecture-migration.md#full-data-release-assembly-before-further-active-gem-integration)
for active-Gem and paired-action follow-through.
