# Native passive and equipment assembly

`search-build` searches connected passive allocations, physical attribute choices,
supplied equipment and reviewed support loadouts with the native Rust evaluator. It uses
immutable source components and lazy candidate admission; catalog storage does not grow
with every combination of classes, passives, items and supports. PoB remains an explicit
reference backend for evaluation and parity tests.

This is a bounded integration of the intended optimizer. It currently covers the Spark
and Mace profiles in [native backend](native-backend.md), including all pinned class and
ascendancy identities. Selecting an identity does not imply complete mechanic coverage.
The supplied minion build, arbitrary active/supporting skills, reservations, broader
receiving defences and complete game legality remain outside this scope.

## Run a search

The native-only executable needs no PoB checkout, Lua runtime or subprocess:

```powershell
cargo run --release --no-default-features --locked -- search-build --problem examples/passive-equipment-search.json --jobs 4 --max-evaluations 1000
```

Use `--output runs/search.json --export runs/search.xml` to save the report and a freshly
verified feasible finalist. Output directories must exist and every output path must be
new. XML exports include a `.data.json` companion identifying the selected data package.
No XML is written when no feasible finalist passes fresh verification. File publication is
not transactional: an I/O failure can leave an already written file.

The command accepts `--data` and `--data-sha256`, uses a local Rayon pool with 1–256 workers,
and defaults to 1,000 full calculations, 10,000 proposals, 128 rounds, a 300-second deadline
and seed 0. At least three calculations are required by the CLI. Budgets and objectives
remain caller choices; the example's damage objective and Spirit floor are illustrative.
`--native-evaluation document` repeats full XML evaluation for intermediate candidates;
the default `typed` path evaluates admitted numerical inputs directly. The new command is
explicitly native-only. The legacy `search-experimental --backend native|pob` command and
problem schemas 1–6 remain available.

## Problem schemas 7, 8 and 9

[Local armour problems](local-armour.md) use schema 9/report 10 and add fixed helmet,
glove and boot choices, with explicit armour/evasion rating metrics. Earlier schemas
retain their source admission boundaries.

[The example problem](../examples/passive-equipment-search.json) is the complete runnable
schema-7 shape. [Receiving-defence problems](receiving-defences.md) use schema 8 with
the same fields and new authored defensive source scope. Its template path is relative to the problem file. All unknown fields reject.

| Field | Purpose |
| --- | --- |
| `template` | Source PoB XML/share-code file within the admitted native profile. |
| `equipment` | Additional physical item instances: unique `instance_id`, `pob_item_id`, exact `item_text`. Imported inventory is retained separately from equipped effects. |
| `objective` | Existing extensible metric objective and hard constraints, with explicit units and violation scales. |
| `constraints` | Core required skill/item sets, independent class/ascendancy/passive/equipment/skill-group locks and explicit point/resource budgets. |
| `attribute_locks` | Required option per physical node and forbidden options per physical node. These augment passive allocation locks. |
| `initial_allocations` | Optional class/ascendancy, ordinary/ascendancy paid-node sets and physical-node attribute choices used as additional starts. |

Required items name physical instances, not just item bases. The initial admitted profile
has one active skill; the core constraint model retains multiple required skill/item sets
for future coverage. Two required physical items can be locked across the weapon and amulet
slots. Contradictory locks or impossible requirements do not consume full calculations.
Point budgets come from the caller; character level does not infer a point entitlement.

Starts include the source, explicit allocations and class/ascendancy root restarts. Bounded
repair assigns required items to compatible slots, applies skill/attribute locks and finds
source-view-compatible paths to required passives. The item assignment uses augmenting
paths so a flexible item cannot steal the only slot available to a required restricted
item. Passive path repair is a heuristic, not a proof of infeasibility or an optimal
Steiner-tree solver. Requirement failures may be improved by equipment neighbors.

Proposals add/remove a connected frontier node, change attribute choices, swap equipment
or supports, or change class/ascendancy. Every proposal passes complete admission. Generation,
rounds, preparation and evaluation are bounded; deterministic shuffling and restart seeds
provide variation. Coordinated larger mutations, random/greedy baselines and 5–30 minute
optimizer-quality benchmarks remain future work. Empty or exhausted samples do not prove
that the full legal domain is empty or that the best-found result is globally optimal.

## Data and source identity

Package schema 9 / `poe2-native-profiles-v9` supplies numeric values, requirements, implicit
ranges, modifier grammar and source-view effect records. Tree schema 3 records 4,109
structural ordinary nodes and 4,758 effective source views. Of these, 1,270 have fully
admitted effects and 3,488 carry explicit exclusions. The four previously reviewed
ascendancy nodes remain supported. A source view contains its physical key, selector,
effective node ID, source stat lines and source digest.

Resolution selects automatic class/ascendancy replacements and explicit Strength,
Dexterity or Intelligence options before checking capability. Attribute choices belong
to physical nodes: two allocated nodes may legitimately resolve to the same effective
attribute record and both contribute. Disconnected allocations, wrong ownership, missing
or dormant attribute overrides, duplicate/conflicting choices and unsupported selected
views reject. Unknown effects never disappear while traversing supported structure.

Custom packages may alter supported numeric records and grammar values within the
reviewed capability-key ceiling. They cannot admit excluded views, invent structural
content or bypass the exact source-version guard. The original PoB pin, complete source
snapshot and six independent calibration pairs remain fixed. Broader source-version
compatibility is a separate migration gate. See [data loading](native-data.md) and
[optional source extraction](game-data-extraction.md).

