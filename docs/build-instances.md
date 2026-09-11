# Owned build source and authored instance identities

R1a adds an immutable source-owning import model and portable identities. It is the first
slice of the [shared build contract](real-build-api-proposal.md), not a complete selected
view, effective actor/action graph or native build evaluator. The
[implementation record](implementation.md) tracks validation and remaining R1-R5 work.

## Inspect a caller build

```powershell
poe-optimizer inspect-build "your-build.xml" --with-instances
poe-optimizer inspect-build "your-build.import.txt" --with-instances --with-definitions
```

The command accepts one caller-supplied XML document or share code. `--with-instances`
adds an `instances` report with its own schema version 1; the existing inspection schema
and source/definition reports retain their behavior. The host obtains a fresh 128-bit
lineage from OS randomness for each independent import. Equal XML has the same source
hash and locations but newly assigned instance identities. This report is diagnostic
output, not a saved editable candidate or an admission certificate.

The report contains source occurrences, authored instance bindings and independent
projection availability/errors. It does not duplicate the original XML buffer in every
record. Attribute values refer to exact source ranges. Existing source projections still
provide decoded attributes and ordered item strings/range instructions. The CLI retains
its existing root/lexical inspection gate; the library wrapper can preserve a valid XML
snapshot whose more restrictive typed projections are unavailable.

## Library ownership

[`ImportedBuildInstance`](../crates/poe-optimizer-import/src/build_instance.rs) consumes the
existing decoder DTO through `from_decoded(decoded, lineage, limits)`. Because that DTO
has public mutable fields, construction rechecks XML size, exact hash and bounded parsing.
It owns the source and indices behind shared immutable storage. Cloning the wrapper keeps
its lineage/revision/IDs and shares storage; it does not clone a character or an item.

`source_xml`, `occurrences`, `instances`, `occurrence`, `binding`, `source_fragment` and
`attribute` expose read-only data. Attribute decoding uses the existing one-pass named-entity
rules, preserving original whitespace. `project_skills`, `project_items` and
`project_configuration` reuse existing source projections as temporary borrowed views.
These inspection APIs may allocate and parse; they are not candidate-loop operations.

`InstanceImportLimits` bounds occurrences, instances, nesting, attribute count and copied
metadata bytes in addition to the decoder's byte/node limits. Exceeding a bound fails the
whole constructor. A failed typed projection is different: its explicit error is retained
alongside every authored occurrence, without converting the section to an empty/default set.

## Identities and preservation

| Value | Meaning |
| --- | --- |
| `SourceOccurrenceId` | Exact XML hash plus preorder element ordinal; source addresses can be shared by identical documents |
| `BuildLineage` / `BuildRevision` | Host-managed build lineage and edit revision; neither proves ownership of a native plan |
| Typed instance ID | One authored skill set/group/entry, item set/record/slot use, passive spec or config set |
| Private compiled handle | Future/native preparation ownership; never inferred from a public ID or matching digest |

The [portable identity module](../crates/poe-optimizer-core/src/build_identity.rs) encodes
128-bit lineages and 64-bit counters as exact-width lowercase hex strings. It needs no RNG,
clock, filesystem or global counter. Hosts supply distinct lineages. Typed wrappers prevent
accidental selector interchange; actual collection/domain membership is checked separately.
An allocator watermark does not prove live membership.

`InstanceAllocator` allocates monotonically, preserves existing identities and clones an
instance with a fresh ID plus origin. Persist its watermark even after deleting the highest
ID. `from_existing` is only an initial enumeration helper; rebuilding from live IDs after
edits could reuse a deleted identity. Restoring the same state twice does not coordinate
parallel branches. Candidate editing must establish deterministic host allocation before
using these values across independent workers; R1a exposes no mutable imported-candidate API.

Source order and duplicate external IDs are retained. Item records and their equipment/jewel
slot uses are distinct; two slots referencing one saved item do not establish ownership of
two physical items. Unknown tags remain unknown even where PoB consumes their positional
role as a gem. Namespace contexts remain opaque. Legacy direct groups/slots are preserved
without manufacturing default saved sets. Raw configuration sets remain addressable even
when duplicate IDs or scalar entries cause configuration projection to fail.

R1a does not choose an effective item record from duplicate raw IDs: original item parsing
can decide whether an entry is registered at all. Independent set selections, source-defined
fallbacks/errors and reference resolution belong to R1b. R1c connects existing numerical
paths to the shared boundary. Effective configuration, grants, actors and actions remain
R2/R3 dependencies. No build name, example path or source hash dispatches production logic.
