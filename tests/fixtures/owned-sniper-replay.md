# Joined Sniper component replay

`owned-sniper-replay.json.gz` contains the public typed inputs of the existing
joined Sniper test graph, exported from
`runs/owned-mixed-minion-damage-publication-06/package`. It is test data, not a
production release or an admitted complete build. Original05 still has four
selected input obligations and incomplete mechanics coverage.

The fixture exercises imported Crown/Solar item inputs, source preparation,
population and intrinsic attack data, selected passive contributions, Offering
activation and non-stacking application, and the mixed increased/MORE consumers.
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

The October 8 fixture is 135,637 compressed bytes and 5,315,517 JSON bytes.
Two independent exports agree byte-for-byte. Its gzip SHA-256 is
`ca722f2f8e3e06cea205da19d23d6f277e88fb5e2494c99fa1a88157e98ea944`.
