# Headless rune selection

Rune-choice preparation certifies the first record and name-based selection without
reproducing the remaining source display order. The implementation and regression cases
are in [rune_choices.rs](../crates/poe-optimizer-import/src/item_sets/rune_choices.rs).
This is a bounded correction at the existing injected-data seam, not an execution-model
choice. The declared source-fed activation comparison and its startup-sort checks pass;
this does not establish public native activation or complete build evaluation.

## Why a strict weak order is too strong

The original [ItemsTab construction](../vendor/path-of-building-poe2/src/Classes/ItemsTab.lua)
at lines2216–2248 contains a real comparator cycle in the injected definitions:

| Row | Order | Required level | Group |
| --- | ---: | ---: | ---: |
| Adept Rune / armour | 993 | 15 | 3 |
| Greater Adept Rune / caster | 993 | 30 | 1 |
| Aldur's Legacy / armour | 6239 | 0 | 1 |

The first row compares less than the second by required level; the second compares less
than the third by order; the third compares less than the first by group. Group is the
number of constructed display lines, including bonded lines. Requiring a strict weak
order rejects this source data even though that property is unnecessary for the consumer
described here.

The native producer still constructs all reached rows, parses ordinary lines and applies
source attribution before filtering. Bonded lines contribute description text and group;
they do not invoke that parser loop. A successful sort proof cannot replace these
dependencies or turn a source failure into success.

## Pinned sort contract

The proof uses `auxsort` in `luajit-src 210.7.3+1ee778a`, pinned by
[Cargo.lock](../Cargo.lock). Its crate checksum is
`869665372263eb337b14f480cfb864b89f12eade4eb42cb415f71517b4a67572`.
The inspected `luajit2/src/lib_table.c` SHA-256 is
`0889591a8861e3daed2be051483d1a7c6eb1b0e8ed29302bc302019e51d94235`;
the comparator and sort implementation occupy lines180–268. A dependency or algorithm
change requires re-auditing this proof. It is not a contract of arbitrary `table.sort`
implementations.

The input is a dense finite sequence of distinct constructed row tables. Comparisons
must be stable, callback-free and total on the relevant finite operands. For every
distinct pair, the producer checks both directions and rejects simultaneous `a<b` and
`b<a`. For sequences of four or more rows it also checks `!(a<a)` for every row: the
source algorithm can compare its pivot with itself. Its shorter cases have no diagonal
comparison, so a two-row unequal-order case need not read otherwise-unused required
levels. Operand/type failures remain explicit unavailable dependencies; this conservative
certificate does not claim an exact source error prefix for a failed arbitrary ordering.

For original-host evidence, the actual C sort must be retained and verified around the
original construction, together with the authenticated source comparator and row inputs.
The startup witness uses a continuation of the exact original module as its completion
boundary, rechecking its retained `runeModLines` local; it does not require a C-return hook.
Ten retained startup observations each check 595 distinct rows and 5,377–6,117 actual
comparator calls, reaching module continuation line2286 with the optional C-return diagnostic
false. See `runs/r2ah-equipment-activation-01/activation-source-review-05.json`.
An observed successful order alone is not a certificate for other permutations. External memory limits and interruption remain separate
from this logical termination argument.

## Completion and first-record proof

The median-of-three setup first establishes `!(a[u]<a[l])`. If the middle row is less
than the left row, swapping them establishes `!(P<a[l])` by asymmetry. Otherwise, choosing
the old upper row as pivot inherits the first comparison, or choosing the middle row
inherits its failed comparison against the left. Every branch therefore establishes
the left sentinel `!(P<a[l])`.

The pivot is retained at `u-1`. Partition scans and swaps leave both that position and
`l` untouched. The forward scan stops at the pivot because `!(P<P)`; the backward scan
stops at the left sentinel. Neither invalid-order guard can fire. Each iteration advances
the indices, and the final pivot position is interior, so both subsequent ranges shrink.
The two- and three-row cases contain bounded comparisons. All mutations are swaps:
the procedure terminates and preserves the row multiset, without requiring transitivity
or establishing that the complete result is sorted.

Separately, require one exact row `m` with `m<x` and `!(x<m)` for every other row.
Do not assume it is the configured empty row. Median setup moves it to the left endpoint
when it occupies an endpoint or the middle; otherwise it cannot be the pivot because of
the pivot's left-sentinel property. It passes the forward scan and stops the backward
scan, remaining or moving into the left partition. Induction on shrinking ranges places
this unique universal minimum at index1.

All pair, diagonal and storage work is bounded and charged. The returned private index
sequence starts with that row; the remainder retains deterministic native storage order.
It is not source traversal or an array-order certificate.

## Allowed consumers and remaining boundaries

Each control prepends the exact owned first-row handle and retains every eligible row.
Require each requested name to identify one exact row within that control. Distinct
same-name records remain unavailable even when their serialized contents match; another
occurrence of the same first-row handle is harmless. A missed name preserves the previous
handle, and cross-owner or wrong-control handles are rejected. The selected record retains
its actual prepared modifier values rather than only a name.

These guarantees cover initial selection and name lookup with unique winners. They do not
cover display indices/order, first-match duplicates, ordered `GetValidRunesForItem` output,
arbitrary source aliases, callback effects, or later consumers of the full sorted array.
Those consumers require their own evidence.

The [equipment comparison](native-equipment-integration.md#r2ah-current-implementation-boundary)
has a narrower source observation scope: rune selection is compared by name, not by
source effect-record identity. Successful component preparation does not establish that
the public parser completed every dependency. Full native build parity remains 0/5.

Regression coverage includes the real cycle with and without a unique minimum, tied
non-initial records, symmetric/diagonal violations, reached malformed operands, duplicate
eligible names, work bounds, and a complete producer call over the three actual raw rows.
Original-sort permutation checks complement the proof; their orders are evidence only
and never become native production inputs.
