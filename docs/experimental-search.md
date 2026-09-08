# Experimental controlled search

`search-experimental` searches supplied normal-Mace weapon/support choices and optional
class/ascendancy/entrance selections with the native
Rust backend or the optional PoB reference backend. Both use the same canonical candidates,
objective policy, budgets, locks and fresh finalist verification. This remains a restricted
diagnostic development profile. The intended general release still requires joint search
across all six build dimensions, multiple required skills/items and broad mechanic coverage.

## Run a supplied problem

```powershell
New-Item -ItemType Directory -Force runs | Out-Null
cargo run --locked -- search-experimental --backend native --problem examples/mace-search.json --jobs 4 --max-evaluations 10 --timeout-seconds 300 --output runs/mace-native-search.json --export runs/mace-native-best.xml
```

The [example problem](../examples/mace-search.json) contains four exact weapons and two
support choices: quality-zero and quality-20 Wooden Clubs and Smithing Hammers, each with
no support or Brutality I. Its objective maximizes selected hit DPS. Ten attempts allow one
template calculation, all eight combinations and one fresh finalist. Other supported typed
objectives and constraints use the same schema as evaluation and assessment.

The native-only build also provides controlled search and defaults to the native backend:

```powershell
cargo run --no-default-features --locked -- search-experimental --problem examples/mace-search.json --jobs 4 --max-evaluations 10
```

Developer builds with the default `pob` feature retain the PoB default for compatibility.
Choose the reference backend explicitly when comparing behavior:

```powershell
cargo run --locked -- search-experimental --backend pob --problem examples/mace-search.json --jobs 2 --max-evaluations 10
```

Native execution uses a local Rayon pool with `ExecutionKind::RustCpu`. PoB execution uses
`ExecutionKind::ExternalProcess` and supervised calculation workers. A native run needs no
PoB checkout and never starts a Lua worker or falls back to PoB. Native-only builds omit
PoB worker, extraction and calibration-harness commands.

Native controlled search accepts `--data <package.json>` and optional `--data-sha256`.
It loads the [game-data package](native-data.md) once and shares the snapshot between its
catalog and native evaluator. The public `ControlledMaceCatalog::with_data` API binds
candidate identities, generated XML, requirements and realization checks to that dataset.
PoB reference runs still use reviewed default content and reject `--data` selection.
Native XML exports include a `.data.json` companion with actual dataset identity, trust,
package path hint and XML hash; destination/alias checks include that companion.

```powershell
cargo run --no-default-features --locked -- search-experimental --problem examples/mace-search.json --data runs/custom.json --jobs 4 --max-evaluations 10
```

## Joint class and entrance example

```powershell
cargo run --no-default-features --locked -- search-experimental --problem examples/mace-class-search.json --jobs 4 --max-evaluations 746 --output runs/mace-class-search.json --export runs/mace-class-best.xml
```

The [expanded example](../examples/mace-class-search.json) uses problem schema **2** and
requires explicit available point budgets:

```json
"tree_search": {
  "ordinary_passive_points": 1,
  "ascendancy_passive_points": 0
}
```

Ordinary points must be 0 or 1; ascendancy points must be 0. Omitting `selections` chooses
all admitted combinations within that ordinary budget: 31 class/ascendancy identities,
each with no paid node or either of its two entrance nodes, giving 93 choices with budget 1.
These combine with four weapons and two support choices into 744 structural alternatives.
Reviewed requirements reject 240 Smithing Hammer choices for classes with strength 7;
504 remain legal and a complete run uses 506 calculations including template and finalist.
The 746 limit also accommodates an otherwise fully legal product. The ordinary-budget-0
example has 31 tree choices and 168 legal weapon/support combinations, using 170 calculations.
Selecting an ascendancy does not allocate or implement its ascendancy passives.

To restrict the supplied domain, add a nonempty `selections` array:

```json
"selections": [
  {"class_id": 1, "ascendancy_id": "Witch3b", "entrance_node_id": 4739},
  {"class_id": 6, "ascendancy_id": null, "entrance_node_id": null}
]
```

Class IDs are canonical numeric IDs; ascendancy IDs are internal source IDs, not display
names. Entrance IDs are physical allocation IDs. Witch 4739 uses effect source 17306;
Huntress 56651 uses effect source 39263. Reports retain both meanings. Duplicate/unknown
selections or wrong ownership reject. An explicitly supplied entrance with budget 0 stays
visible as a canonical admission rejection and consumes no evaluation.

