# Named contribution-query authoring

Status: implemented; retained source migration and real publication verified,
October 24, 2026.

Contribution queries describe which exact effects may enter a calculation, how
they are grouped and reduced, and which coverage is still unknown. Their runtime
model is unchanged. Named KDL sources resolve to the existing checked query DTOs
after Stat allocation, alongside DSL programs, in the same atomic authoring build.
This removes the need to predict IDs or publish a ledger before authoring queries.

The maintained release manifest accepts bounded source fragments:

```kdl
contribution-queries "queries/resources.kdl" names="names/resources.kdl"
```

Each source has one `queries` root with the schema's `game`, `version` and explicit
`coverage`. Each `query` names its Stat and contribution kind; each `group` names
its reduction, ordering and membership coverage. Empty-fold values are explicit
and typed. Omitting an empty value preserves absence, including numeric selection;
it never invents zero or false. Program producers retain exact owner, program,
effect and source-origin authority. Application-group producers retain their
post-stacking family, modifier and complete declaration census. Ordered folds
retain all source/program/effect and equipment-slot ranks. Partial inventories
carry exact subject/facet/code gaps.

Source aliases resolve against the final schema. A query source receives only
its own aliases and the current Stat declarations, through the same checked
overlay used by program packets. Namespace mismatch, ambiguous or wrong-kind
symbols and conflicting aliases fail. Persisted names that are not declared in
this build do not become implicit lookup authority.

Fragments append in manifest order without deduplicating members. Duplicate query
IDs fail across the complete source set. All partial gaps remain in order; any
partial fragment keeps the combined inventory partial. Complete fragments do not
erase uncertainty from another fragment or from the incoming empty inventory.
An absent query inventory remains distinct from an explicit complete empty one.
The original captured constituent may retain an empty inventory for declarations
of coverage, but may not contain executable query members, even when no KDL query
source is supplied. There is no captured-JSON fallback after a source error.

The shared official KDL parser enforces its existing offline byte and retained
document limits. Every fragment counts against release source/component and
encoded/decoded byte budgets. Aggregate query, group, member, binding and gap
counts are checked before fragments are appended. The existing final storage and
Engine validators still check the complete graph, channels, units, reducers,
origins, recipients, membership, order and work budgets. KDL parsing alone grants
no numerical coverage or permission to execute an incomplete build.

The migration moves all 114 retained queries, their 120 groups and 2,117 members
to named source files. Exact DTO and runtime-byte preservation are verified,
including the partial outer inventory and 45 partial groups. The canonical
allocation snapshot remains valid because this source-format change adds no
allocations or runtime semantics. The maintained historical corpus is independent
evidence; a converter or previous source layout is not a normal build dependency.

The first subsequent consumer is revival. Its source review and missing contributor
authority remain separate work: a symbolic query can represent a new Stat without
establishing that all gameplay sources have been converted. See the
[implementation plan](implementation.md) for the current validation and resume point.
