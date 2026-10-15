# Configured target distance inputs

This packet adds two Enemy Scenario assumptions through the existing Import
projection policy. It publishes no native rule, Stat, contribution or coverage
closure. Its predecessor is the Life equipment-domain release
`fe061a37c795102c8ca025cf52ebca199cb0ba3be42c9fa901e6c42ac5bc6f3f`.

| Definition | Owned meaning | Saved value selection |
| --- | --- | --- |
| `def.0000000000003380` | Configured target distance for skill range | Input, including zero; otherwise effective Placeholder |
| `def.0000000000003381` | Configured target distance for distance effects | Nonzero Input; otherwise effective Placeholder |

Both use existing distance Unit `def.0000000000002425` without conversion:
ten source units represent one metre. The injected admission range is
−1,000,000 through 1,000,000. This is a bounded import domain, not a claim about
valid gameplay distance or a source UI clamp. The constructor is 20 only after
checked fresh-frame absence; a malformed or out-of-range saved lane prevents
both outputs. Every Input and Placeholder leaf requires receipts for both
projections and receives one exact Scenario link.

For Input 0 / Placeholder 80, the values are 0 and 80. A second value of zero is
a known candidate; it is not an emitted zero modifier. Recipient selection,
downstream overrides, multiplier composition and applicability remain separate
native responsibilities. Source `Effective` is a calculation-view policy and
is not imported as a gameplay flag.

## Source authority

`source-evidence.json` authenticates complete pinned original source archives,
18 exact body spans, and existing configuration constructor dependencies. The
review includes the direct skill-range path, copied ailment configuration,
generic distance ramps, the callback, presence/defence/accuracy consumers,
generic multiplier thresholds and parent/override composition. The Overreach
source definition illustrates threshold use; it is not a restriction to that
support or evidence that it participates in every build.

The compressed `distance-witness.json.gz` retains the complete existing
`owned_minion_accuracy_source` report, unchanged. JIT-off and JIT-on reports
were byte-identical, SHA-256
`a61e14e329084002bdc1b1ef9034f33e62574e0982619a9a6a5dcb4554786890`.
The ordinary Rust proof authenticates that identity against the earlier
minion-accuracy authoring record, source manifest, original XML hashes and
archived acquisition observer. The 12 distance controls yield 14 explicitly
reviewed MAIN/CALCS rows. They distinguish raw stored Config records from
view-dependent consumed values: Combat may consume zero while the configured
candidate is 80. A saved Placeholder zero produces no stored distance record.

No source VM was run for this packet. The broader witness contains 31 cases
and 32 loads per JIT mode, with no plain uninstrumented control or independent
repetition of each distance variant. These are bounded reference observations,
not native downstream parity or a source-completeness certificate. The full
raw witness is retained rather than replacing it with selected observations.
Upstream source remains covered by the referenced PoB license in the existing
configuration-accounting evidence directory.

## Verification and publication

`owned_configuration_distance_inputs` validates data and source semantics with
mutations of body/source identities, missing/duplicate controls, raw versus
consumed values, tags and zero-record behavior. The shared singleton-source
checks remain intact.

The explicit `owned_configuration_distance_publication` gate stages the data,
reimports all five originals, inverses the complete appended input vector and
one reviewed leaf delta, preserves exact local IDs and all 110 queries, and
requires a byte-identical rebuilt package. Its controls include mixed-zero
values, absence, constructor fallback, negative/fractional values, boundaries,
malformed losing lanes and rejected encounter contexts. Global inventories and
selected completeness remain unchanged. Publication results are recorded in
the run's `validation.json`; this source packet alone does not claim release
success.

Validated publication: `runs/owned-configuration-distance-publication-02/package`,
input `0efaf9bebc6a3f40e448cb22596c539b53436e15c063f92209c87627a6b4fa62`.
The all-five gate passed with 16 input controls and a byte-identical rebuilt
package. Each original gained two assumptions; only its reviewed distance leaf
lost the configuration fallback link. Original05's linked origins changed from
32 to 31 (17 to 16 inside Config, 15 outside unchanged). Selected issue counts
remain `[106, 117, 109, 123, 4]`; no containing inventory was completed.

Three new source tests and all three existing scalar-source tests passed.
Strict Clippy passed the distance and default input/publication targets. Logs:
`runs/owned-configuration-distance-tests-01.log`,
`runs/owned-configuration-distance-publication-02.log`, and
`runs/owned-configuration-distance-clippy-02.log`. The earlier publication01
log records a test-helper assertion on an omitted false field, corrected before
the successful run; no source reacquisition or data repair was involved.
