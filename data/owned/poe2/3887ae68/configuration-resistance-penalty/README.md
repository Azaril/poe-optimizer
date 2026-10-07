# Configured Player resistance penalty

This packet consumes the saved or constructor-default `resistancePenalty` as
typed Player external input `334d`, measured in percentage points (`0002`). The
existing shared Player Actor `332a` contributes that value once to Fire `32e6`,
Cold `09d4`, and Lightning `32e7`. It does not add it to all-elemental `09d5` or
Chaos. Existing actor programs and their Partial closure remain intact; Encounter
`31d1` gains the explicit input member without completing membership.

The input constructor default is -60. A valid numeric Input overrides it,
including zero, fractions, and values outside the seven UI list options.
`ConfigTab.Load` writes the numeric value directly; the dropdown's `SelByValue`
updates only presentation selection and never changes an unmatched input.
Numeric Placeholder values are a separate lane and never supply this direct
calculation input. This packet opts into `ignore_numeric_placeholder`: an admitted
numeric Placeholder is checked as finite numeric source text, then ignored,
matching the source witness. It cannot override a numeric Input or replace the
constructor default, and its unused magnitude is not constrained by the owned
output range. Malformed or wrong-lane Placeholders still block projection.
String Placeholder is an unusual input-writing source lane and remains refused.
Missing input receives the
authenticated constructor value in Import, not an evaluator fallback. Malformed,
ambiguous or otherwise unproved source fields must remain unresolved.

The admitted interval of -1,000,000 to +1,000,000 is a finite adapter bound, not a
game clamp. The source's direct penalty calculation has no list-membership clamp.
`source-vectors.json` pins the source manifest, bounded source-code spans and the
retained configuration witness's five default observations. Those historical
reports have no explicit penalty mutation cases; their final Player resistance
observations remain diagnostics until complete contributors and caps are owned.
The separate fresh witness pins seven exact controls: absent Input, zero, -30,
-12.5, +10, numeric Placeholder alone, and zero Input with numeric Placeholder.
Two independent fresh loads per control and JIT mode each observe cold state and
exactly two normal rebuilds. All three Player BASE channels and the distinct
zero Chaos record agree in MAIN/CALCS and across those fixed observations; the
two report files are byte-identical. The packet retains the complete compact
observations, report hashes, observer/test-code hashes, original fixture hash
and exact mutated XML hashes. This evidence authorizes the packet's narrow
numeric-Placeholder opt-in; it grants no authority to ignore other source fields.

`closure.json` contains the refined encounter, new external definition and full
updated Actor owner. `dependencies.json` preserves the exact prior definitions,
Actor owner, six permanent-reward owners and existing-actor binding. `input.json`
is the typed Import default-input row; `bindings.json` names its native channels.
The publication starts from input
`b559c6c55aedfe5cf6e947ca210103c14444647b42f5f0483a32d5b7fa8c3d88`.

The ordinary native test target `owned_configuration_resistance_penalty_native`
checks authored/source pins, all seven fresh source controls and their exact
three-channel observations, signed/fractional values, missing and wrong-target
input, actual Partial refusal, and fresh/reused/Rayon determinism. Its explicit
finite domain runs the real penalty and six real reward programs together,
checking their contributions to the same three Player channels. Finite test
closure is not published. The ignored endpoint test authenticates retained
reports and exact source spans and checks the published component through
`POE_OPTIMIZER_TEST_CONFIGURATION_PENALTY_RELEASE`.

This supplies one real configuration consumer. Complete assumptions,
configuration/scenario-usage dispositions, complete incoming resistance
contributors, final reductions/caps and full-build parity remain unresolved.

## Validation and reproduction

Checked endpoint: `runs/owned-resistance-penalty-02/package`, input
`b19134496c85e208a735568b5e5191f3b60f68154be336ab4afb6fd953b6fa01`.
The 18-file publication has 135 provenance rows and rebuilds byte-for-byte.
All five original imports retain complete draft/sidecar inverses after removing
only the new assumption and authenticated derived commitments. All 110 queries,
selections and unresolved issue counts (107/117/109/123/5) remain unchanged.
Original05 gains its seventeenth assumption; no evaluation bundle is published.

The publication test passes in 37.74s, with eleven additional controls for saved
values, malformed/wrong-lane/duplicate fields and numeric Placeholder precedence.
The endpoint/source authentication passes in 2.33s. Six ordinary native tests,
the ordinary packet test and all 307 Import normalization tests pass. Strict
workspace/all-feature/all-target Clippy passes. These are focused checks, not a
new full-workspace or full-build parity result.

To reproduce the ignored publication target, set
`POE_OPTIMIZER_TEST_PENALTY_PRIOR` to the checked predecessor package and
`POE_OPTIMIZER_TEST_PENALTY_OUTPUT` to a fresh directory, then run
`owned_resistance_penalty_cli::publish_penalty_input_and_native_consumer_preserving_originals`.
For the ignored native endpoint/source check, set
`POE_OPTIMIZER_TEST_CONFIGURATION_PENALTY_RELEASE` to the new package and run
`owned_configuration_resistance_penalty_native::published_penalty_uses_exact_authored_component_and_authenticated_source`.
The separate PoB target `owned_resistance_penalty_source` runs
`fresh_penalty_defaults_and_authored_values_reach_player_channels` with the
optional `pob` feature and source witness environment specified in its test.

Local logs: `runs/owned-resistance-penalty-publication-02.log`,
`runs/owned-resistance-penalty-source-authentication-02.log`,
`runs/owned-resistance-penalty-source-01.log`,
`runs/owned-configuration-defaults-tests-03.log` and
`runs/owned-resistance-penalty-clippy-03.log`. Fresh source reports live under
`runs/owned-resistance-penalty-source-01/`; both are 15,325 bytes with SHA-256
`c76eafe90420d1a722f2c7e31663d6f18b0c11848ebd24e51abc80ada5fa4b99`.
The tracked source-vector manifest authenticates these reports and their observer.
