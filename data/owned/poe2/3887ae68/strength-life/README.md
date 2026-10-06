# Inherent Life from final Strength

This packet publishes one pure Player stat receiver, `inherent-strength-life`, which derives an inherent Life amount (331a) from final Strength (1d2e) and five explicit resolved Boolean inputs (3315–3319). The five controls independently express disabling all inherent bonuses, disabling Strength bonuses, disabling Strength-to-Life, doubling inherent bonuses, and halving Strength-to-Life. Disabled branches explicitly derive zero; missing active inputs remain unknown. Zero Strength still requires the active controls. This packet creates no flag or final-Strength producers and emits no Life contribution or final Life value.

The data follows the accepted [contribution-stage design](../../../../../docs/owned-contribution-stages.md): the derived bonus is an input for a later resource recipe. The existing Class owners and all other Partial closures remain unchanged. It allocates six Actor Stats, one Complete owner for this reviewed pure recipe, and one Player-only receiver. It uses operations V20 independently of the separate ordered-contribution framework.

Source02 passed after 566.00 seconds. It executes the unchanged complete original Player attribute-and-bonus function for all five builds, parsed Original05 double/half/disable controls, a parsed negative Strength control whose final Strength is zero, and an independent original replay. Each of 13 cases runs in a fresh observed host and a fresh hookless host, in both LuaJIT modes. The observation follows exact original actor-function entry, bonus boundaries and return, binds actual emitted record objects, preserves original methods and saved inputs, and records final Strength, resolved flags, relevant ancestor records and full outputs. Its original Strength-filtered Sum supplies the witnessed bonus amount; the test contains no copied oracle calculation.

Disabled cases emit no Strength record; enabled zero Strength emits one record with value zero. Both project to an explicit numeric zero in this derived-bonus contract. The custom modifier controls establish source behavior, not item obtainability or real flag-producer completeness. The first source run is retained as failed observer evidence: its cleanup assertion obscured an unfinished observation frame. The corrected observer binds exact call/return lifetimes, validates repeated line notifications and preserves original error tracebacks. No game calculation was changed or exempted.

Authoring passes. Publication passes in 26.22 seconds and preserves all five imported originals, 110 query rows, existing owners and definitions, with an exact inverse covering only the six new Stats, one owner and one receiver. Five native tests pass in 1.99 seconds using actual published bodies and source final inputs in an explicitly separate finite domain. They cover missing active inputs, the distinction between Strength contributions and final Strength, lazy disabling, missing/Partial receiver coverage, actual Partial Class ownership, and fresh/reused/Rayon determinism. Full real-build completion remains 0/5; neither attribute nor final-Life completion is claimed.

## Checked endpoint and reproduction

The predecessor is `runs/owned-player-intrinsic-life-01/package`, input `35bdaca427fc934f337a908e5060feb72d2be24c62c6936185e869f17202abe7`. The checked output is `runs/owned-strength-life-01/package`, input `f94f845b04f9821373070a5e746303d043e9ae1a8cdd3f95e1d74caa1e4d7856`. Its eighteen files total 60,921,948 bytes and rebuild exactly; all 120 provenance entries are preserved or explicitly appended. Definitions use `pob-3887ae68-strength-life-v1`; operations remain V20. Selected unresolved issues remain 106/117/109/122/5.

Compile each target using `cargo test -p <package> --test owned_strength_life --no-run`, wait for completion, then invoke that command's exact emitted executable. Do not compile while a test process or its child is running. Use fresh output directories and retain failed runs.

| Package / executable filter | Environment |
| --- | --- |
| `poe-optimizer-pob`: `--ignored --exact actual_strength_life_bonus_preserves_original_stage_and_controls` | `POE_OPTIMIZER_TEST_STRENGTH_LIFE_SOURCE_OUT`: fresh source directory; the authenticated report paths are recorded in `source-vectors.json` |
| `poe-optimizer-cli`: `--exact authored_strength_life_is_one_derived_receiver_with_explicit_inputs` | No runtime/source host required |
| `poe-optimizer-cli`: `--ignored --exact publish_strength_life_preserving_all_five_originals` | `POE_OPTIMIZER_TEST_STRENGTH_LIFE_PRIOR`: checked predecessor package; `POE_OPTIMIZER_TEST_STRENGTH_LIFE_OUTPUT`: fresh publication directory |
| `poe-optimizer-cli`: `--ignored native::` | `POE_OPTIMIZER_TEST_STRENGTH_LIFE_RELEASE`: resulting checked package |

The source driver alone requires the complete pinned optional PoB checkout. Publication authenticates those saved reports, original source files and observer hashes. Native evaluation has no Lua runtime. Source02 reports are each 2,233,585 bytes with SHA-256 `d83e6c556900f10011aa8b802191271414ac810aa9cdbc35f106180ec64bc030`.
