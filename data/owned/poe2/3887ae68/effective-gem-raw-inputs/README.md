# Raw corruption delta normalization

This complete policy binds the unchanged effective-input schema and replaces
only the Twister `000a` and Sniper `0011` Gem input rows. It uses the existing
Cleric `0916/3072` Count codec for the new `30a9` and `30aa` slots. The codec reads
the exact `corruptLevel` attribute, rejects duplicates, accepts scientific finite
quantities without trimming, applies scale 1/1, and explicitly maps `nil` to zero.
Missing, malformed and non-finite values remain Pending.

The `corruptLevel == 0` guard is removed. The `corrupted` guard still requires
literal `false` or `nil`; true corruption remains unconverted until those Gems
have explicitly declared Boolean inputs. Parameter membership remains Partial.
These are raw supplied values, not effective levels or final skill parameters.

`authoring.json` binds the exact prior release input
`bb501a9181af6cf4c93828fb753a37cb107f3d61c26c0f82a5bf20ffac4245c0`.
`publish-owned-normalization` checks the unchanged schema v4 / operations v13
release and produces an intermediate compact transition. That format does not
carry release ancestry. The real test therefore feeds its checked policy and
tree bindings into a full endpoint cloned from the predecessor, retains every
prior provenance entry, adds an explicit authoring record, and publishes through
the existing `assemble-owned-release` command. No migration or loader guard is
changed. The compact directory is not the final baseline.

```powershell
cargo test --test owned_effective_gem_raw_cli
$env:POE_OPTIMIZER_TEST_EFFECTIVE_GEM_RAW_PRIOR = 'runs/owned-effective-gem-release-01/package'
$env:POE_OPTIMIZER_TEST_EFFECTIVE_GEM_RAW_OUTPUT = 'runs/owned-effective-gem-raw-inputs-02'
cargo test --test owned_effective_gem_raw_cli real_raw_input_successor_preserves_release_and_normalizes_originals_and_probes -- --ignored --exact --nocapture
```

The explicit output directory must be new. The real test verifies the exact
policy change allowlist, every unchanged artifact, five query files/110 queries,
full provenance preservation, byte-identical release rebuild, and fresh
normalization of all five original builds. It checks selected raw values through
their exact source origins, including all saved Twister/Sniper occurrences.
Sixteen CLI probes cover positive, negative, fractional, explicit nil, missing,
malformed, non-finite and true-corrupted inputs for both owners. Separate default
codec tests cover exact whitespace, scientific notation and the finite maximum.
All five original builds remain Pending; full numerical coverage remains 0/5.

Validated on 2026-10-01 UTC: two default tests passed, with the real test explicitly
ignored. Its separate explicit run passed, including all five originals and 16
CLI probes. The original selected raw-delta counts were `[0,1]`, `[6,0]`, `[0,0]`,
`[0,0]`, `[0,5]` for `[Twister,Sniper]`, covering every saved matching row.

The final baseline is `runs/owned-effective-gem-raw-inputs-02/package`; `rebuilt`
is byte-identical. Its 18 files total 58,389,504 bytes. The release input is
`90208367c88c36cf2e20fad88573430bc7bfc13c56f9002c2570ab9eaf133ced` and normalization
identity is `5442bf70ba683484170dea9c4a544f675f5712931e05441858426ea70a5ce83e`.
Schema, registry, rules and all 110 queries are unchanged. All three predecessor
provenance entries remain, followed by one explicit normalization authoring entry.

Receipts are `validation.json` and `execution-receipt.json` in that run directory.
Logs are `runs/effective-gem-raw-cli-tests.log` and
`runs/effective-gem-raw-real-publication-tests.log`. The separate
`runs/owned-effective-gem-raw-inputs-01/compact` directory records the initial
publisher compatibility check and is not the final baseline.
