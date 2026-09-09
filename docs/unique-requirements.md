# Native unique requirements

The `unique_requirements` section supplies immutable item-data facts to native loading.
Its boundary is independent of the calculation backend: the native consumer reads a
selected `GameDataSnapshot`, while an optional source exporter and independent parity
suite can load Path of Building. Runtime lookup needs no Lua, subprocess, filesystem
access or constructed build. This is part of the [game-data boundary](game-data-boundary.md)
and [ordered item loader](item-source-and-loading.md).

## Data and availability

Package schema 18 adds the section without changing the preceding 26 sections. The
section has two explicit states:

- `unavailable` retains a reason and cannot answer whether an item exists.
- `complete` contains every prototype outcome, the resulting unique requirement entries,
  lookup policy, construction inputs and source provenance. A missing key here is a
  confirmed miss.

Entries retain the canonical key, constructed base identity and optional natural and
current level requirements. At least one requirement must be present; stored numbers
must be finite. Absence is distinct from numeric zero, including negative zero. The
original raw prototypes remain in `item_loading`; finished requirements are not inferred
by scanning those strings or by consulting a hard-coded item list.

Complete input accounting includes prototypes skipped because their base is absent.
Every insertion maps to exactly one final entry. Duplicate canonical keys and potential
cross-entry database reads during construction reject this representation, because they
can make requirements depend on insertion order. These restrictions apply to the
construction model, not to a user's choice of item names or numerical objectives.

## Native lookup and composition

`GameDataSnapshot::unique_requirements()` returns a shared immutable catalog. Lookup
first checks the exact canonical name. Only after a miss, and when both title and base
name are present, it tries the configured base prefixes in order. The first matching
prefix with a nonempty suffix is removed once. The fallback key is the title, configured
separator and remaining base name. Lookup is exact and case-sensitive; it does not trim,
fold case or repeatedly remove prefixes. An empty present title retains Lua truthiness.
The catalog uses binary search and borrowed string components, without allocating a
fallback key.

The item-loading state machine owns rarity and base eligibility. It prefers a present
natural requirement over the stored current requirement; nil permits fallback, whereas
numeric zero does not. It then combines that requirement with the current base level,
preserves a higher authored level, and includes the current rune requirement in source
order. Its maximum operation preserves Lua comparison order, including signed zero and
non-finite values supplied through an explicit provider. A missing required operand is a
source error, with the state established before that error retained in the loading report.
The portable data catalog separately rejects non-finite stored requirements.

`BuiltinItemLoadProvider::new(snapshot)` uses the selected native catalog. Library hosts
can also use `NativeItemLoadProvider::with_native_unique_lookup(snapshot, dependencies)`:
the catalog answers unique lookups, while the explicit dependency provider supplies other
operations. A catalog miss or unavailable state never falls through to that provider.
`with_dependencies(snapshot, dependencies)` preserves fully explicit provider selection
for unique lookup, allowing an independently selected parity/reference implementation.
Custom provider provenance remains the host's responsibility.

## Construction and trust

The optional exporter runs the original unique-database construction loop to completion,
including original item constructors, parser, stored modifier cache, defaults and tree
inputs. It observes the loop without substituting sorted construction. Canonical export
ordering is applied after construction. An outcome ledger binds each original group/index
and raw prototype digest; source files and relevant construction spans remain recorded.
The completed/loading-cleared state must be established before publishing the section.

The package loader validates bounded records, full input accounting and cross-section
identity. A complete section binds its item-loading and tree inputs; stale identities
reject. When those inputs change, regenerate the completed requirements or explicitly
supply an unavailable section. Resealing digests alone is not evidence that constructors
ran or that custom values match Path of Building. Caller-authored supported records
remain possible under the package's custom/unreviewed trust policy.

The catalog bounds prototype/entry collections to 50,000, cumulative text to 8 MiB and
individual lookup/source fields separately. It is subject to the enclosing package's
portable size/value limits. Offline construction has its own supervised process and VM
limits, described in [game-data extraction](game-data-extraction.md#isolation-and-limits).

## Parity and remaining work

Validation must compare completed records and native consumer transitions with independent
execution of original source. It includes all shipped entries, exact/fallback lookup,
missing and unavailable data, original constructor traversal and cache states, and
deliberate duplicate-key/cross-entry counterexamples. Numerical witnesses distinguish
absent, zero, signed zero, authored requirements and error prefixes. Custom-data checks
prove selection and isolation; they do not establish source parity for arbitrary edits.
Current results and resume points belong in the [implementation log](implementation.md).

This section provides reusable unique-item requirement facts. It does not implement full
native construction from raw prototypes, selected modifier callbacks, crafted affixes,
runes or final item assembly. Those operations retain separate native capability and
parity gates. General complete-build evaluation, additional actors/skills and global
optimization still require the remaining implementation phases.
