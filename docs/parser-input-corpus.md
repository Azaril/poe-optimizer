# Real-build parser input corpus

This corpus records the actual public modifier-parser calls made while pinned PoB loads
all five supplied original builds. It supports the [execution-model investigation](rule-execution-model-investigation.md)
and identifies dependencies for [real-build integration](real-build-rollout.md).
It does not make any original build evaluable by the production native backend.

## Reproduce

```powershell
$env:POE_PARSER_BREADTH_OUTPUT = 'C:\code\poe-optimizer\runs\parser-input-corpus-new'
cargo test --release --locked -p poe-optimizer-pob --test source_program_parser_breadth -- --nocapture
```

Use a fresh absolute output directory; reports are created without overwriting earlier evidence.
The manifest is `tests/fixtures/builds/breadth-20260908/index.json`. The harness verifies each
original XML hash and uses the pinned submodule, without rewriting either. Its five child
processes isolate the reference host's working-directory/global state. Native parser sessions
run in process; this is not a subprocess design for optimizer candidates.

## What is compared

An observer installed after source initialization records ordered parser attempts during the
unchanged XML import. It preserves raw declared `line` and `isComb` parameter values, duplicate
attempts, non-item callers and actual Item identities. Final inventory joins include every item
set and receiving slot; temporary item objects remain visible without false final-item joins.
The observer does not intercept return packs or recover original argument arity. The component
replay supplies exactly those two declared parameters.

Each distinct parameter pair within a build is replayed from a fresh native session against the
original parser as a forced cache miss and a subsequent hit. Complete supported plain result
and cache graphs are compared, including types, number bits, arbitrary byte strings, arities,
aliases and miss/hit history. Unsupported operations and resource/comparison limits retain
explicit outcomes. Successful empty/no-match results count as matches; that count alone is
not the number of meaningful item effects implemented.

The source parser can mutate shared dictionaries. A guard acquired before source loading checks
captured cells, table entries/order, raw lengths, metatable contents and the closed set of consumed
global bindings. Cache bindings are restored after each probe; any other observed mutation requires
coherent owner/input/catalog recapture before the next probe. This does not restore physical cache
allocation history or prove sequential native state mutation/invalidation. Each reported generation
binds the captured input and program owner used by its probes.

The graph comparison has explicit node, row, depth and byte bounds on both sides. Native/source
canonicalization uses the same traversal order. A separate report-detail budget can omit large
already-compared graphs, retaining their hash and size; it never substitutes a hash for a comparison.
Source errors are separate from native unsupported/resource failures and are not declared matching
error prefixes. A fresh unobserved source import provides a control for public scalar outputs;
that control does not compare every nested source output.

## Checkpoint evidence

The complete release-mode run in `runs/r2x-breadth-01/corpus-01/` passed on Windows
against parent `901a9af8e18fa1e38fe867ba385c6dd28a59c213`. It observed 1,375 parser attempts
and replayed 760 distinct parameter pairs **counted separately within each build**:

| Original line | Retained saved items | Attempts | Distinct pairs | Matched miss and hit | Native unsupported |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 16 | 184 | 128 | 93 | 35 |
| 2 | 34 | 378 | 185 | 135 | 50 |
| 3 | 17 | 212 | 151 | 113 | 38 |
| 4 | 21 | 229 | 147 | 111 | 36 |
| 5 | 28 | 372 | 149 | 106 | 43 |
| Total | 116 | 1,375 | 760 | 558 | 202 |

Of the 116 retained saved items, 114 have an actual item-context parser call; item 18 in
build 2 and item 25 in build 5 have none. They remain in the inventory accounting. All 169
nonempty saved item-set/slot bindings match the retained identities.

No comparison mismatch, unexpected native error/success, hit failure, source error or resource/
comparison-bound result occurred. The 202 unsupported pairs reached three reported first stops:
125 unavailable current-session traversal orders, 51 unavailable source table key/value accesses,
and 26 positive tails for reserved-string template constructors. These are first-stop categories,
not counts of independently implemented mechanics or evidence that each needs a new VM feature.
The 51 source-entry failures are observation/environment-coverage frontiers, not proof of
missing numerical kernels.
The corresponding observed-attempt mapping is 1,064 matched and 311 unsupported; attempts sharing
an input are not separate native executions. Each original used one capture generation with no
reported semantic-state mutation. Change detection has focused unit coverage; these original inputs did not exercise the
coherent recapture branch.

The executable SHA256 is `3ceddc7fc723347ae6939804b8fc5db4d75cf10327005c8f7ce00b4cff5bcedb`.
The checkpoint binds harness/input hashes and all five reports. Elapsed wall time was 336.8 seconds
for controls, observed imports, captures, guards, native replays and serialization combined;
this is one validation run, not a candidate-throughput measurement. Six observer tests, seven
state-guard tests, five graph/parameter tests, the existing five-original configuration lifecycle
regression (including a reused-build import), strict workspace Clippy and formatting pass.
The original inputs, bundled schema-29 data, dependency lock and PoB pin are unchanged.

## Cost and next integration boundary

The observer, state guard, capture/replay harness and their adversarial tests are reference/parity
tooling. They add no production evaluator, schema, bundled game definitions or runtime dependency.
The six new test/support files contain 2,477 physical lines; the shared hook adds 51 net lines.
This is a descriptive source count, not a runtime-size or maintenance-effort measurement. Their
implementation and maintenance costs belong in A1 alongside acquisition and the native interpreter costs. The A2 comparison must assess whether a domain model can preserve consumer
behavior with less total machinery, including the adapters and tests it would require.

The next production dependency is complete item modifier assembly at `ItemLoadProvider`, with
an owned assembled result and source-ordered registration connected to `NativeBackend::prepare_view`.
Current inspection evidence cannot substitute for base/per-slot modifier lists, local item data,
requirements or granted skills. Twister and Skeletal Sniper must share this general integration;
configuration, effective skills/supports, actors and actions still need their own complete producers.
The production path still has Spark/Mace compatibility-profile restrictions, and complete native
coverage of the five original builds remains **0/5**.
