# Experimental controlled search

`search-experimental` searches supplied Mace weapons, support loadouts and optional
class/ascendancy/passive selections with the native Rust backend or the optional PoB
reference backend. Problem schema 6 adds [actor configuration](actor-resources.md) and
uses report schema 7. Actor modifiers are fixed configuration for the run and affect
requirements/resources; the search still varies the declared item/tree/support axes.
Problem schema 5 adds normal or rare weapons with five admitted local
modifier families; earlier schemas keep their original weapon scope. Both backends use the
same canonical candidates, objective policy, budgets, locks and fresh finalist verification.
This remains a restricted
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
It loads the [schema-6 game-data package](native-data.md#schema-migration) once and shares
the snapshot between its catalog and native evaluator. Base values, item-rule grammar,
modifier mappings and the critical-chance cap come from that selected package. The public
`ControlledMaceCatalog::with_data` API binds
candidate identities, generated XML, requirements and realization checks to that dataset.
PoB reference runs still use reviewed default content and reject `--data` selection.
Native XML exports include a `.data.json` companion with actual dataset identity, trust,
package path hint and XML hash; destination/alias checks include that companion.

```powershell
cargo run --no-default-features --locked -- search-experimental --problem examples/mace-search.json --data runs/custom.json --jobs 4 --max-evaluations 10
```

## Native candidate and document paths

Native search defaults to `--native-evaluation typed`. After the fresh template calculation
binds its scenario, preparation compiles immutable weapon, tree and support components
against the same selected dataset and backend. Each weapon's admitted local rolls resolve
once into prepared local stats, reused across its tree/support combinations. The CLI creates
private handles only for candidates that passed full domain and requirement admission. Each search attempt computes
a fresh numeric snapshot from those components, without materializing or parsing candidate
XML or constructing full diagnostic attachments. Preparation does not calculate or cache
candidate results.

Use `--native-evaluation document` to compare the complete document path under the same
objective, locks and budgets:

```powershell
cargo run --no-default-features --locked -- search-experimental --problem examples/mace-search.json --native-evaluation document --jobs 4 --max-evaluations 10
```

Both native modes perform the template calculation and reserved finalist verification
through the full document evaluator. The finalist is materialized, parsed, calculated and
checked against its requested state and baseline identity before its fresh measurements
are compared with the search assessment. The generic [verification hook](search-kernel.md#scheduling-and-limits)
charges this work as the reserved attempt; exporting the verified source adds no calculation.
PoB keeps its document path and rejects an explicitly supplied `--native-evaluation` flag.

The successful prepared numeric snapshot path performs no heap allocation. Its conversion
to owned metric measurements, objective scoring, candidate validation, search archives and
reporting still allocate. Catalog construction also retains its existing bounded XML
materialization and hashing work. This is not an allocation-free optimizer or an expansion
of native mechanic coverage.

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

## Resistance objectives and ascendancy passives

Problem schema **3** allows one of four admitted ascendancy small nodes in addition to
an ordinary entrance. Both available point budgets are explicit 0/1 inputs. The nullable
`ascendancy_node_id` selects a paid physical node owned by the chosen ascendancy; omitting
it means no paid ascendancy node. Schema-2 problems retain their no-paid-ascendancy scope.

```powershell
cargo run --no-default-features --locked -- search-experimental --problem examples/mace-resistance-search.json --jobs 4 --max-evaluations 842 --output runs/mace-resistance-search.json --export runs/mace-resistance-best.xml
```

The [resistance example](../examples/mace-resistance-search.json) maximizes selected DPS
subject to chaos resistance >= 1%. Its 105 tree selections and eight weapon/support choices
yield 840 structural alternatives. Reviewed requirements admit 576 and reject 264, so a
complete run uses 578 calculations. Only the admitted Monk3 passive supplies positive chaos
resistance in the reviewed package; the constraint therefore changes the chosen build.
The 842 ceiling accommodates a fully legal supplied product.

For an exact ordinary-plus-ascendancy allocation, supply:

```json
"tree_search": {
  "ordinary_passive_points": 1,
  "ascendancy_passive_points": 1,
  "selections": [
    {"class_id": 6, "ascendancy_id": "Warrior3", "entrance_node_id": 3936,
     "ascendancy_node_id": 14960}
  ]
}
```

`locks.allocated_passives: [3936, 14960]` requires both paid nodes; the corresponding
unallocated lock excludes either category. Roots remain implicit. Wrong ownership,
unsupported nodes and more than one node per category reject; an explicit selection over
its point budget remains diagnostic rejection evidence and consumes no calculation.

Schema-3 problems produce report schema **4**, retaining the `[weapon, support, tree]` axis
layout and actual data identity/trust. Native tree attachments use version **2**, with
separate used ordinary/ascendancy counts, per-node allocation kinds, physical/effective
IDs and exact configured effects. These observed counts do not establish available budgets.
Custom signed resistance values can change feasibility using `--data`; export companions
retain their actual identity and custom status. See [package migration](native-data.md#schema-migration).

## Configurable support loadouts

Problem schema **4** adds explicit zero/one/two-support arrays and exact loadout locks,
with report schema **5**. See [support search](support-loadouts.md#cli-problem-and-locks)
and [the complete example](../examples/mace-support-search.json). Schemas 1–3 retain their
existing input scopes and output layouts.

## Supplied local weapon modifiers

Problem schema **5** retains the schema-4 loadout and tree inputs and adds supplied normal
or rare items with local flat physical/fire damage, increased physical damage, increased
attack speed and increased critical chance. Reports use schema **6**. The
[local-weapon example](../examples/mace-local-weapon-search.json) combines these item choices
with all seven support loadouts and the admitted class/tree selections. Its synthetic rolls
exercise calculations; they do not establish affix legality or item availability.

The [local weapon guide](local-weapons.md) documents exact source preservation, injected
modifier templates, numeric bounds, optional `LevelReq`, prepared weapon components and
reference realization. Unknown, global, conditional, socket/rune/enchant or unique item
mechanics remain unsupported. Schemas 1–4 reject rare items, modifiers or explicit item
requirements in their template or alternatives before starting evaluation.

## Supported input and locks

`template` is relative to the problem file or an absolute path. XML/share codes pass through
the bounded importer. The legacy template fixes an unallocated Warrior without ascendancy. Expanded templates
accept any admitted class/ascendancy/passive selection. All require one
level-1 quality-0 Mace Strike, one admitted weapon base (reviewed names: Wooden Club or
Smithing Hammer), and level-1 quality-0 reviewed supports. Schemas 1–3 allow no support or
Brutality I; schemas 4–5 allow zero to two supports in an explicit loadout. Supplied items
can change their base, quality 0–20 and item level 1–100. Schemas 1–4 require the original
normal, modifier-free five-line format; schema 5 uses the bounded normal/rare grammar above.
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

Optional `locks.weapon_id` fixes an exact supplied weapon. In schemas 1–3, `locks.support`
fixes `none` or `brutality_i`; schemas 4–5 use the exact `locks.support_loadout` set instead.
These locks are represented in discrete axes and canonical candidate constraints;
replaying validation therefore preserves them. The main skill is fixed by this profile.
Class, ascendancy and tree are fixed in schema 1 and controlled by the expanded
selections/locks above in schemas 2–5.

The CLI checks canonical point/connectivity/ownership rules, then selected-data equip/use
level and attribute requirements before dispatch. Available attributes come from the
resolved selected class; admitted entrance operations do not modify attributes.
A weapon's explicit `LevelReq` replaces its base equip-level requirement when present;
item level remains separate. Each attribute uses the maximum of individual requirements and
the matching support-color aggregate; weapon and support requirements are not added. Requirement evidence reports
available/required values and failed boundaries for choices allowed by the locks. Illegal
choices consume no calculations. If all fail, `empty_legal_domain` reports zero evaluations
without even a template attempt or XML export. Source materialization and diagnostic
`evaluate` remain available for inspection and do not themselves certify legality.

The shared `poe_optimizer_import::controlled_mace` module owns immutable catalogs,
source-preserving materialization and realization checks. `preflight` owns shared structural
evaluation checks. Both are pure Rust, with compatibility re-exports at the old
`poe_optimizer_pob::mutation` and `poe_optimizer_pob::preflight` paths.

## Strategies and accounting

Catalog preparation is bounded to 105 tree choices and 448 weapon/support combinations per
tree (47,040 candidates; legacy inputs remain bounded to 128 combinations per tree). A second cap requires `candidate_count * (template_bytes + 4096) <= 256 MiB` to limit
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
verification, and `total_evaluations` includes all three. Typed component preparation uses
run time but zero calculation attempts. A typed candidate calculation failure counts in
its dispatched search attempt, just as a document calculation failure does. A wholly
infeasible or unavailable result set does not spend the unused finalist reservation.

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
budgets/statistics, ranked assessments and fresh verification. The additive `calculation_path`
field is `native_typed_candidates`, `native_documents` or `pob_documents`; existing report
schema versions and candidate/catalog identities remain unchanged between native modes.

`native_candidate_preparation` is null unless typed preparation succeeds. Its
`admitted_handles` counts the legal candidates, `elapsed_ms` records component preparation
time, and `calculations: 0` plus `caches_results: false` describe what preparation does.
The `footprint` reports component/metric-selector counts, deferred character errors and
owned component bytes; `retained_xml_bytes` and `cached_candidate_results` are zero for
these prepared numeric components. That estimate excludes shared game data, the import
catalog and candidate handles, backend identity, allocator metadata, Arc control blocks
and stack frames. It is not total run memory or a memory limit.

Preparation failure emits an explicit report with no search or XML export. `best_verified` appears only when the top
feasible candidate passes a fresh calculation with matching assessment and realized state.
The diagnostic marker remains set; consistency does not certify complete game legality.

Full document realization, used for the baseline and every reserved finalist in both
native modes, requires the backend's XML export to equal the exact materialized candidate
bytes. It checks selected class/ascendancy, implicit roots, physical paid node,
effective entrance and configured effect evidence, the selected Mace action and exact support
gem projection. Native Mace media version **3** retains the resolved weapon base/quality/item
level and support records, and adds exact `weapon_item` source/roll evidence plus prepared
`weapon_stats`. Realization compares the parsed item diagnostics with the selected payload.
The backend identity and external configuration/placeholders must match the fresh template;
candidate-derived condition tables are not frozen. Native validation never calls the
PoB-normalized baseline binder. PoB realization checks normalized metadata and live
coverage, allowing only the exact derived `LevelReq` and one neutral `ModRange` per admitted
modifier line with ordered IDs and `range="0.5"`. See the
[reference item boundary](local-weapons.md#prepared-calculations-and-verification).

Requested XML export contains the materialized source with only item/support and selected
Build/Spec class, ascendancy and allocation attribute ranges changed;
other source bytes are retained. A fresh verification attempt must pass before writing it,
and existing output files are never overwritten. No export is written for an unverified
or infeasible best candidate. Re-import and native re-evaluation of the exported winner are
covered by tests. Typed/document comparison tests additionally match complete, tight-budget,
empty, infeasible and unavailable searches, including archives, attempt counts, warnings,
fresh finalist assessments and exact XML/data companion exports. Complete and tight-budget
comparisons run with one and four workers.

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
