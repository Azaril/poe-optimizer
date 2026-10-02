# ADR: Explicit empty character-rune selections

**Status:** Implemented and validated within the existing Import boundary.
**Date:** 2026-10-01

## Context

The fourth original saves five character-rune selections with `runeName="None"`.
The source importer preserves those XML occurrences, but prior normalization
created an equipment use for each, with unresolved item, destination and scope.
Those fifteen obligations describe items that the source did not select.

Character rune controls are distinct from runes inside equipment and from passive
jewels. Their source loader initializes a finite set of named controls, and the
calculation consumer delivers their modifiers only when the character-rune gate
is enabled. A disabled gate does not establish an absent selection. Unknown rune
names are also unsafe evidence: the dropdown can retain its previous selection
when a name lookup fails. Explicit absence needs its own checked source proof.

## Decision

Extend the opt-in equipment import policy with a V3 profile containing the pinned
mapping-source identity, reviewed slot-name inventory and exact empty-selection
token. The slot names and token are injected data. Source grammar belongs to
Import; neither Core nor the native evaluator receives PoB control names.

Prove only explicit empty rows in a canonical direct ItemSet under a checked
Items container. First check the loader's table-write branches across every
ItemSet using the injected ordinary/rune control inventories and canonical URL
keys. A malformed archived set can abort loading before the selected set becomes
active. Keep this load-safety check separate from each set's stricter absence
proof, so a harmless unresolved duplicate in another set need not become shared
authority or invalidate an independent selection.

Census same-container rune siblings before accepting any row,
so a duplicate, namespace alias or malformed row cannot conceal another write.
Reject unsupported attributes, content and ownership. Missing, unknown and
occupied values retain their existing unresolved obligations. Process archived
ItemSets independently, without manufacturing default occurrences for unsaved
controls or borrowing a selected ItemSet's proof.

After normal receiving-use construction, remove only the exact proven empty use
and its three matching Pending issues. Remove its membership from the owning
equipment preset and retain its source occurrence as
`SourceOnly(explicit-empty-character-rune-selection)`. Keep spent allocator IDs,
all remaining identities and the existing sidecar version. The source importer
continues to retain the complete original XML and source occurrence inventory.

Do not close the containing ItemSet inventory in this step. Its other contents
still require independent proofs, including occupied item augments. This change
adds no item, socket definition, numerical program or alternative runtime path.
The existing V1/V2 equipment policies retain their historical behavior.

The source profile binds the source it interprets. The enclosing V3 policy keeps
V2's item-line/source bindings. Attaching V3 changes the equipment-policy identity,
so checked release assembly must explicitly rebind the passive-placement policy
and the full tree normalization envelope. Tree content and all owned definitions
remain unchanged. Bound and validate profile data and source work before use;
reuse the immutable source census without raising the existing work budget.
The same bounded sibling scan can collect safe per-set candidates before any
retirement; it does not repeat item modifier parsing.

## Alternatives and consequences

| Approach | Benefit | Cost or limitation |
| --- | --- | --- |
| Source-only absence with bounded post-construction retirement | Reuses existing provenance and preserves every remaining occurrence ID. | Requires exact retirement checks and an explicit policy identity transition. |
| Skip rows before allocating receiving uses | Simpler construction loop. | Renumbers later occurrences in already published builds. |
| Add a physical item or runtime rune object for `None` | Fits the existing receiving-use list superficially. | Invents an entity and couples the evaluator to an external UI convention. |

This is cold normalization work. Native candidate evaluation still consumes
immutable owned inputs with worker-local scratch. The separate skill-input,
skill-preset usage and preparation-readiness proposals are unaffected.

## Acceptance

1. Load all five unchanged originals through the complete pinned source in both
   JIT modes. Observe saved and effective selections, actual modifier delivery,
   ItemSet switching and fresh/reused controls.
2. Exercise explicit empty, occupied, absent, missing, unknown, duplicate,
   namespaced and malformed source shapes. Keep source permissiveness distinct
   from the narrower normalization admission proof.
3. Verify injected profile validation, stale source binding, bounded work,
   historical policies and exact source/use ownership in Rust.
4. Publish from the checked passive-jewel endpoint. Compare every canonical fact,
   source link and remaining local ID, preserve allocator watermarks and all
   110 queries, leave the predecessor unchanged and rebuild byte-identically.
5. Re-finalize the five saved selections. The expected isolated issue change is
   Original04's 137 to 122; all other selected issues and all ItemSet closures
   must be unchanged. Complete native parity remains a separate gate.

All five acceptance gates pass. Original04 has 122 selected issues after the exact
five-use/fifteen-issue retirement; the other originals remain at 116, 117, 109 and
20. Source witnesses agree byte-for-byte across JIT modes, and publication preserves
all remaining facts and identities with a byte-identical rebuild. Complete native
parity is still 0/5. The [implementation log](implementation.md) records the checked
endpoint, validation and next blocker. Initial evidence is
`runs/owned-empty-character-runes-audit.md`.
