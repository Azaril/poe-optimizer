# Preset reporting inclusion intent

This packet preserves a saved skill group's `includeInFullDPS` preference as a
typed Boolean on each exactly eligible skill occurrence. It uses the existing
skill-preset preferences, source applicability, scenario override and native
usage-policy contracts. It does not implement an aggregate report.

The two new definitions are UsagePolicy `338e` and its required Boolean parameter
`338f`. The ordinary-gameplay owner has an explicit Complete-empty program set
and an `AuthoredAssignmentsOnly` support-source declaration. Consequently the
preference participates in request identity and ordinary target/input validation
without creating an effect, invocation, numeric producer or query. Completing
that empty gameplay program set does not complete preset usage or reporting
execution.

The existing seven physical-skill and six occurrence-usage rules each receive
one exact appended policy. Their eligibility, provider/source identity, field
census, prior policies and ordering are otherwise unchanged. A containing group
projects only to the occurrences admitted by those existing rules. Unknown or
archived sources do not acquire invented targets.

The injected exact grammar retains `true` as true and `false` or literal `nil`
as false. An absent, malformed or unrecognized value remains Pending. Literal
`nil` is a saved string here; it is distinct from an absent XML attribute. The
loader source establishes that only the exact string `true` enables inclusion.
The narrow codec does not use that fact to silently accept arbitrary spellings.

## Source authority

`SkillsTab.lua.gz` and `Calcs.lua.gz` retain exact upstream Git blobs and are
authenticated against the pinned source manifest. The source spans cover the
saved inclusion loader and its separate FullDPS consumer. The source remains
offline evidence; neither these bytes nor a Lua value type enters native rules.

`source-attribution.json.gz` retains an existing 44-case observer report with
fresh and two rebuilt states. The JIT-off and JIT-on acquisition files have the
same authenticated raw hash; both acquisition identities are retained without
duplicating the bytes. The source verifier authenticates the entire proof,
archive inventories, source manifest, body locations and observer controls.
Compressed reads and expanded bytes have explicit hard bounds. No new source
execution was needed for this packet.

The retained report supplies source semantics and reporting attribution evidence.
It supplies no native aggregate arithmetic, provider inventory closure or
whole-build parity authority. Upstream source licensing is retained in the
repository's Path of Building submodule and existing source-manifest provenance.

## Validation and remaining responsibility

Rust import tests exercise the actual Boolean grammar, group fanout, separate
presets, disabled sources and absent/malformed values. Native tests install the
actual schema and owner with namespace-only fixture rebinding, check exact
support-domain equivalence and missing/unmapped-domain refusal, and verify that
true/false request identities differ while ordinary values, effects, activation
and query selections do not. They also exercise required inputs, exact authored
and generated targets, disabled providers, whole-record scenario overrides,
reused scratch and Rayon evaluation.

Inserting a preference can change the canonical position of an existing usage
record. Its diagnostic invocation index follows that position. The native test
authenticates that exact positional change separately and requires unchanged
invocation ownership, program, recipient, result and ordinary value inventory.

The all-five release gate requires an exact inverse after removing only the new
proved preferences, retaining prior local IDs, values, source correspondence,
queries and selected unresolved issues. Publication and final validation results
are retained in `runs/owned-reporting-inclusion-publication-01/validation.json`.

Completed Rust checks:

- `owned_usage_policy`: 15 tests passed, including four new reporting controls
  (`runs/owned-reporting-inclusion-native-04.log`, 0.01 s).
- `owned_normalize`: 349 tests passed, including three new reporting controls
  (`runs/owned-reporting-inclusion-import-02.log`, 1.06 s).
- Strict targeted Clippy passed for both targets (the corresponding
  `native-clippy-01` and `import-clippy-01` logs).
- The CLI source/inverse target passed all three ordinary tests in 0.28 s
  (`runs/owned-reporting-inclusion-cli-02.log`); strict targeted Clippy passed
  (`runs/owned-reporting-inclusion-cli-clippy-01.log`, 0.65 s).
- The all-five publication passed in 45.32 s
  (`runs/owned-reporting-inclusion-publication-01.log`). Its replay produced
  byte-identical package artifacts. New preferences by original are
  **8 / 0 / 0 / 1 / 40**, across the existing reviewed source rules and all saved
  presets. Every previous preference, local ID, value, origin and selected issue
  survived the exact inverse. Selected issues remain **106 / 117 / 109 / 123 / 4**;
  no issue or owner-coverage gap was retired, and zero complete builds are claimed.

The new endpoint is
`runs/owned-reporting-inclusion-publication-01/package`, with 18 files,
64,432,702 bytes and 187 provenance entries. All 110 query rows are unchanged.
The 8,186 owner programs and one effect application remain unchanged; the new
empty owner consumes no additional program budget.

- Input: `e1c2abd299aead48ff1da38aa517c4fcbcaacd2671f754deb921364c0cad4b39`
- Definitions: `86764ee48af08a18ca08ad7eac3e2443f38b66ff948021a3b6ef673ff1d24211`
- Rules: `7d7ecf94119f30cb6209f0c3a91b3e167f5088efb96285944d88350b4b3049ce`
- Compiled rules: `6401dc0a0122ae43414c52051c70acc83361c4edaab7b9c2a775f93523fa21a1`

Aggregate execution, output counting, tie/attribution semantics and any future
aggregate metric or request target remain separate work. The selected TotalDPS
requests, existing activation/count/population policies and all current
whole-build coverage gates remain unchanged.
