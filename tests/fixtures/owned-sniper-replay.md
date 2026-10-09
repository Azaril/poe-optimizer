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

The current October 9 V27 fixture has 925 definitions and 618 owners. It is
138,407 compressed bytes and 5,405,368 JSON bytes, with SHA-256
`c0aa1c70b4b5e42eba0c780d98a7b9d48acfa862825ff862644843dbe5c63614`.
The exporter verifies full-report equality before writing; the ordinary native
test checks deterministic re-encoding.

The intrinsic added-attack source adds three Stats, two programs, one exact
Basic route and their stage dependencies. Every pre-existing definition, slot
and program is retained exactly. Build, scenario and queries are unchanged;
support preparation/input/receiving content changes only its dependency
identities. The snapshot retains explicit Actor-producer → routing → Action
ordering and frozen channels.

Five new ordinary checks cover exact source/route preservation, identity
inactivity, zero and changed injected scales, missing profile/authorized
producer/route, Partial refusal, premature reads, reordered occurrences and
fresh/reused/four-worker equality. They do not construct a foreign Actor or
claim a final added-damage aggregate. All ten replay tests pass.

Regeneration: `runs/owned-sniper-replay-added-attack-01.log` (29.17s), from
input `c4c38e18383f708bc36d4a571bb260d0afadf0e69c1e7a6881ef80b8a0098ba3`.
The canonical publication is broader than this finite graph. Earlier refreshes
are recorded in the implementation history; no old-format decoder is retained.
