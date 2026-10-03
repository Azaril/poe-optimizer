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

V1 runtime files are `registry.json`, `schema.json`, `rules.json`, `routing.json` and their
recipe manifest. Separate files carry import policies, source metadata and queries. The
new `release.json` commits the canonical full input, all constituent identities, source
footprints, ordered query counts and every emitted artifact's hash/size except itself.
It contains no duplicated full recipe and establishes no numerical parity. A directory
reader verifies the receipt itself by reproducing its exact bytes after validation.

Existing offline compiler commands accept V1 full releases through the shared checked loader.
They may still emit successor publications with their existing contracts; full-release
assembly can consume those outputs. The loader distinguishes release V1 from legacy
successor V1, so the new format never selects legacy duplicated-recipe behavior by accident.

## Evaluation releases

Release V2 adds an explicit `evaluation` group. Its metric mapping is mandatory; an optional
support group contains all four stages, preparation, input and receiving artifacts, plus
optional final-type outputs. A metric-only release uses the ordinary evaluator. Support
mode follows the declared group, never the presence of files guessed from a character build.
V1 rejects evaluation groups and retains its original canonical bytes and input digest
domain. V2 requires the group and uses a separate input digest domain.

Assembly validates these artifacts against the exact owned schema and stored rule package
in dependency order. The full-input commitment and receipt bind every constituent identity.
Runtime files are `metrics.json` and, for support mode, `support-stages.json`,
`support-preparation.json`, `support-inputs.json`, `support-receiving.json` and optional
`support-outputs.json`. The existing `mapping.json` remains the source import mapping.
Aggregate and constituent resource limits apply before publication. Missing or stale
packages, extra directory files and unsupported versions reject; complete reassembly must
reproduce every canonical byte, including the receipt.

```text
poe-optimizer evaluate-owned --input path/to/request.json --release runs/my-evaluation-release
```

This is mutually exclusive with manually supplied schema/rule/routing/metric/support paths.
The CLI consumes the validated in-memory package snapshot without reopening its files and
uses the same native effect/metric plans as the explicit-file path. The request remains a
separately supplied owned document; release publication cannot manufacture missing build
inputs, contributor coverage or numerical parity. V1 packages have no metric mapping and
cannot be selected with this evaluation option.

V2 can be republished unchanged through `assemble-owned-release`. Migration V2 supplies a
complete replacement evaluation group for its exact endpoint, as described below. Schema
revisions, migration V1 and legacy successor compilers still reject evaluation-bearing
inputs. They cannot silently discard the group or guess classifications while changing
dependent identities. A fully authored V2 endpoint with exact bindings also remains valid
input to ordinary release assembly.

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

## Explicit contract migrations

Schema v3 actor supply needs new allocations and corrections to earlier declarations
in one transaction. `OwnedReleaseMigrationInput` names the exact previous full-input
commitment, a new release, explicit endpoint contract versions, canonical schema rows,
ordinary rule additions and optional patches to existing query targets. The v1 migration
targets schema v3 and rule operations v11; unsupported or future versions reject.

```text
poe-optimizer assemble-owned-release runs/prior-package --migration path/to/migration.json --output runs/migrated-release
```

`--migration` and `--revision` are mutually exclusive. Existing descriptor addresses can
be explicitly replaced, while new descriptors must match the next typed registry
allocations in order. Identities cannot be retired or reused. All schema changes are
validated together, allowing a corrected actor slot to reference its new Actor definition.
Existing tables, programs, receivers and coverage closures cannot be rewritten by rule
additions. Query patches change only named existing targets; query IDs, metric selectors,
set order and row order survive unchanged.

The compiler bounds the prior input and migration before cloning, rebinds dependencies
through the same validated sequence as schema revisions, and validates the complete
endpoint before atomic publication. Provenance commits both inputs. Normal assembly,
V1 schema revisions and monotonic successor checks retain their existing contracts.
Neither publication nor a contract upgrade establishes numerical coverage.

Migration V2 extends the same input with a mandatory `evaluation` group and targets schema
v4 / rule operations v13. It accepts checked prior releases with schema v2–v4 and explicitly
supported operations v6–v13. The staged predecessor must already validate its own contracts.
V1 keeps its prior support matrix, wire bytes and authoring digest domain; V2 has separate
authoring and combined-budget domains. The CLI continues to use `--migration` and rejects
unsupported version/group combinations rather than falling back to another format.

The supplied evaluation packages must already name the exact new schema, stored rules and
dependent artifact identities. The compiler rebinds inherited source-import policies using
the existing checked sequence, then validates the supplied evaluation group unchanged. It
does not repair stale evaluation hashes, reuse a predecessor's group implicitly or assign
program stages. Upgrading a V1 release or replacing a V2 group is explicit authored data;
the resulting full release is V2.

Both prior and authored evaluation data share the migration entry budget, including nested
predicates, paths and memberships, before recursive hashing or cloning. The combined byte
ceiling includes the full prior input and migration; the CLI also charges both files against
one read allowance. New allocations, ordinary rule/table contents, coverage closures and
ordered-query preservation retain the original migration checks. Provenance commits the
complete predecessor and exact V2 authoring input. Complete packages and successful native
component execution do not certify source conversion or original-build parity.

Migration V3 targets schema V5 / operations V17 for typed inputs on exact Skill
occurrences. Its reviewed predecessors use schema V4 or V5 and operations V15,
V16 or V17; the predecessor must independently validate that combination. V3 has
its own authoring and combined-budget digest domains. V1/V2 keep their previous
version matrices and canonical behavior.

A V3 migration may omit evaluation artifacts only if its predecessor has none.
An existing evaluation group, support group or support-output group cannot be
dropped. Supplied endpoint groups still require exact new identities and pass the
ordinary stage/dependency checks unchanged. Inherited Direct-input policies may
rebind their definition and role identities after predecessor validation; their
source/catalog commitments, source recipes and slot authority are not repaired.
Explicit policy replacements must already validate against the new endpoint.

The first authored V3 endpoint adds raw inputs for manual Sand/Water Djinn skills.
It retains Partial inventories and has no evaluation bundle. See the
[occurrence-input contract](owned-skill-occurrence-input-proposal.md) for input
authority and [implementation status](implementation.md) for its publication gate.

The next V3 endpoint stays on V5/V17 and adds Frost Bomb's physical primary
supply and requested global-effect preference through the existing ordinary rule
and usage-policy paths. It preserves inherited Direct policies and all 110
queries. Its PhysicalV2 extension closes only source inventories with actual
typed scalar/usage records; mechanics, preset usage and final-input coverage are
still incomplete. The source receipt records cold and normal-rebuild observations
without choosing a canonical parity lifecycle. See the
[source evidence](owned-frost-bomb-usage-evidence.md) for that distinction.

The Sniper count/reservation V3 endpoint also stays on V5/V17. It adds one typed
usage parameter and two ordinary programs, reusing the existing intrinsic
reservation table and producer. UsageV2 preserves the prior Boolean rows and
adds a finite numeric source recipe; it changes no physical-inventory admission.
Publication requires matching complete-source reports from both JIT modes and
authenticates the catalog's negative effect evidence for reviewed companion
gems. Final-input and modifier producers remain unresolved, and no evaluation
bundle or hidden parent-action query is added. See the
[reservation contract](owned-summon-reservation.md) and current publication
receipt in [the implementation plan](implementation.md).
