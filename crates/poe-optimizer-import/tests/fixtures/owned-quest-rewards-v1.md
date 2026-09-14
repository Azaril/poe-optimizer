# Finite quest input metadata

`owned-quest-rewards-v1.json` is a reviewed test fixture exported from schema-40 `configuration.definitions` metadata. All 17 source-generated reward controls are retained: nine boolean controls and eight option lists containing 30 exact tokens, including their eight `None` alternatives. Exact keys, defaults, decoded source strings, source records and source-location/generator/default provenance remain in the artifact.

The exporter verifies the injected data package's exact raw hash and every configuration source file's LF-normalized hash against the vendor checkout. The existing optional `extract-game-data` CLI produces the full catalog when explicitly invoked; this fixture export only reads the already committed catalog and source files. No Lua execution is needed.

```text
python scripts/export-owned-reward-fixture.py --data crates/poe-optimizer-data/data/game-data.json --data-sha256 a90217d9bab6c0469917a2ba75ed9517205d2ac2b3d59f8d68580604f26df07f --source-root vendor/path-of-building-poe2 --output crates/poe-optimizer-import/tests/fixtures/owned-quest-rewards-v1.json --check
```

Omit `--check` only for a new destination. The exporter refuses overwrite. The fixture is 26,768 bytes; raw SHA-256 is `3afadc1baa928c91368b711cde665a3dc6737c68cb15f51be1f7e86ed86e66fe`.

`tests/support/owned_reward_fixture.rs` stages fresh owned Option and Reward definition IDs through the registry, mapping each reviewed configuration outcome and reviewed default alias. It injects all keys/tokens/defaults into ordinary value/reward policy DTOs; production does not match these names or source positions. Merge `source_pin()` before compiling other artifacts bound to the combined source pin, then call `stage_rewards` after their fresh identity allocations and `policy_input` after final schema/mapping assembly.

The narrowly reviewed input-schema fact is that each fixed selected outcome has no further authored parameter, choice, socket or provider-output input ports. `Known` empty declarations describe that input boundary only. `effects_compiled: false` is intentional: neither this fixture nor its helper parses reward stat text, generates game effects, infers activation or certifies numerical coverage. False/`None` outcomes select no reward. Unrelated or incompletely mapped reward membership remains pending in normalization.