Expanded problems add independent locks; omitted fields remain free:

```json
"locks": {
  "class_id": 1,
  "ascendancy": {"kind": "id", "id": "Witch3b"},
  "allocated_passives": [4739],
  "unallocated_passives": [],
  "weapon_id": "wooden-q0",
  "support": "brutality_i"
}
```

`ascendancy: {"kind":"none"}` explicitly locks no ascendancy. Contradictory locks or
locks matching no supplied tree selection reject before calculation. Roots are implicit;
paid-node locks and budgets concern ordinary physical allocations. The legacy schema-1
problem requires no `tree_search` and keeps class/tree fixed. Expanded reports use schema
**3** and the discrete axis order `[weapon, support, tree]`; `tree_choices` maps the third
axis after tree-lock filtering. Legacy reports stay schema **2** with two axes.

## Supported input and locks

`template` is relative to the problem file or an absolute path. XML/share codes pass through
the bounded importer. The legacy template fixes an unallocated Warrior without ascendancy. Expanded templates
accept any admitted class/ascendancy/entrance selection. Both require one
level-1 quality-0 Mace Strike, one normal base (reviewed names: Wooden Club or Smithing Hammer), and zero or one
level-1 quality-0 Brutality I. Supplied item alternatives can change their base, quality
0–20 and item level 1–100 while retaining the exact supported five-line item format.
Character level, configuration and source fields outside the selected mutation ranges
remain fixed throughout a run.

Native Mace currently admits normal enemies only. The PoB controlled profile also supports
its documented boss/Pinnacle scenarios. Unknown JSON fields, unsupported structural
mechanics and invalid locks reject before search; backend-specific admission can also reject
the initial template attempt. Read [controlled mutation support](controlled-mutations.md)
for structural ranges and [native backend coverage](native-backend.md) for the native gate.
The supplied Sorceress/minion build remains an evaluator fixture and is not mutated here.

Native Mace returns eight finite metrics: life, mana, energy shield, four capped resistances
and selected hit DPS. `selected_average_hit` is explicitly unavailable because its shared
contract does not aggregate per-hand attack averages. Objectives requiring unavailable
values cannot produce a feasible recommendation. The standalone native evaluator also
supports its restricted Spark profile, but this mutation command currently operates on the
Mace profile only.

Optional `locks.weapon_id` fixes an exact supplied weapon. `locks.support` fixes `none` or
`brutality_i`. Both are represented in discrete axes and canonical candidate constraints;
replaying validation therefore preserves the locks. The main skill is fixed by this profile. Class, ascendancy and tree are fixed in legacy
problems and controlled by the expanded selections/locks above in schema-2 problems.

The CLI checks canonical point/connectivity/ownership rules, then selected-data equip/use
level and attribute requirements before dispatch. Available attributes come from the
resolved selected class; admitted entrance operations do not modify attributes.
Each attribute uses the maximum of individual requirements and the matching support-color
aggregate; weapon and support requirements are not added. Requirement evidence reports
available/required values and failed boundaries for choices allowed by the locks. Illegal
choices consume no calculations. If all fail, `empty_legal_domain` reports zero evaluations
without even a template attempt or XML export. Source materialization and diagnostic
`evaluate` remain available for inspection and do not themselves certify legality.

The shared `poe_optimizer_import::controlled_mace` module owns immutable catalogs,
source-preserving materialization and realization checks. `preflight` owns shared structural
evaluation checks. Both are pure Rust, with compatibility re-exports at the old
`poe_optimizer_pob::mutation` and `poe_optimizer_pob::preflight` paths.

## Strategies and accounting

Catalog preparation is bounded to 93 tree choices and 128 weapon/support combinations per
tree. A second cap requires `candidate_count * (template_bytes + 4096) <= 256 MiB` to limit
source-hashing work. Admission checks the entire lock-admissible domain, independently of
proposal count; `admission.complete` and `checked_candidates` record whether that check
finished before the deadline. This cap does not implement process-memory admission.

The default `--strategy exhaustive` enumerates the bounded supplied product, subject to
`--max-proposals`. `--strategy guided` starts from a deterministic complete point and uses
the reusable discrete proposer:

