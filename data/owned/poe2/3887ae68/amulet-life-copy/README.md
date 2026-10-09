# Numeric Amulet Life copying

This packet implements `amulet-copy-flat-life` through the existing typed rule
graph. It adds no native opcode, schema definition, Lua runtime or alternate
evaluation path. The copy reads the exact modifier's effective Count value and
bypass flag, its equipment use's eligibility, and the frozen Player pre-copy
percentage. Its output is an ordinary Life/Add contribution to the exact Player.

For factor `percentage / 100`, identity or bypass preserves the effective value.
Otherwise integral inputs use `trunc(floor((value * factor) * 100 + 0.5) / 100)`;
fractional inputs use `floor((value * factor) * 10) / 10`. These injected
arithmetic branches match the authenticated original numeric Life path. They do
not reuse the different nested gem-level floor rule. The existing Count-to-Life
unit conversion remains explicit.

Each modifier produces its own copy, separately from its direct contribution.
Two records of 17 and 19 at 25% produce copies of 4 and 4, not a combined 9.
An eligible zero copy remains a known zero; an ineligible copy remains inactive.
Identity and bypass do not demand unused scaling arithmetic.

Four already-known non-Amulet templates receive false eligibility through
ordinary data programs. Existing Amulet eligibility is reused. The copy's
potential writer is added to `life-base-contributions` with exact source
membership across all twenty equipment slots. The equipment group remains
Partial: its provisional ranks do not prove game accumulation order.

Only `amulet-bonus-copy-unconverted` is retired from the flat-Life owner. Its
five remaining gaps cover source encoding/corrupted ranges, ordered magnitude
transforms, canonical input admission, ordinary routing and external contributor
membership. The owner remains Partial; this packet supplies no final Life
formula or complete-build result.

## Evidence and validation

The [source witness](../../../../../docs/owned-amulet-life-copy-evidence.md)
captures four complete-build parsed controls and nine original-method scalar
probes. The near-integer factor is constructed as `99.99 / 100` in both paths;
native controls verify identical factor bits before comparing results. Negative,
fractional and bypass probes test source arithmetic, not obtainable item rolls.
Raw source ring-record ordering remains diagnostic rather than native ordering
authority. Numeric and Amulet-sequence comparisons remain exact.

Ordinary Rust tests execute the actual injected copy program in the finite joined
Sniper replay. They cover units/recipient, direct/copy separation, duplicate
identity, zero/identity/fractional/negative branches, missing effective producers,
inactive sources, overflow and lazy bypass, source removal, storage permutations,
membership/stage refusal, real Partial-query refusal and fresh/reused/four-worker
execution. Only the finite controlled query domain is closed in those fixtures;
production data preserves its gaps.

Publication authenticates the complete retained reports, observer/driver/witness
bytes, pinned source files, predecessor owners and query. It checks all potential
Life/Add declarations before and after the change. An exact inverse permits only
the five added programs, one retired gap, query member and provenance to differ.
The ordinary release assembler reimports all five originals and verifies
byte-identical rebuilding; source evidence files are not runtime inputs.

```powershell
cargo test --locked -p poe-optimizer-cli --test owned_amulet_life_copy
$env:POE_OPTIMIZER_TEST_AMULET_LIFE_COPY_PRIOR='runs/owned-reward-support-publication-01/package'
$env:POE_OPTIMIZER_TEST_AMULET_LIFE_COPY_OUTPUT='runs/owned-amulet-life-copy-publication-03'
cargo test --locked -p poe-optimizer-cli --test owned_amulet_life_copy -- --include-ignored
```

Use a fresh output directory when reproducing. The current validation results and
canonical package identities are recorded in the
[implementation plan](../../../../../docs/implementation.md). Selected imported
input obligations remain 107/117/109/123/4, with 110 queries, no evaluation bundle
and **0/5 complete native builds**.
