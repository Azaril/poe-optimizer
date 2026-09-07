# Search kernel and calibration runner

The host-side `poe-optimizer-search` library composes a candidate domain, a calculation
adapter and the shared `ScoringPolicy`. It has no PoB, Lua, CLI or game-metric dependency.
The [candidate model](candidate-model.md) remains in portable core; the separate search
crate owns Rayon and OS scheduling. This is an experimental kernel, not the complete
six-dimension PoB optimizer described in [design](design.md).

## Search contracts

`SearchPlan::Finite` processes exactly the supplied candidate list. Its completion reason
is `finite_domain_processed`: every eligible distinct state in that list received an
attempt. Rejections, unavailable results or calculation failures still prevent any claim
of a complete optimum. This never means the game's domain was exhausted.

`SearchPlan::Explore` starts from supplied seeds. A `SearchDomain` validates complete
states and generates a bounded next set from separate feasible and infeasible beams,
a round index, and a deterministic seed. The proposer owns coordinated mutation,
repair and restart semantics; the kernel never changes locks or invents missing mechanics.
By default an empty proposal set means `search_stalled`, not exhaustive completion.
A stochastic domain can declare `can_propose_after_empty()` so empty samples retry under
the same round/time limits before later mutation radii or restarts. Proposal and
round limits bound duplicate-only or invalid-only searches. The reusable [discrete proposer](experimental-search.md#strategies-and-accounting) now
provides coupled moves and restarts without materializing its Cartesian product. One test domain searches
six binary dimensions and needs a two-choice move to cross an interaction trap; its
best result is compared with all 64 states. These are invented mechanics, separate from
real PoB calibration.

Available, finite assessments enter separate archives. Each archive orders normalized
constraint shortfall, violated-constraint count, oriented scalar score, then canonical
candidate identity. Feasible and infeasible states never share a score cutoff; a failed
strict equality remains infeasible even when its distance is zero. Unavailable/nonfinite
primary values or aggregate violations cannot win. Retained results preserve their
`diagnostic_only` flag. Feasible means the configured metric constraints passed, not
that every game mechanic was verified.

The compiled objective is the current production policy. Custom `ScoringPolicy`
implementations must return coherent, finite, versioned evidence under their stated
specification and stable measurement/constraint order. The kernel does not infer how
a custom policy derived its values. Finalist checks compare the full assessment,
including exact policy and metric metadata and numerical evidence at absolute `1e-8`
or relative `1e-9` tolerance. Tolerance never relaxes constraint comparisons.

## Scheduling and limits

Rust CPU evaluators use a per-run local Rayon pool. External-process evaluators use
bounded scoped supervisor threads while Rayon is idle; no Lua object enters this API.
Batches have at most `jobs` calculations and results merge in candidate order. Complete
rounds produce the same choices with one or multiple jobs when the backend and inputs
are deterministic. Deadlines, cancellation and timing-dependent failures can truncate
different prefixes. The seed does not guarantee reproducibility across those cutoffs.

One startup-inclusive deadline covers search and fresh verification. Domain validation,
proposal generation and evaluation receive a cooperative control object. Stops are
checked between calls and before dispatch. External adapters must enforce a process
deadline and reap their children. The generic kernel cannot forcibly preempt a Rust
callback that ignores its control; scoped work is joined before returning. Cancellation
is currently a library `AtomicBool`, not a CLI signal-handling or desktop run-control API.

The attempt limit includes failures and reserved finalist calculations. Reservation
reduces exploration capacity even if no feasible finalist exists. Every evaluator call
must calculate fresh: the kernel deduplicates ordinary proposals, then deliberately
calls the evaluator again for verification. Backend adapters must not silently cache
those calls. A stopped/late result does not enter the search archive. Verification errors
and late results remain visible and count in the aggregate statistics. Search termination
records the exploration phase; verification has separate entries, so completion of the
finite list does not imply verification completed before the deadline.

A run retains a bounded seen-set, bounded archives and at most 32 error samples. The
proposal budget bounds candidate count, not candidate byte size. There is no persistent
cache, worker reuse, cross-run resource manager, process-memory admission or hard memory
limit yet. Multiple simultaneous runs must be budgeted by their host. Oversized callbacks,
shared memory accounting, throughput scheduling, persistent checkpoints and progress events
remain implementation work. A host-side search crate is not a WASM execution claim;
portable core/native calculation builds are checked independently.

## Calibration CLI

`search-calibration` is a developer integration harness over the four exact Mace source
fixtures, using the [calibrated PoB registry](pob-candidates.md). It runs the normal fresh
process evaluator, rejects requested-versus-realized state drift and backend identity
changes, and emits versioned JSON with candidate/catalog identity, settings, assessments,
failures and fresh verification. It can export the unchanged source XML for a verified
feasible finalist. Diagnostic coverage remains diagnostic after verification.

The registry deliberately accepts only independently calibrated byte identities. It
currently has no user build input, arbitrary tree/class/ascendancy mutations, encounter
selector or generic equipment pool. That restriction is an integration gate; it does not
change the first usable optimizer's joint search scope.

```powershell
cargo run --locked -- search-calibration --objective examples/calibration-objective.json --jobs 2 --max-evaluations 5 --output runs/calibration-search.json --export runs/calibration-best.xml
```

Five attempts permit four distinct alternatives and one independent finalist calculation.
With a smaller budget, the JSON explicitly records a partial search. If no feasible
candidate passes fresh verification, an export is omitted with a reason. Existing output
files are never overwritten. The complete four-case optimum for selected hit DPS is
Smithing Hammer without Brutality (`18.208694`). Among the two Brutality cases, Wooden
Club wins (`13.6565205` versus `11.0552785`), demonstrating a real support/weapon interaction.
Neither comparison is a recommendation for an actual character.
