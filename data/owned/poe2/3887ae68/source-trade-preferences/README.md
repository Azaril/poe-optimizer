# Saved trade-search preferences

This source-only import policy accounts for a complete ordinary
`Items/TradeSearchWeights` frame and its `Stat` children. It preserves the source
bytes, labels, stat names, weight strings, order and duplicate occurrences in
import provenance. It creates no native gameplay input or optimization objective.
Callers must still author their desired optimizer objectives explicitly.

The reviewed source loader writes only `tradeQuery.statSortSelectionList`.
Known stat names refresh source display/transform metadata; unmatched names are
not inserted into that source list. Native import does not copy that filtering
or replace saved names. Those records remain source preferences, including
unknown names and zero or negative finite weights.

Complete pinned Build, ItemsTab, TradeQuery, TradeQueryGenerator and ButtonControl
files establish the boundary. Fresh construction creates empty trade state;
ItemsTab.Load appends saved preferences without launching the trade generator.
The explicit trade button opens PriceItem, which creates the generator, and later
controls use the weights for comparisons and searches. These are meaningful
future trade preferences, not universally inert display fields. No PoB VM,
network call, market access or source mutation is needed for this static review.
The shared [upstream license](../configuration-accounting/evidence/pob/LICENSE.md)
applies to the archived source.

The existing opt-in source-presentation policy gains one family. It accepts a
unique ordinary root Items frame and at most one complete weight frame. Each
Stat is a plain leaf with nonempty label/stat and a finite, whitespace-free
numeric weight. Unknown attributes/children, malformed numbers, namespaces,
ambiguous frames and unsupported parents retain their prior obligations. All
candidates are validated and work charged before any disposition changes.
Existing semantic links are never replaced. The Items parent and any unresolved
configuration/usage inventories remain pending.

The five original fixtures contain 1/1/1/0/3 affected source rows. Original05's
verified change is only ordinals 741–743: configuration-origin links 19 → 16,
with five inside Config unchanged and fourteen → eleven outside. These are
source-accounting results; the four complete input inventories remain Pending.

The focused Import suite passes 21 tests, including five new tests for finite
values, duplicate choices, unknown names, complete-frame rejection, exact parent
and source binding, all-five preservation and charged work/replay. Strict target
Clippy passes. Logs: `runs/owned-trade-preferences-import-01.log` and
`runs/owned-trade-preferences-import-clippy-01.log`.

Eight ordinary CLI checks and strict Clippy pass. The production CLI publication
and exact eighteen-artifact rebuild pass in 24.36 seconds, preserving all five
originals, all 110 queries, earlier IDs/values/preferences and selected issue
counts 106/117/109/123/4. Only normalization, tree-normalization and release
metadata change from the selected physical-base predecessor. Logs:
`runs/owned-trade-and-ci-cli-01.log`,
`runs/owned-trade-and-ci-cli-clippy-01.log`, and
`runs/owned-trade-preferences-publication-01.log`.

The checked package is `runs/owned-trade-preferences-publication-01/package`,
input `df28e08f4cb13246832fd8e6e8124f1ceb52c321b16bf927dc586bb8fd71be95`.
It has 64,464,962 bytes, 195 provenance rows and unchanged 8188 effective
programs. Original05 retains 33 known assumptions, 35 usage preferences and
20 usage links. No gameplay coverage, final metric or complete build is claimed.
