# Pinned tree topology investigation

Investigated on 2026-09-07 against PoB commit `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, without changing the submodule or downloading another tree. This extends [skill coverage](skill-coverage.md) and informs the canonical candidate model and cross-class search.

The pinned graph retains two ordinary-tree entrances for every class-start location and the starts of its catalog ascendancies. The 14 dangling connections do not establish that an available class lacks a usable tree. They remain unresolved source-data coverage issues: the repository does not contain the missing targets' identity or stats, so it cannot prove they are optional/inactive rather than missing game nodes. Search may operate against the explicitly identified PoB graph; a claim of complete game topology needs separate evidence.

## What is verified

`PassiveTree` first maps the exported records by `skill` ID. For each raw connection it looks up the target, skips missing targets and image-only/self edges, and then inserts **both directions** into `linkedId`. It skips drawing class-start and cross-ascendancy connectors only after constructing those links. Missing targets therefore affect the allocation graph, not merely rendering. The same adjacency also adds `Condition:ConnectedTo<Class>Start` modifiers to ordinary starting nodes. See [PassiveTree.lua](../vendor/path-of-building-poe2/src/Classes/PassiveTree.lua), lines 211-337 and 409-416.

A static scan of `TreeData/0_5/tree.lua` found these relationships. Ordinary neighbors below include reverse-listed connections, matching the loader's undirected graph:

| Start | Classes in the actual `classes` catalog | Ordinary neighbors | Missing raw targets |
| --- | --- | --- | --- |
| 44683 | Monk (10) | 10364 Skill Speed; 52980 Evasion and Energy Shield | 5162, 45406, 50198 |
| 47175 | Warrior (6) | 3936 Melee Damage; 38646 Armour | 16732, 51916, 54579 |
| 50459 | Ranger (2), Huntress (8) | 13828 Evasion; 56651 Projectile Damage, with a class-specific override | 24665 |
| 50986 | Mercenary (9) | 59779 Armour and Evasion; 59915 Projectile Damage | 39383, 10889, 62386 |
| 54447 | Witch (1), Sorceress (7) | 44871 Energy Shield; 4739 Spell Damage, with a class-specific override | None |
| 61525 | Druid (11) | 13855 Armour and Energy Shield; 50084 Spell and Attack Damage | 35715, 26353, 950, 28429 |

The numerical class identities here are the pinned catalog's `integerId` values. Start labels additionally mention Shadow, Marauder, Duelist and Templar, but those names are absent from this revision's class catalog. They must not become selectable classes merely because `classesStart` mentions them. Existing alternate-start item mechanics can still refer to these labels; class selection and start-location identity are separate concepts.

Several valid ordinary edges are recorded only from the neighbor: 10364 to the Monk start, 59779 to the Mercenary start, 44871 to the Witch/Sorceress start, and 50084 to the Druid start. Reading only a start's `connections` would omit these paths. The observed ascendancy neighbors are:

| Start | Existing ascendancy-start node IDs |
| --- | --- |
| 44683 | 74, 9994, 11495 |
| 47175 | 5852, 32534, 33812 |
| 50459 | 1583, 36365, 41736, 46990, 63493 |
| 50986 | 7120, 36252, 55536 |
| 54447 | 8305, 22147, 23710, 32699, 40721, 59822 |
| 61525 | 35535, 42761 |

Some classes/ascendancies share a location or switch the record's contents. For example, Lich/Abyssal Lich uses a switchable record at 23710. Candidate identity must preserve the base graph node and its chosen override; counting catalog names as independent physical roots is incorrect. `PassiveTree` establishes start identities and `BuildAllDependsAndPaths` applies class/ascendancy-specific replacements.

All 14 currently missing targets are also absent as top-level node records in every bundled historical tree, `0_1` through `0_4`. Other dangling start references became available in later bundled versions: 11495 and 63493 appear in `0_5`, for example. This establishes persistent references, not their intended gameplay meaning. No archived node definition or replacement mapping for the current 14 targets was found in those trees.

## Ascendancy components and separate point budgets

Ascendancies have separate allocation roots and point allowances. They are not necessarily
separate connected components of the raw `linkedId` graph: that graph includes class-to-
ascendancy connectors even when they are not drawn. All **23** ascendancies in the pinned
catalog resolve to valid roots, including the shared Lich/Abyssal Lich location. Their allocations
are validated from the selected ascendancy's root, with its own allowance; they do not
need a paid path through ordinary nodes. The selected class root and ascendancy root are
implicit and cannot be submitted as paid allocations.

This is already implemented in the canonical
[candidate validator](../crates/poe-optimizer-core/src/candidate.rs) and the injected-data
[passive allocation validator](../crates/poe-optimizer-data/src/passive_allocation.rs):
ordinary and ascendancy node sets receive separate connectivity checks, with ascendancy
ownership validated against the selected class. The core candidate validator additionally
enforces their separate point budgets; the data resolver has no budget input and does not
certify point entitlement. The new breadth phase
will exercise these rules on complete imported builds as well as small graph cases.

The **14 dangling references** above are a different condition: their target IDs have no
node record anywhere in the pinned extracted graph. They are not valid ascendancy nodes
that were merely omitted from an ordinary-tree traversal. The fact that they all originate
at class starts makes filtered ascendancy data a plausible explanation, but does not prove
their identities. Retain them as source-coverage diagnostics; do not join separate trees,
spend ordinary points on ascendancies, or fabricate missing nodes to eliminate the warning.

## Six isolated ordinary notables in the broader corpus

The 2026-09-08 [breadth inventory](breadth-mechanism-inventory.md) finds six ordinary
notables in build 1 without an ordinary allocated path: **338 Invocated Limit, 8483 Ruin,
47441 Stigmata, 49088 Splintering Force, 55180 Relentless Fallen and 59387 Infusion of Power**.
These are present in the full pinned tree and are not ascendancy allocations or missing
node definitions.

Fresh original-PoB MAIN state preserves all six as allocated, with `connectedToStart=false`
and `intuitiveLeapLikesAffecting=[7960]`. Socket 7960 contains item 16, **From Nothing
(Diamond)**, whose `fromNothingKeystone` is `ritual cadence`. Its small-radius provider
permits those allocations without an ordinary path. The separate Time-Lost jewel at socket
61419 overlaps one of them; it is not the shared provider for all six.

The native legality model must resolve the enabling item/radius provider before applying
ordinary connectivity rules, and revalidate allocations if that provider changes. The
observed relationship does not by itself establish point entitlement or complete game
legality. None of the five corpus builds allocates any of the 14 source-missing target IDs.
Local provenance and node/provider details are retained in
`runs/breadth-passives-inventory.json` and `runs/breadth-dependencies-analysis-3/summary.json`.

## Likely source of the dangling references, and limits

The pinned data exporter can create exactly this structural condition. [passivetree.lua](../vendor/path-of-building-poe2/src/Export/Scripts/passivetree.lua), lines 689-744, skips nodes with empty names, filtered `[DNT...]` names, or disabled/filtered ascendancies. Lines 1064-1075 copy the surviving record's connection IDs without checking whether their targets were emitted. Its class exporter also excludes classes with no included ascendancies (lines 566-618), while start labels are copied separately (lines 1037-1044).

This is a plausible explanation for retained references to inactive data. It is **not proof of the reason for any of these 14 targets**: the source game tables and generation log that produced this particular tree are not provided here. The repository also contains a different GGG-export conversion path with its own inactive-node handling. Do not infer a specific target's class, ascendancy, replacement ID, stat, or availability from the number of missing edges. Do not synthesize nodes or reconnect to nearby IDs.

The local investigation proves internal graph structure and PoB behavior. It does not verify the current live game tree, a complete route to every notable, or every mechanic's legality. A corrected upstream revision or version-matched authoritative game export should be compared as a new, separately fingerprinted dataset. Preserve the old revision for replay and numeric parity.

## Candidate validation implications

The canonical model needs a graph and legality report separate from calculation success. The backend contract already permits this separation; a successful PoB import is insufficient evidence that the requested candidate was evaluated unchanged.

1. **Resolve catalog identities before mutation.** Validate tree version, class identity, and ascendancy ownership together. In [PassiveSpec.lua](../vendor/path-of-building-poe2/src/Classes/PassiveSpec.lua), lines 176-245, XML import resolves `ascendancyInternalId` to an index without checking its owning class; `SelectAscendClass` then selects that index in the chosen class. An incompatible class/internal-ascendancy pair can select a different valid ascendancy. Invalid indexes can fall back to None. Compare resolved internal identities after import, not merely numeric positions or labels.
2. **Construct the observed graph faithfully.** Use base node IDs and bidirectional valid connections. Exclude image-only/self edges, retain dangling-edge diagnostics outside usable adjacency, and preserve node types, ascendancy ownership, class overrides, and unlock prerequisites. Stable IDs are scoped to the rules revision. Shared start locations do not make classes interchangeable: attributes, ascendancies, and switched passives differ.
3. **Validate per-allocation connectivity.** Main-tree, weapon-set 1 and weapon-set 2 allocation paths differ. Common allocations cannot depend on a weapon-only route; each weapon set can use common nodes plus its own nodes. Ascendancy roots and paths require their own ownership rules. Mastery and class-start nodes cannot be ordinary transit shortcuts. `CanPathThroughAllocMode`, `GetAllocationPath` and `BuildNodePathsToRootNodes` implement these distinctions in [PassiveSpec.lua](../vendor/path-of-building-poe2/src/Classes/PassiveSpec.lua), lines 863-976 and 1367-1428.
4. **Represent exceptional allocation provenance.** Alternate-start jewels, radius allocation, free/granted passives, generated subgraphs, mastery choices, attribute choices, and unlock constraints need explicit providers and coverage. A simple connected-subgraph check would reject some supported special allocations and accept others without their enabling item. If a feature is unmodeled, return unsupported/unknown legality rather than silently treating it as ordinary connectivity.
5. **Compare intended and realized allocation.** Import adds the chosen class/ascendancy starts automatically, defers unknown node IDs into `allocSubgraphNodes`, applies overrides, and rebuilds paths. `BuildAllDependsAndPaths` can prune disconnected nodes through `DeallocSingleNode`; class changes reset the ascendancy and can remove disconnected allocations. Compare node IDs, effective overrides, weapon-set modes, class/ascendancy identities, and granted-node provenance after normalization. Report unexpected drops/additions, and never score the original candidate key with a different realized build. See `ImportFromNodeList`, `SelectClass`, and pruning around lines 358-413, 655-710 and 1929-1978.
6. **Enforce the configured progression budget.** `CountAllocNodes` distinguishes ordinary, ascendancy, secondary ascendancy, and weapon-set use and excludes starts/free allocations. PoB's [Build.lua](../vendor/path-of-building-poe2/src/Modules/Build.lua), `EstimatePlayerProgress` at lines 1025-1112, primarily estimates progression and emits over-allocation warnings. Candidate legality must enforce the user's actual level, quest rewards, ascendancy progression, and relevant item/passive-granted point changes. Import success or an inferred level is not permission to spend extra points.

Record two independent statuses: consistency with the pinned rules graph and completeness relative to the target game version. The 14 global dangling edges may make the latter unknown without making every internally consistent allocation invalid. This keeps all catalog classes in scope while presenting the evidence honestly. User-facing results should identify assumptions and data coverage; diagnostic evaluation can remain available for imports that cannot yet be certified.

## Concrete fixtures and tests to implement

These are proposed tests, **not tests run by this investigation**:

| Fixture/test | Required assertion |
| --- | --- |
| Pinned topology extraction | Exactly the 14 known dangling pairs at this pin; every catalog class resolves a start; both ordinary entrance IDs per location survive bidirectional reconstruction. Derive runtime adjacency and compare to an independent static export, rather than duplicating one loader. |
| Every catalog class and ascendancy | Minimal no-item/no-special-passive builds enumerate all allowed identities, including None and replacement ascendancies. Fresh MAIN and exported identities must match the request; no fallback or silent class reassignment. |
| Two entrances per start | Allocate each ordinary first node individually for each owning class. Assert it remains allocated and costs one point. This explicitly covers reverse-listed edges and the shared-start class overrides. |
| Invalid class/ascendancy pair | Combine Sorceress class ID 7 with `Warrior1`; reject before calculation or reject the realized mismatch. Never accept Stormweaver merely because both use ascendancy index 1. Include out-of-range indexes and a legacy-only start label as a class. |
| Unknown/dangling allocation | Request an absent target such as 5162 without a verified generated subgraph provider. Preserve the requested ID in import evidence and report unresolved allocation; export must not quietly turn it into a successful mutation. |
| Disconnection and class change | Start with a verified legal branch, remove its only connector or change to a distant start. Assert pruning is detected. A repaired candidate must explicitly include a verified connecting route and pay for its added nodes. |
| Weapon-set separation | Build two individually connected branches. Reject a common node depending solely on set 1 or a set 2 node depending solely on set 1; verify legal common-plus-own-set routes survive. |
| Special roots and constraints | Versioned fixtures for alternate-start/radius/free allocation, mastery choice, and unlock prerequisites. Removing the provider or prerequisite must update legality and realized nodes. Keep unsupported mechanics visibly unknown. |
| Point boundaries | Test exact configured allowance and one point over for ordinary and ascendancy points; exercise unequal weapon-set counts and a verified source of additional points. An over-budget PoB output must not become feasible solely because it calculated. |
| Round-trip identity | Import, normalize, export and import again; require stable effective nodes, overrides, allocation modes and identities. Preserve the original XML and report intentional normalization separately. |

The first four cases allow useful cross-class fixture coverage without inventing any missing node. Broader topology certification remains a tracked data question, not a reason to remove cross-class search from the design.

## Executable identity and root coverage

The existing [native passive parity regression](../tests/native_passive_parity.rs) was rerun
at the injected-configuration checkpoint: 100 fresh PoB evaluations pass. It derives 98
class/ascendancy/entrance combinations from the two small Spark/Mace fixtures and reimports
two selected cases. It compares requested and realized class/ascendancy identities, effective
source records, implicit roots, ordinary entrance allocations and supported measurements.
All 23 catalogued ascendancies are represented; none requires an ordinary paid route to its
ascendancy root. Separate point-budget enforcement remains the candidate validator's job;
this diagnostic evaluator explicitly does not certify a character's point entitlement.

These generated combinations are identity/topology regression cases, not independent
held-out builds or coverage of arbitrary paid ascendancy effects. The full-build breadth
phase still needs imported allocation provenance, separate budget boundaries and exceptional
allocation cases. The 14 absent source targets remain a separate diagnostic.