## Equipment and modifier layers

Equipment parsing reuses the injected actor grammar used for configuration. Exact input
lines retain rule/capture evidence and item identity. Local weapon modifiers are consumed
once by the local weapon pipeline; surviving global actor records enter shared actor
preparation. Unknown, ambiguous, partial or unsupported lines reject.

The current slots are `Weapon 1` and `Amulet`. Mace requires one of the two admitted mace
bases; Spark currently admits jewellery only. Seven source-derived amulet bases are
supported: Amber, Jade, Lapis, Bloodstone, Solar, Lunar and Pearlescent. An amulet has quality 0 and exactly
one implicit matching its configured range; normal and rare supplied payloads can contain
reviewed global attribute/resource/accuracy lines and the [receiving-defence subset](receiving-defences.md). Stellar Amulet remains excluded because
its source emits additional unsupported `All` bookkeeping. Untagged weapon Accuracy also
rejects because PoB rewrites it with hand-specific conditions; tagged attribute-condition
Accuracy and jewellery Accuracy follow their admitted source behavior.

PoB assembles the local actor modifier layer in configuration, equipment-slot, then
passive order. The native path combines borrowed compiled fragments in that same layer,
using Weapon 1 before Amulet. Fragment boundaries must not create extra modifier database
layers or extra MORE rounding. Parent/local layers remain explicit. General order-sensitive
passive MORE/OVERRIDE/FLAG records are not admitted; complete passive BASE/INC records are
checked against actual `PassiveTree.ProcessStats` output.

Equipment requirements use the resulting actor attributes after all selected gear and
passive effects, as does skill calculation. There is no separate approximate requirement
model or invented equip/unequip iteration. Supplied rare rolls validate calculation syntax;
they do not establish obtainable affix combinations, crafting cost or acquisition legality.

## Library boundary and costs

| API | Responsibility |
| --- | --- |
| `ControlledBuildCatalog` | Parse the source and item alternatives once; retain source components, compiled actor programs and the structural catalog. |
| `ControlledBuildDomain` | Apply caller locks/budgets, resolve every selected source view, combine actor fragments in task-local `ActorScratch`, check requirements and issue private admitted handles. |
| `AdmittedBuildSelection` | Bind canonical selection, full actor, character and requirement evidence to the exact catalog/compiled data instance. It cannot be deserialized or forged from arbitrary numerical inputs. |
| `NativeBackend::prepare_controlled_build` | Resolve the strict profile, fixed scenario, metric selectors and numeric weapon components once, without a Cartesian candidate/result table. |
| `PreparedBuildCandidates::calculate/measure` | Fresh skill calculation from an admitted handle; successful pure calls allocate nothing and use no clock or OS services. |
| `NativeBackend::evaluate_controlled_build` | Same typed calculation with backend/data identity and cooperative deadline checks. |
| `materialize` and `validate_native_realization` | Produce exact source XML and verify complete native result/export/tree/equipment/resource/condition evidence. |

Admission recomputes actor resources for each changed candidate. The handle retains those
resources for repeated skill calculations. The pure-call allocation result does not cover
admission, graph checks, owned metric conversion, archives, XML materialization or reports.
Catalog/engine footprints expose their separate retained components. Compact search
identity keys release old prepared candidate payloads after bounded archives no longer
need them; deduplication still retains bounded canonical identity hashes.

`ActorModifierLayer` combines borrowed `CompiledActorModifiers` programs. Programs and
prepared actor resources bind to the exact `CompiledGameData` instance. Task-local scratch
resets between calls, while immutable configuration is shared across Rayon workers. The
calculation seam remains portable Rust, independent of XML and PoB hosting. Browser APIs,
WASM runtime tests and browser worker scheduling remain separate implementation work.

## Results and verification

Report schema 8 records the backend/data identities, trust, normalized problem, catalog
and numerical preparation footprints, source-assembly attempts, full-calculation ledger,
seed repair diagnostics, feasible/infeasible archives and fresh verification. Actor-only
admission is recorded separately from full build calculations; it is work, not a free
numerical operation. When a legal initial seed exists, a full source baseline consumes
one attempt. A failed imported numerical calculation is recorded as `baseline_error` and
spends that attempt without suppressing valid alternatives. Otherwise the search can
inspect repaired neighbors without a baseline.
Every search calculation and finalist verification consumes the remaining shared budget.

Only a feasible candidate whose fresh complete document assessment agrees with its search
assessment can be exported. Verification checks source XML, backend/data identity,
physical/effective passives and attribute choices, item/local/global evidence, support
selection, actor resources and context. Intermediate document mode uses the same checks.
The lazy PoB normalization/realization adapter is not implemented; independent full-build
PoB comparisons and export reimports remain the oracle for this expanded native scope.

Validation includes original source modifier and actor functions, all admitted passive
views, complete native/PoB build pairs, exact native-export PoB reimports, typed/document
comparisons, custom-data isolation, foreign-handle rejection, malformed state checks and
serial/parallel CLI agreement. Numerical parity applies only to the stated admitted
mechanics. Results retain `diagnostic_only` until broader legality and coverage are proven.
Current test counts, performance distributions and next work live in
[implementation.md](implementation.md).
