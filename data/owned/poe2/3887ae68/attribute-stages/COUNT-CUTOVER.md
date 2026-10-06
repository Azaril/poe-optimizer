# Per-occurrence Count contribution cutover

This authored step preserves final Integer attributes `1d2e/1d2f/1d30` and introduces six Actor Count input channels using existing unit `295a`: `331b/331c/331d` for the first Strength/Dexterity/Intelligence pass, and `331e/331f/3320` for the second. It supplies no final attribute formula, query ordering, MORE grouping, comparison conditions or contributor closure.

The pinned predecessor is `runs/owned-strength-life-01/package` (input `f94f845b04f9821373070a5e746303d043e9ae1a8cdd3f95e1d74caa1e4d7856`). Exact before/after bodies cover 368 programs: eight class bases, 293 attribute choices, 63 passive views and four item attribute projections. Each of the original 992 Add and 17 Increase effects fans out to the two pass inputs while retaining its provider, condition and numeric contribution kind. Unrelated effects and all owner closures are unchanged. There is no aggregate Integer-to-Count lift.

Class/passive integer Add literals become exactly representable Count literals. Item projections retain their existing per-occurrence `QuantizeInteger` and then use `ScaleInteger(1 Count, integer)`. Their formatter already produces integral Counts, but the quantization also enforces the browser-safe Integer bound; retaining it preserves that failure behavior. All Attributes still converts once per modifier occurrence before emitting to the three attributes in each pass. Repeated equipped uses remain independent occurrences.

`count-programs.json` stores only the affected programs and their exact owner closures, including the predecessor bodies needed for inverse checks. The shared test helper uses the existing V5 six-definition migration as an internal authoring step, replaces only those exact bodies, and assembles a checked V20 release. It verifies the inverse against that exact schema migration, the prior rules' serialized bytes, and all inherited Import dependency rebindings. Frozen migration and converter APIs are unchanged. A later V21 recipe can use these streams without retaining a second live Integer contribution path.

The authored check passes. Publication passes in 29.89 seconds and preserves all five imported drafts, exact selections and 110 queries. The eighteen output files rebuild byte-identically. Five native tests pass in 3.16 seconds: actual class/passive occurrences across all five selections, exact node15782 choice changes and inactive loadouts, preserved Partial class refusal, fresh/reused/four-worker Rayon equality, and the four item projection programs' missing-input and overflow boundaries. Class/passive tests operate in an explicitly finite unpublished domain; item tests inspect exact projection programs, not full equipped-item routing.

The first build exposed fixture type/import errors. Native01 then exposed a missing referenced Unit in the finite schema. Native02 retains exact units from published Quantity Stat descriptors. Those fixes change no production validation, authored body, coverage rule or arithmetic. Failed runs remain available beside the passing evidence.

The published endpoint is `runs/owned-attribute-count-01/package`, input `fffc60b6e9e7754b7cf394fcead94bb9c8db4f2dfc9c788acf68e5798e621275`. Definitions use `pob-3887ae68-attribute-count-inputs-v1`; the registry ends at `3320`. Eighteen files total 61,304,010 bytes and carry 121 provenance entries. Selected unresolved counts remain 106/117/109/122/5, with no evaluation bundle and 0/5 complete native builds. Source correspondence, all incoming contributor inventories, item routing and final staged attribute calculations remain separate obligations.

## Reproduction

Compile each target with `cargo test -p poe-optimizer-cli --test <target> --no-run`, wait for completion, then invoke its exact emitted executable. Never compile while a test or child is live. Use fresh output directories; preserve failed evidence.

| Target / filter | Environment |
| --- | --- |
| `owned_attribute_count_cutover_authored`: `--exact authored_attribute_count_cutover_preserves_each_existing_occurrence` | No local published artifact needed |
| `owned_attribute_count_cutover`: `--ignored --exact publish_count_contributions_preserving_all_five_originals` | `POE_OPTIMIZER_TEST_ATTRIBUTE_COUNT_PRIOR`: checked Strength-Life predecessor; `POE_OPTIMIZER_TEST_ATTRIBUTE_COUNT_OUTPUT`: fresh output directory |
| `owned_attribute_count_cutover`: `--ignored native::` | `POE_OPTIMIZER_TEST_ATTRIBUTE_COUNT_RELEASE`: resulting checked package |

The optional original-function pipeline witness is independent of this exact conversion proof. No Lua runtime, source output graph or observer code enters the native package.
