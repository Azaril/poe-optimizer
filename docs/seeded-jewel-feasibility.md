# Seeded-jewel feasibility investigation

Status: J1 source and corpus audit on 2026-09-10; no seeded native evaluator or search is
implemented. This supports the [design opportunity](seeded-jewel-search.md) and its
[J1-J5 delivery gates](implementation.md#seeded-jewel-opportunity-j1-j5-follow-up).
The broader real-build model remains the implementation priority.

## What the reference can establish

The project pins PoB2 `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. Its
[item definitions](../vendor/path-of-building-poe2/src/Data/Uniques/jewel.lua) contain
Heroic Tragedy and Undying Hate, eight named variants in total, authored seed rolls
100-8000, Very Large radius and a shared one-Historic limit. These are source facts at
this pin, not independent verification of the live game's current item domains. The
[parser](../vendor/path-of-building-poe2/src/Modules/ModParser.lua) captures digit strings
and conqueror metadata without enforcing the authored seed bounds. Parsing alone cannot
certify legal input.

The [tree consumer](../vendor/path-of-building-poe2/src/Classes/PassiveSpec.lua) has active
radius/conquest assignment and partial keystone/Tribute handling, but its seed-dependent
notable and normal-node paths are disabled pending data. The pinned Lua sources contain
no active initializer for the inherited lookup helper or its seed/type/cache globals. Its
lookup directory has three support Lua files and no seed archives. Implementing the reader
alone would not yield a complete PoB2 seed-to-node reference.

Numeric family IDs cannot cross that boundary unchanged. The inherited
[reader](../vendor/path-of-building-poe2/src/Modules/DataLegionLookUpTableHelper.lua) uses
1/2 for Glorious Vanity/Lethal Pride; the unfinished PoB2 tree branch locally uses 1/2 for
Kalguur/Abyss. The current independently inspected PoB1 source uses further family IDs
and local-to-global passive remapping. A provider must bind the game, actual family,
node-map revision and transformation format rather than reuse an integer or display name.

## Available data and bounded archive probe

A targeted official repository check pinned PoB1 to
[`16de4b82d57f1c0de6eb40f37143c32d4da36a02`](https://github.com/PathOfBuildingCommunity/PathOfBuilding/tree/16de4b82d57f1c0de6eb40f37143c32d4da36a02).
Its [Timeless data directory](https://github.com/PathOfBuildingCommunity/PathOfBuilding/tree/16de4b82d57f1c0de6eb40f37143c32d4da36a02/src/Data/TimelessJewelData)
contains a Heroic Tragedy archive. The project downloaded that one 2.18 MB reference asset
for this investigation; it is retained only under ignored `runs/seeded-jewel-j1-probe`.
The complete observed 11-family inventory totals 177,487,738 bytes (169.27 MiB) compressed,
which is a distribution size, not runtime memory. No other seed archive was downloaded.
No archive was added to the shipped game data, and no PoE2DB/PoEDB scraping occurred.

| Observed property | Evidence |
| --- | --- |
| Compressed bytes | 2,181,337; verified both Git blob SHA1 and upstream manifest raw-content SHA1 |
| Compressed SHA256 | `461d5ca14d9f90e7c263c777735936587ac047fc1ff9bc9cff7be68fddf24561` |
| Format | Complete zlib stream despite the `.zip` suffix; no trailing bytes |
| Decoded bytes | 3,587,054, equal to 454 rows of 7,901 bytes |
| Decoded SHA256 | `87b836c9c4ed4273a032fe1abc58755c03a1ce61d2dc14755716d50ceec9eb8c` |
| Decode bound | 16 MiB; the probe rejects a stream exceeding the bound |

The [PoB1 mapping](https://github.com/PathOfBuildingCommunity/PathOfBuilding/blob/16de4b82d57f1c0de6eb40f37143c32d4da36a02/src/Data/TimelessJewelData/NodeIndexMapping.lua)
declares 1,937 nodes and 454 notable rows; the pinned PoB2 mapping declares 1,913/447.
The newer [reader](https://github.com/PathOfBuildingCommunity/PathOfBuilding/blob/16de4b82d57f1c0de6eb40f37143c32d4da36a02/src/Modules/DataLegionLookUpTableHelper.lua)
also remaps local passive IDs. These differences prevent treating this archive as a drop-in
PoB2 provider. The PoB1 Abyss dataset also names Zorath where the pinned PoB2 definitions
name Kulemak, and uses a different structured reader rather than this flat table format.
The Heroic reader's inclusive notable-index check and neighboring-row boundaries need exact
source tests; the probe's byte-count/checksum agreement proves decompression only.

A generic Rust `flate2`/`miniz_oxide` probe produced the same decoded checksum in five
fresh decoder states. Median decode time was **22.39 ms** (22.32-22.48 ms); output hashing
was separately 1.28 ms median and the single file read took 7.71 ms. Timing includes
bounded allocation/copying and complete-stream checks; CPU/OS/allocator caches were not
flushed. A separate 1 KiB output bound rejected the archive before oversized output growth
and wrote no result. Process/decoder memory was not measured. Neither these timings nor
the fixed-width byte layout measures transformed-node assembly, complete evaluation,
parallel lookup, full search cost or browser throughput.

The inspected repositories contain a project MIT notice; generated passive definitions
separately identify Grinding Gear Games game-data copyright. No seed-pack-specific
provenance/license manifest or reproducible generator was found in the examined inputs.
Record the applicable data provenance and distribution terms before shipping a pack; the
repository notice alone is not evidence for every underlying dataset. These remain J1
inputs, separate from successful decompression.

## Requirements exposed by actual source consumers

The pinned [radius construction](../vendor/path-of-building-poe2/src/Classes/PassiveTree.lua)
uses tree-version-selected geometry, squared inclusive inner/outer boundaries and node
eligibility. The conquest consumer can transform unallocated eligible nodes, provided the
receiving socket is allocated and its item is enabled. Caching only currently allocated
nodes would miss effects on future paths. The legacy search uses Large radius and ranks
weights for a fixed socket/tree; authored PoB2 Timeless items use Very Large. That search
is useful prior art, not a complete joint-build optimization algorithm.

Item assembly determines variant lines and conquest metadata. The
[limit pass](../vendor/path-of-building-poe2/src/Modules/CalcSetup.lua) treats Timeless
items as a shared Historic group and permits an explicit limit bypass. Overlapping effective
providers assign conquest in source iteration order; do not invent a stable priority for
ambiguous inputs. Reset/reapply, hash overrides, class switches and saved item references
used in multiple sockets require full provider-lifetime tests. Preserve node identity,
allocation and topology as well as the changed definitions/modifiers.

The [current data loader](../crates/poe-optimizer-data/src/game_data.rs) embeds and eagerly
decodes its JSON package; its default byte limit is 32 MiB and the G4 package is 23,431,124
bytes. J2 should investigate optional content-identified binary packs and lazy per-family
loading, keeping host I/O outside the portable evaluator. This is a packaging question for
the future provider seam, not an implemented API change. Every actually loaded pack/map
must participate in data and cache identity; a filename or modification time is insufficient.

## Supplied-build coverage and remaining gates

The five supplied originals contain 116 item records, including 18 jewels and three
radius items, but **zero seeded Timeless items**. Their empty `TimelessData` search settings
are UI defaults. The three radius cases are Time-Lost Sapphire, From Nothing and Time-Lost
Emerald; preserve them as real provider/allocation cases without counting them as seed
coverage. Existing selected-view PoB reports contain 613 passive-node observations and
zero `is_conquered=true` observations. This does not cover every inactive saved tree.

J1 has established a concrete lookup-data lead and the relevant incompatibilities. It
remains open for a complete intended-game mapping/reference, data provenance/distribution
review and meaningful transformed-node/full-evaluation costs. Do not fill that gap by
matching disabled upstream branches and advertising complete seed behavior.

Before J2-J5 completion, obtain independent concrete seeded builds and boundary seeds for
multiple variants. Require full node records and whole-build outputs, alternate sockets,
provider removal/readdition, overlap/limit cases, class/tree changes, corrupt/missing data,
and serial/reused-worker/cache agreement. Compare joint seed/socket/path search against
fixed-tree ranking under equal budgets. Hypothetical discoveries and market availability
remain separate claims.

Local evidence: `runs/seeded-jewel-j1-source.json`, `runs/seeded-jewel-j1-data.json`,
`runs/seeded-jewel-j1-corpus.json`, `runs/seeded-jewel-j1-reference-coverage.json`,
`runs/seeded-jewel-j1-loading-seam.json`, `runs/seeded-jewel-j1-probe/archive.json` and
`runs/seeded-jewel-j1-probe/native-bench/validation.json`.
The preserved source/caller/package identities are unchanged.
