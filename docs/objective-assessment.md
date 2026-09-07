# Configurable objective assessment

Objective assessment is the first executable scoring policy. It operates on typed metrics
through [the shared core API](../crates/poe-optimizer-core/src/objective.rs), independently
of Lua and the calculation backend. [Implementation status](implementation.md) records the
current validation checkpoint; [the design](design.md#objective-and-constraint-contract)
defines the richer target policy and joint optimizer.

## CLI workflow

Evaluate once with an explicit scalar objective and constraints:

```powershell
cargo run --locked -- evaluate example.import.txt --objective examples/evaluation-objective.json --output runs/assessed.json
```

Use a new output path; create `runs` first if necessary. The example maximizes PoB EHP and
requires three capped elemental resistances of at least 75%. The supplied build's lightning
resistance is 71%, so that constraint fails with a shortfall of four percentage points.
These are editable example choices; no metric or threshold is inserted by the tool.

Change thresholds or objective direction and assess the saved calculation without loading
PoB or calculating again:

```powershell
cargo run --locked -- assess runs/assessed.json --objective examples/evaluation-objective.json --output runs/reassessed.json
```

`evaluate --objective` adds `objective_assessment` to its schema-2 report. With an explicit
`--metric` filter it also requests the metrics needed by the objective, preserving the
explicit filter's other queries. With no filter the complete declared catalog is returned.
The CLI validates the objective against backend capabilities before launching its worker.

`assess` accepts schema-2 evaluation reports and emits a schema-1 diagnostic assessment
artifact. It uses recorded metric units/versions, even for a backend or custom metric the
current PoB adapter does not know. A filtered report cannot assess an unrecorded metric;
obtain a new evaluation containing that metric. An unavailable recorded value remains
unavailable. Saved assessment does not change encounter assumptions or skill selection.
Use a fresh evaluation for those changes. Existing output files are never overwritten.

## Specification and mathematical semantics

The supported specification has `schema_version: 1`, an explicit `scalar` policy, and zero
or more constraints. [The runnable JSON example](../examples/evaluation-objective.json)
contains all fields. Each metric uses its registered ID and an actor (`player` or
`selected_minion`), plus an explicit unit. Inspect `poe-optimizer metrics` for the catalog.
One assessment addresses the selected actor/actions and encounter from one evaluation.
Multiple independent scenarios/selectors and reducers remain future policy work.

The scalar policy supports `maximize` and `minimize`. It preserves the observed value and
also records an oriented score: the original value for maximization, its negative for
minimization. Higher oriented scores mean better objective values. Objective direction
does not alter hard constraints. A policy can reference any registered applicable metric;
it contains no special handling for DPS, resistances or other game-specific names.

Each constraint has a unique nonblank `id`, metric, unit, operator, finite `threshold`, and
finite positive `violation_scale`. Supported operators are `>=`, `>`, `<=`, and `<`.
Contradictory lower/upper bounds on the same actor/metric are rejected, including equal
bounds where either side is strict. Equal inclusive bounds express an exact target. The
compiler checks schema, units, metric registration, IDs and a 1,024-constraint limit.
Unknown JSON fields, policy kinds and operators fail instead of selecting a fallback.
CLI configuration files are bounded to 64 KiB.

Comparisons use exact unrounded `f64` values. Calibration tolerances are never used to
relax a constraint. A value of 75 satisfies `>= 75` and fails `> 75`. At failed strict
equality the numerical gap is zero; `status: violated` and `strict_boundary: true` preserve
the failure. No arbitrary epsilon is added. The registry currently has no universal
range/cap metadata; it does not prove every individually valid bound is achievable.

For a violated lower bound, shortfall is `threshold - value`; for an upper bound it is
`value - threshold`. A satisfied constraint has zero shortfall. Normalized violation is
shortfall divided by the user-supplied scale, and the total is their sum. Scaling expresses
relative distances only; it never turns a failed constraint into a pass. Floating-point
overflow is preserved as classified nonfinite evidence. A zero total alone does not prove
constraints passed, particularly at strict equality.

## Availability, evidence and API boundaries

| Overall status | Meaning |
| --- | --- |
| `constraints_satisfied` | The primary metric and every constraint observation are finite, and every constraint passes. |
| `constraints_violated` | Required observations are finite and at least one constraint fails. |
| `unavailable` | The primary metric or a constraint observation is absent, explicitly unavailable, NaN or infinite. Individual constraint results remain visible. |

Nonfinite values never become zero or an automatic objective/constraint win. Their original
classification is retained, including positive infinity from modeled immunity. The initial
policy does not opt into infinity comparison. Overall unavailability also makes the total
normalized violation unavailable; individual finite constraint shortfalls are still reported.

These statuses describe **objective observations only**. They do not certify mechanic
coverage, legality, required skills/items, transition feasibility, combat uptime or an
optimized recommendation. Both CLI workflows retain diagnostic context and coverage. The
current PoB results remain diagnostic even if all configured constraints pass.

The compiled `ScoringPolicy` returns its required metric queries and assesses measurements.
The resulting artifact contains the complete resolved specification, required measurements
with units and metric schema versions, primary value/oriented score, per-constraint results,
shortfalls and overall status. Search implementations can consume this interface without
knowing PoB field names. Richer policies will implement explicit capabilities rather than
silently treating Pareto or ordered priorities as scalar objectives.

`EvaluationResult::validate_recorded()` checks the entire saved result before assessment:
recorded options, numeric context/timing/coverage, unique nonblank metric queries, positive
metric schema versions and finite-tag integrity. The live evaluation engine uses the same
structural validator. It does not authenticate a file or compare observations with today's
rules or catalog; the source/backend/context records remain evidence supplied by that file.
Raw backend attachments are not read by scoring. This preserves a path to native and browser
hosts without requiring a PoB checkout for saved assessment.

## Validation and next steps

Core tests cover custom metric names and non-1 schema versions, both directions, strict and
inclusive neighboring floats, contradictory bounds, missing/nonfinite observations,
normalized/overflow shortfalls, malformed catalogs/results and unsupported specifications.
CLI tests exercise a fresh evaluation plus saved reassessment, required-metric collection,
rejection before worker startup, alternate backend data and output preservation.

Candidate and lock models, game legality, coupled mutation/repair, parallel search, multiple
scenarios, composite/Pareto policies and report visualization remain separate work. The
illustrative [objective.toml](../examples/objective.toml) describes that broader future run
configuration; it is not accepted as the current JSON assessment specification.
