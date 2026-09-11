# Seeded jewel and passive-transformation search

Status: investigation requested by the owner on 2026-09-10; not implemented.
The [J1 feasibility findings](seeded-jewel-feasibility.md) record source/data availability,
a bounded reference-archive probe and the current zero-seeded-build coverage.
This is an optional extension to joint build search, with delivery tracked under J1-J5
in the [implementation record](implementation.md). The target is to discover combinations
of item seed, socket placement and passive allocation that ordinary incremental search
may miss, including useful items outside a short manually curated seed list.

## Source lead and terminology

The pinned PoB checkout calls the seed-bearing Legion items **Timeless Jewels**. Its
[lookup helper](../vendor/path-of-building-poe2/src/Modules/DataLegionLookUpTableHelper.lua)
implements loading compressed/precomputed jewel tables and seed/node lookups. Its
[list control](../vendor/path-of-building-poe2/src/Classes/TimelessJewelListControl.lua)
describes searching and generating items with specific seeds. The
[passive definitions](../vendor/path-of-building-poe2/src/Data/TimelessJewelData/LegionPassives.lua),
[node mapping](../vendor/path-of-building-poe2/src/Data/TimelessJewelData/NodeIndexMapping.lua)
and [trade identifiers](../vendor/path-of-building-poe2/src/Data/TimelessJewelData/LegionTradeIds.lua)
are concrete investigation inputs. This is stronger evidence for reusing lookup data
than for assuming PoB directly runs a random generator for every candidate.

The pinned [passive transformation path](../vendor/path-of-building-poe2/src/Classes/PassiveSpec.lua)
contains disabled seed-based branches with comments requesting updated seed data (around
lines 1613 and 1709), alongside active special handling. Therefore the checked-in reader
is not a verified complete current PoE2 seed generator. The pinned
[item definitions](../vendor/path-of-building-poe2/src/Data/Uniques/jewel.lua) do include
Heroic Tragedy and Undying Hate, with active conquest metadata and partial keystone/Tribute
handling. The legacy search UI targets different PoE1 families and its entry button is
disabled; old trade identifiers/URLs also need game-specific review. No seed binary archives
were found in the checked-in table directory. Data availability and completeness are an
explicit J1 gate, not merely a later performance tuning task.

The presence of these assets in the PoE2 fork is not proof that every represented family
is available in the selected game/version. J1 must distinguish active PoE2 mechanics,
inherited PoE1 code and any analogous PoE2 seeded items. Treat game and data revision as
explicit capability boundaries; keep applicability unresolved until verified. Neither
rune names nor the user's descriptive terminology should select a runtime algorithm.

## Candidate and data contract to investigate

Model this as a reusable seeded passive-transformation provider. Inject the finite legal
seed domain, item family/variant, governing identity where relevant, radius rules, node
mapping, transformation definitions and lookup/generator version. The jewel's authored
seed is separate from the optimizer's random seed. Preserve exact imported item identity
and rolls, and distinguish multiple allowed item instances from one item moved between
sockets. A required fixed item also fixes its seed unless the user explicitly defines a
different lock policy.

A provider produces source-attributed changes to the affected passive definitions. The
normal legality/preparation/evaluator paths then account for allocation costs, reachability,
replacement/addition rules, overlap restrictions and all resulting interactions. Do not
score transformed nodes solely as an additive list of desirable stats. Removing/replacing
a provider must remove its prior changes; class, tree version and socket moves must
invalidate the appropriate prepared dependencies.

First investigate an injected lookup adapter and original-source parity. If a deterministic
native generator is available or worth implementing, place it behind the same provider
boundary and prove it against the authoritative lookup/source behavior. All item-specific
facts remain data; normal native runs must not acquire a PoB/Lua subprocess dependency.
This proposal depends on the now-accepted shared build/provider model. Seed-specific
data/algorithm readiness remains a separate gate.

## Search, cost and reporting

Make seed-domain exploration opt-in, with explicit time, evaluation, memory and seed-count
budgets. Ordinary searches over supplied concrete items should not load or enumerate the
seed catalog. Offer bounded supplied-item exploration first, followed by an explicit
hypothetical seed-domain discovery mode. An existing imported seeded item still requires
correct evaluation even when broad seed exploration is disabled.

Investigate joint proposals that change seed, socket and connected tree paths together,
plus multiple diverse starting allocations. Retain diversity across families and resulting
transformations: a seed that looks weak on the current tree may support a much better
allocation. Cheap node scores may order candidates but are not safe global pruning bounds.
Evaluate finalists with the user's full objective/constraints and other unlocked build
dimensions. Compare against fixed-tree seed ranking and the ordinary joint-search baseline
under equal evaluation/time budgets.

Measure lookup/decompression cost, resulting evaluation cost, domain size and cache memory
before choosing eager preprocessing. Consider lazy per-family/per-node loading, compact
shared immutable lookup storage, bounded batches and Rayon workers. Cache transformations
by game/tree/data/algorithm revision and every actual transformation input; keep allocation,
context and full candidate identity in the evaluation cache. Avoid silently merging
seeds that look identical only on the currently allocated subset.

Results should identify exact seed/family/variant/socket, affected nodes, before/after
allocations and metrics, point cost and reproducible item/build export. Show theoretical
discoveries separately from items present in a dated inventory/market snapshot. Preserve
price/availability uncertainty; a numerically good seed is not evidence it is purchasable.
A later trade handoff can export exact searchable identifiers or user-opened searches
for promising items. Live market ingestion and automated transactions remain separate
product decisions. A later CLI report/GUI view should highlight transformed nodes and
allow comparing alternative socket placements and seed families.

## Validation gates

- Authenticate the complete lookup/generator and applicable game data, including boundary
  seeds, index arithmetic, variant rules, node mapping, radius geometry and source ordering.
- Do not equate parity with disabled/incomplete upstream branches to complete game seed
  correctness; obtain a complete versioned provider/reference before that claim.
- Compare full transformed-node records against original PoB across several families,
  sockets, seed boundaries and independent holdouts; then prove whole-build metrics.
- Exercise provider removal/readdition, moves, overlap/limit behavior, separate point
  categories, locked items and native export/reimport. Keep unsupported cases visible.
- Verify serial/Rayon and fresh/reused-worker agreement, cache invalidation across revisions
  and bounded memory/cancellation. Record the tested seed/domain denominator.
- Use real multi-node interactions to assess search quality under equal budgets. Do not
  claim global optimality, complete seed coverage or profitable trades from a best-found
  result or a collection of local scores.
