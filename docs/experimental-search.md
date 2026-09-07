# Experimental controlled search

`search-experimental` searches a supplied finite weapon/support problem using reusable
candidate, scoring and search interfaces. Unlike `search-calibration`, it constructs
parameterized candidates from a structurally validated XML template. It is still a
restricted diagnostic development profile, not the first general joint optimizer release.
The first usable release continues to require all six dimensions, multiple required skills
and exact items, and broad mechanic coverage.

## Run a supplied problem

```powershell
New-Item -ItemType Directory -Force runs | Out-Null
cargo run --locked -- search-experimental --problem examples/mace-search.json --jobs 2 --max-evaluations 10 --timeout-seconds 300 --output runs/mace-search.json --export runs/mace-best.xml
```

The [example problem](../examples/mace-search.json) has four exact weapons and two support
choices: quality-zero and quality-20 Wooden Clubs and Smithing Hammers, with no support or
Brutality I. Its objective maximizes selected hit DPS. The user can supply another typed
objective and constraints through the same policy schema used by evaluation and assessment.
Ten attempts allow one template calculation, all eight combinations and one fresh finalist.

`template` is relative to the problem file, or an absolute path. PoB XML/share imports pass
through the existing bounded importer. Exact item text, support choices, optional locks
and neighborhood settings are in the JSON. Unknown fields and unsupported profile mechanics
are rejected before calculation. Read [controlled mutation support](controlled-mutations.md)
for the structural profile and accepted ranges. The supplied Sorceress/minion build remains
an evaluator fixture; this command does not yet mutate it.

Optional `locks.weapon_id` fixes an exact supplied weapon. `locks.support` fixes `none` or
`brutality_i`. Both are represented in the discrete axes and canonical candidate constraints;
replaying candidate validation does not depend on CLI-only lock rules. Other character/tree/
main-skill fields are fixed by this profile, not silently dropped from the general design.

## Strategies and accounting

The default `--strategy exhaustive` enumerates the bounded supplied product, subject to
`--max-proposals`. It never enumerates the game's whole space. `--strategy guided` starts
from a deterministic complete point and uses the reusable discrete proposer:

```powershell
cargo run --locked -- search-experimental --problem examples/mace-search.json --strategy guided --jobs 2 --seed 42 --max-evaluations 10 --max-rounds 64 --max-proposals 4096
```

The proposer cycles its mutation radius, changes several unlocked axes together, and samples
full random restarts at a configured interval. It does not allocate the Cartesian product
for sampling, and supports up to 128 axes with u32 cardinalities. Frozen/singleton axes never
change. Empty samples can retry until round/proposal/time limits; they must not stop the run
before later coupled moves or restarts. No sampled stall is an optimality certificate.
Actual game-state repair and more advanced island/diversity policies remain future work.

`--max-evaluations` includes the initial template calculation and one attempt reserved for
fresh finalist verification; its minimum is three. Preparation establishes an immutable
scenario, not an independent mechanics golden or a scored seed. Search receives the remaining
attempt capacity and remaining duration. Failed calculations count. Reports distinguish
preparation, search and verification, and `total_evaluations` includes them all.

The deadline begins before problem import/catalog construction. These host operations are
bounded but do not have hard CPU preemption; a deadline check prevents late admission to the
first calculation. Each PoB worker also has a 30-second maximum. Final artifact persistence
is outside the calculation/search duration. Library cancellation remains cooperative; CLI
signal handling and hard process-memory admission are not implemented.

## Evidence and export

JSON retains the template and hash, problem, exact catalogs/payloads, discrete layout and
canonical constraints, backend identity, baseline evaluation, search budgets/statistics,
ranked assessments and fresh verification. Preparation failure emits an explicit partial
report with no search or XML export. `best_verified` appears only after the highest feasible
candidate passes a fresh process calculation with matching assessment and realized state.
It retains `diagnostic_only`; consistency is not a full game-legality certificate.

When requested, XML export contains the materialized source with only the supported item/
support source ranges changed. Other source bytes are retained. Existing output files are
never overwritten. No export is written for an unverified or infeasible best candidate.
The original independent C-host Mace goldens validate generated quality-zero combinations.
Quality-20, changed-level and Pinnacle cases validate structural realization and interactions;
they do not constitute new independently generated numerical goldens.

## Tree extraction foundation

The new offline command runs a separate bounded worker:

```powershell
cargo run --locked -- extract-tree --tree-version 0_5 --timeout-seconds 30 --output runs/tree-0_5.json
```

It verifies the pinned source manifest and exact executable data literal, then exports
4,914 nodes, eight classes, 23 ascendancies, shared physical roots and override provenance.
All 14 dangling connections remain explicit. The snapshot preserves typed Lua keys/values
as owned serialized data; consumers need no live Lua state. Read [tree-data.md](tree-data.md)
for source hashes, bounds, graph semantics and unsupported mechanics. The Rust extractor/
snapshot implementation currently lives in the PoB adapter; a portable consumer module is
a separate integration step. No site scraping or bulk remote extraction is involved.

The next expansion is to connect this source data to validated class/ascendancy/passive
mutations, then broader equipment and skill/supporting-skill catalogs. Automatic overrides,
point/resource legality and exact realized-state comparisons must survive those extensions.
