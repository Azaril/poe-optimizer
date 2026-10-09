# Joined Sniper component replay

`owned-sniper-replay.json.gz` contains the public typed inputs of the existing
joined Sniper test graph, exported from
`runs/owned-intrinsic-added-attack-publication-02/package`. It is test data, not a
production release or an admitted complete build. Original05 still has four
selected input obligations and incomplete mechanics coverage.

The fixture exercises imported Crown/Solar item inputs, source preparation,
population and intrinsic attack data, selected passive contributions, Offering
activation and non-stacking application, mixed increased/MORE consumers, and
exact Action Command queries on Basic plus all three Gas Arrow stat sets.
It retains two Sniper recipients and two Offering occurrences. Its explicitly
finite contributor inventories, activation/type facts and excluded mechanics
have the same scope as `owned_sniper_item_attack`; they do not close production
coverage. The original build XML and all-five import validation remain separate.

The snapshot stores schema, original validated rules, routing, stages, support
preparation/input/receiving declarations, build, scenario and query inputs. It
stores no private execution graph, scratch, evaluated results, PoB objects or
Lua state. Replaying it runs the normal Core, Data and Engine constructors and
checks all dependency identities before native compilation. The compiler's
normalized rule copy is deliberately not substituted for its source artifact.

The ordinary Engine tests need neither a local release directory nor PoB
execution. They compare against the existing retained source vectors, check the
snapshot's damage consumers/queries and Gigantic/population programs against current owned
data, and exercise independent quality, source removal, inactive/missing Offering
inputs, scratch reuse and four-worker equality of full reports. These remain
component parity checks, not final physical damage or complete build parity.
Fourteen retained Command vectors compare original-call results for every mode
and branch-removal control on both recipients. Gas level projection comes from
the same real population graph. The fixture also includes current numeric Life
copy and its checked source membership, preserving the finite Life scope.

Run the ordinary tests with:

```text
cargo test --locked -p poe-optimizer-engine --no-default-features --test owned_sniper_replay
```

To regenerate, set `POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE` to the current
checked release and `POE_OPTIMIZER_TEST_SNIPER_REPLAY_OUTPUT` to a new output file,
then run:

```text
cargo test --locked -p poe-optimizer-cli --no-default-features --test owned_sniper_item_attack export_joined_sniper_replay -- --ignored --exact --nocapture
```

The exporter rebuilds the existing fixture from its authenticated source/release
inputs and requires the decoded replay's full report to equal the original
report before writing. It refuses to overwrite an existing file. Review the new
fixture and replace the checked-in file explicitly. Development-format changes
require regeneration; there is no old-format decoder.

Compression is ordinary gzip over JSON, using the existing Rust compression
backend with fixed timestamp and OS header. Decoding is bounded to 8 MiB; the CI
test requires deterministic re-encoding. Compression is a test dependency only.
Numerical JSON uses the workspace's exact float-roundtrip configuration.

The current October 9 V27 fixture has 927 definitions, 620 owners and 34 stages.
It is 138,584 compressed bytes and 5,412,816 JSON bytes, with SHA-256
`b54d4468cb28560b32a22d87aac4135f36ec176c9aa5cf7d4a64c8e3244e2dca`.
The exporter verifies full-report equality before writing; the ordinary native
test checks deterministic re-encoding.

The prior intrinsic added-attack source contributes three Stats, two programs,
one exact Basic route and its stage dependencies. The current refresh joins the
published minion Life Increase receiver and the two remaining actual selected
Life passives, preserving the four existing paired Life/Damage sources. Six
carrier records total 44, then one received contribution reaches each exact
Sniper. Source delivery precedes the frozen-channel read. The separate joined
Life-query fixture retains the canonical member and runs its diagnostic probes
afterward; it derives no canonical final Life pool.

All fourteen ordinary replay tests pass. Four Life tests cover exact published
sources/recipient lineage, the seven retained 44/38/34/0 source cases, missing
and actual Partial authority, early reads, quality changes, reordered inputs
and fresh/reused/four-worker equality. Removal controls compare the Increase
channel only: the original tree removal of 229 also removes Gigantic 46365, and
removal of 1218 also removes cooldown 14945. These finite projections do not prove
legal optimizer mutations or final Life/MORE parity. The preceding ten damage,
Command and Offering checks remain intact.

Regeneration: `runs/owned-sniper-replay-life-increase-01.log` (30.23s), from
input `c4c38e18383f708bc36d4a571bb260d0afadf0e69c1e7a6881ef80b8a0098ba3`.
An independent inverse comparison verifies the two added passive definitions,
owners, allocations and support-discovery rows; the received program and stage
metadata; and dependent digest changes. Every previous program and unrelated
field is unchanged. Scenario, queries, routes and source-property content are
identical. No production artifact changed.
The canonical publication is broader than this finite graph. Earlier refreshes
are recorded in the implementation history; no old-format decoder is retained.
