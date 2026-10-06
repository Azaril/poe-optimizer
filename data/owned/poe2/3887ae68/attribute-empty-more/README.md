# Guarded empty attribute MORE domains

This packet implements the six existing effective-MORE producers (Stats
`3324`–`3329`) for an explicitly checked empty incoming domain. Each producer
reads a native ordered `Multiply` query with a `Product` reduction and identity
Factor 1. The group has a complete, empty membership inventory. The program
contains only the query read and the resulting stat write; it has no literal
factor node or missing-input fallback.

The candidate binder checks every potential matching contribution against all
query memberships before evaluating guards or values. Any potential matching
effect rejects the empty domain, including disabled, identity-valued,
zero-valued and unresolved effects. Inactive loadouts excluded by ordinary
occurrence discovery remain distinct from a discovered effect whose guard is
false. This packet does not change either behavior.

Six producers, six Player receivers and six queries are the only runtime
changes. No IDs, definitions, source admission, input ownership or other rule
programs change. The global query registry and twelve BASE/INC membership
inventories remain Partial. Missing or Partial providers still make the checked
evaluation unavailable; complete-build progress remains **0/5**.

This is not a claim that MORE modifiers do not exist. For example, the pinned
source has Enhanced Effectiveness (tree node 58591, owned node `18d9`) with less
Attributes; its native owner remains Partial. Nonempty grouping and rounding
remain unresolved and are outside this packet's supported incoming domain.

## Evidence and validation

The existing `owned-attribute-pipeline-source-05` witness observes original
attribute calls in fresh, repeated, restored and independent unhooked loads,
with byte-identical JIT-on/off reports. Its Original05 and exact attribute-choice
edit have no MORE records (including no zero-valued records), a single store at
depth zero (the original ModStore's explicit `false` parent marker), and actual
original MORE returns of 1 for both passes and both saved
selectors. The inherited source projection and full reports are authenticated
by the existing step-consumer helper. `source-vectors.json` retains only the
empty-domain projections and their report identities. Runtime artifacts contain
the authored queries and programs, not the source evidence.

The native tests reuse the existing finite fixture with the actual Original05
class and 22 selected attribute-choice occurrences. They replace its former
test-only factor producers with these published producers and preserve exact
contribution keys, targets and inactive choice effects. The six existing step
consumers produce 27/7/105, then 22/12/105 after changing node 15782 from Strength
to Dexterity. This finite fixture closes only its known test inventories; it
does not close the production class, query registry or incoming memberships.

Separate controls check every stage with enabled and disabled potential MORE,
zero and identity amounts, and missing effect input. Missing factor producers
and actual Partial class/query coverage retain their respective unresolved or
unavailable outcomes. Repeated evaluation, candidate restoration and Rayon
workers must agree exactly.

Publication follows `runs/owned-attribute-step-02/package` and uses the existing
checked full-release assembler. Its inverse restores the entire predecessor
input exactly, including owner positions and closures, query inventories,
receivers, schema, imports, stock and provenance. The shared publication check
rebuilds the 18 runtime artifacts and verifies all five normalized builds and
110 query rows without changing selected inputs.

## Reproduction

Build the `owned_attribute_empty_more` test target through the root's serialized
Cargo workflow. Its authored test needs no environment. Publication uses
`POE_OPTIMIZER_TEST_ATTRIBUTE_EMPTY_MORE_PRIOR` and a fresh
`POE_OPTIMIZER_TEST_ATTRIBUTE_EMPTY_MORE_OUTPUT`, filtered to
`publish_empty_more_preserving_all_five_originals` with `--ignored --exact`.

The four `native::` tests need `POE_OPTIMIZER_TEST_ATTRIBUTE_EMPTY_MORE_RELEASE`
pointing to the new package, `POE_OPTIMIZER_TEST_ATTRIBUTE_STEP_RELEASE` pointing
to the verified step predecessor, and `POE_OPTIMIZER_TEST_ATTRIBUTE_COUNT_RELEASE`
pointing to the immutable Count fixture package. Run the `native::native_`
filter with `--ignored` to exclude inherited fixture tests.

## Verified publication (2026-10-06)

`runs/owned-attribute-empty-more-01/package` has input
`ef1abf90ee6aa7a0ec3a975ccde015f9e5b1a2a95d22d6daa3af1c3ffcd7c159`.
The authored check passes; publication passes in 21.59 seconds and all four native
checks pass in 11.79 seconds. Its eighteen files rebuild byte-identically, all
five unchanged inputs retain their exact selections and 110 queries, and no
whole-build evaluation is admitted. Fresh imports use current sidecar21; the
separate generated accounting replay proves the importer cutover against stored
older output. Logs are `runs/owned-attribute-empty-more-{authored-02,publication-01,native-01}.log`.
The first authored run caught a mistaken assertion of an absent source parent;
the witness correctly retained PoB's explicit false sentinel. Only that offline
assertion was corrected; runtime guards and authored data were unchanged.