```powershell
cargo run --no-default-features --locked -- search-experimental --problem examples/mace-search.json --strategy guided --jobs 4 --seed 42 --max-evaluations 10 --max-rounds 64 --max-proposals 4096
```

The proposer cycles its mutation radius, changes several unlocked axes together and samples
full random restarts at a configured interval. Sampling avoids allocating a Cartesian
product and supports up to 128 axes with u32 cardinalities. Frozen/singleton axes never
change. Empty samples can retry until round/proposal/time limits so later coupled moves or
restarts remain possible. A sampled stall does not establish optimality. Game-state repair
and broader island/diversity policies remain future work.

`--max-evaluations` includes one initial template calculation and one reserved fresh
finalist attempt; its minimum is three. Preparation binds the immutable scenario and is
not a scored seed or independent mechanics golden. Search receives only the remaining
attempt capacity and duration. Failures count. Reports separate preparation, search and
verification, and `total_evaluations` includes all three.

The deadline starts before problem import/catalog construction. These bounded host
operations have cooperative checks, with no hard CPU preemption. Each engine call receives
at most 30 seconds or the remaining run time, whichever is smaller. PoB enforces that limit
through its process supervisor; native calculation uses its cooperative clock boundary.
Late search results are discarded. Final artifact persistence is outside search duration.
CLI signal cancellation and hard process-memory admission are not implemented.

## Realization evidence and export

Search JSON retains selected data identity/trust and requirement rejections, the
template/hash, problem, exact catalogs and payloads, discrete layout,
canonical constraints, requested backend/execution kind, baseline identity/evaluation,
budgets/statistics, ranked assessments and fresh verification. Preparation failure emits
an explicit report with no search or XML export. `best_verified` appears only when the top
feasible candidate passes a fresh calculation with matching assessment and realized state.
The diagnostic marker remains set; consistency does not certify complete game legality.

Native realization requires the backend's XML export to equal the exact materialized
candidate bytes. It checks selected class/ascendancy, implicit roots, physical paid node,
effective entrance and configured effect evidence, the selected Mace action and exact support
gem projection. It separately checks resolved weapon base/quality/item level and support
choice from the immutable native calculation inputs recorded in diagnostic evidence. The
backend identity and external configuration/placeholders must match the fresh template;
candidate-derived condition tables are not frozen. Native validation never calls the
PoB-normalized baseline binder. PoB realization retains its existing normalized-export and
live coverage checks.

Requested XML export contains the materialized source with only item/support and selected
Build/Spec class, ascendancy and allocation attribute ranges changed;
other source bytes are retained. A fresh verification attempt must pass before writing it,
and existing output files are never overwritten. No export is written for an unverified
or infeasible best candidate. Re-import and native re-evaluation of the exported winner are
covered by tests.

The four unchanged independent C-host quality-zero Mace references anchor numerical parity
for generated candidates. Serial and four-worker native runs match those values. The
supplied eight-state example agrees across exhaustive and guided native search, including
its quality-20 alternatives. Changed level/quality and reference-backend Pinnacle checks
supply additional realization evidence; they are not new independent numerical goldens.
See the [native benchmark guide](native-backend.md#fixed-input-throughput-benchmark) for
fixed-input typed API throughput measurements. Those benchmarks include result construction,
validation and accounting; they do not measure raw arithmetic or optimizer quality.

## Tree extraction foundation

Tree extraction remains a separate offline reference command requiring the `pob` feature:

```powershell
cargo run --locked -- extract-tree --tree-version 0_5 --timeout-seconds 30 --output runs/tree-0_5.json
```

Its bounded worker verifies the pinned source manifest and exact executable data literal,
then exports 4,914 nodes, eight classes, 23 ascendancies, shared physical roots and override
provenance. All 14 dangling connections stay explicit. The owned serialized snapshot can
be consumed without a live Lua state. Read [tree data](tree-data.md) and
[controlled tree projection](tree-projection.md) for authenticity, graph semantics and
unsupported mechanics. No site scraping or bulk remote extraction is involved.

The portable admitted class/entrance graph now composes with controlled search without a
live reference or full extraction. Future expansion must broaden source-derived passive
modifiers, equipment and skill/supporting-skill catalogs. Automatic
overrides, point/resource legality and exact realized-state comparisons must survive each
extension. Progress checkpoints and remaining work live in [implementation.md](implementation.md).
