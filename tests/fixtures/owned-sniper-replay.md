# Joined Sniper component replay

`owned-sniper-replay.json.gz` contains the public typed inputs of the existing
joined Sniper test graph, exported from
`runs/owned-mana-override-publication-01/package`. It is test data, not a
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

The October 9 V27 fixture is 138,049 compressed bytes and 5,397,012 JSON bytes.
The exporter compares the decoded replay's full report against the original
graph, and the ordinary native test checks deterministic re-encoding. Its gzip
SHA-256 is `5ae76529f235385e2af8611972cccf4ce9553c708e139e87fc5ab43f513796e0`.
The latest refresh changes only the operations capability and its dependent
identities; the finite graph's 922 definitions and 615 owners retain their
contents. Actual Mana override component checks extend these public inputs in
the existing CLI `owned_intelligence_mana` test, without adding Blood Magic's
unfinished mechanics to this fixture's claimed coverage.

Revalidation after the Ascendancy-root and generated-Warrior publications
reproduces this exact fixture (30.03s; `runs/owned-sniper-replay-warrior-01.log`).
It used input `d8a030ae8dd7f98b237511e010f654a89c87a9bf4d55fe52dfebff8d0857a8c0`.
The final `runs/owned-warrior-correspondence-publication-07/package` reproduces
seventeen artifact files from that provisional Publication03 package byte-for-byte.
Only `release.json` differs: its input and final evidence-authoring digests
reflect the reproduced Source04 witness after parent-harness formatting. All
other receipt fields match, including definitions, rules and compiled identities.
No fixture bytes changed.
The current package is broader than this deliberately finite numerical graph.
