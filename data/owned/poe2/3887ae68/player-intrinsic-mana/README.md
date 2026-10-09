# Shared Player intrinsic Mana

One injected Actor program contributes `4 × character level + 30` Mana points
through the existing Mana channel `29f9`, using existing unit `0003`. It belongs
to shared Actor `332a`, whose existing Player binding invokes it once independently
of class. No new definition, receiver, reduction, engine operation or compatibility
behavior is introduced. The shared Actor's Partial coverage remains unchanged.

`migration.json` is a checked append to the current V25 release. All prior
programs, input policies, query membership and five original imports survive.
`dependencies.json` identifies the existing descriptors and binding; the class
descriptors are also used by explicit finite native test domains. `authoring.json`
and `source-vectors.json` are offline provenance, not evaluator inputs.

The source observer loads all five unchanged builds through pinned PoB, locates
the actual Player Mana BASE record with source Base, and calls the original
`EvalMod`, `Sum`, `GetMultiplier` and `Override` methods. It verifies the relevant
modifier read set and scalar output are unchanged. It installs no method wrapper
or debug hook and contains no copy of the arithmetic. PoB's constructor is in
`CalcSetup.lua:837`; its per-level constant is in `Data/Misc.lua:157`.

Ten cases cover the five originals, level 1/91/100 controls, a fresh repeat and
a warm level-1-to-original transition. A second independent warm replay also
matches. JIT-on/off reports are byte-identical: 13 complete loads per mode,
62,034 bytes per report, SHA-256
`441c1e547e5db98a292d32f5f1a6bf5def004c10e34657588f708786ff5ac839`.
The five observed intrinsic contributions are 414, 382, 402, 414 and 398.

The native tests consume those retained observations and execute the published
program on all eight classes at boundary/interior levels. They verify exact
Player ownership, one contribution, level rejection, Partial-owner refusal and
fresh/reused/four-worker equality. Their finite domain excludes all other
mechanics; it is never published as game coverage. The existing support-free
fixture exercises the already sufficient V22 operation subset; the actual
publication remains V25 and passes normal compilation.

Run ordinary native checks with:

```text
cargo test --locked -p poe-optimizer-cli --test owned_player_intrinsic_mana
```

Source reacquisition uses the ignored `poe-optimizer-pob` test of the same name
and a fresh `POE_OPTIMIZER_TEST_PLAYER_INTRINSIC_MANA_SOURCE_OUT` directory.
Publication uses `publish_mana_preserving_all_five_originals` with
`POE_OPTIMIZER_TEST_PLAYER_INTRINSIC_MANA_PRIOR` and a fresh
`POE_OPTIMIZER_TEST_PLAYER_INTRINSIC_MANA_OUTPUT` directory. Publication also
authenticates the retained source reports and their observer/source hashes.

This is an intrinsic contribution, not a final Mana pool or complete contributor
census. Native character level comes from validated build data; the rule does
not emulate PoB's general `Multiplier:Level` lookup or permissive level clamping.
All observed level modifiers/overrides were absent, and unsupported producers
remain subject to normal coverage checks.

The Original05 source read set suggests the next Mana dependency: Intelligence
contributes 210, followed by the already represented 5% quest increase. Its
reported final Mana is 638. This observation guides follow-up; it is not authority
to close other contributor domains or bypass final rounding validation.
