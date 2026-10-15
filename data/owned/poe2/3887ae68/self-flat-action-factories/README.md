# Exact Basic Attack factory applicability

This offline proof closes the reviewed Action-local factory boundary for Original05's selected Skeletal Sniper Basic Attack. It does not claim that spell-only supplier records are absent, that every minion-store writer is accounted for, or that native contributor membership is complete. No runtime rules, registry IDs, build selections or evaluation requests change.

Seven complete upstream files and nineteen source spans are authenticated against the original PoB source manifest. CalcOffence, ModStore, ModList, ModDB and Data reuse existing immutable archives. This packet adds only complete Global and minion skill-data archives; the upstream [license](../configuration-accounting/evidence/pob/LICENSE.md) remains shared. Compressed inputs are checked before reading, expanded bytes are bounded, and source files/spans must match their reviewed hashes. Full-body review is the authority; the writer-site count is a checksum, not a general Lua parser.

The exact physical Main Hand call at CalcOffence 4137 is reconstructed from the retained `minion-preconversion-source/report.json`. Its original pass binding, actor/profile, summoner/source occurrence, Main Hand selection and cfg must agree with the authenticated selected-domain evidence. This uses the original observed call's cfg; early construction cfg is corroborated separately and is not treated as the final object. Supplemental channel values are not used to infer supplier absence.

That cfg has flags **42,949,813,253**, including `ModFlag.Attack = 1` and excluding `ModFlag.Spell = 2`. The complete ModStore Sum dispatch and both flat-list/keyed-store SumInternal implementations preserve the necessary predicate `band(flags, mod.flags) == mod.flags`, including parent traversal. It runs before tag/value evaluation. The original wide-bit operation and flag definitions are retained; the test helper exercises only the necessary zero/Attack/Spell mask predicate, not a general Lua compatibility evaluator.

| Reviewed source body | Source write sites | Result for this exact Basic Attack |
| --- | ---: | --- |
| Battlemage, CalcOffence 791–800 | 2 | Every added-damage write carries Spell and fails the flag predicate. |
| Spellblade, 801–818 | 4 | Same exclusion for both weapon branches. |
| Spellslinger, 1234–1252 | 6 | Same exclusion for the three weapon branches. |
| Dagger Detonation, 1253–1271 | 6 | Same exclusion for the three weapon branches. |
| Sacrificial Zeal, 2399–2407 | 2 | Same exclusion, independently of cost and branch activation. |

These are twenty source write sites, not twenty observed executions or constructed records. The proof does not require those branches to be inactive. Positive controls show that adding Spell to the cfg would admit the necessary predicate; unqualified and Attack modifiers remain eligible under this predicate and still need their own source/recipient proof.

The parent-weapon factory at 819–834 is separate: it emits Attack-applicable tagged records. Its closure here reuses the original captured Flag calls, nil cfg, exact Sniper source identity and branch outcomes from the runtime-gate proof. Spell filtering provides no authority over that route.

`runSkillFunc` dispatches `initialFunc`, `preSkillTypeFunc` and `preDamageFunc` by reading the selected granted-effect table. The complete Basic Attack declaration is a plain table with no callback; Data's complete loader/postprocessing body does not install a callback metatable on it. The exact loaded seven-key recipe has six null mappings, one arrow flag mapping, no mapped/base callback, and an empty callback inventory. Callback mutations therefore require a new source review. This is source-structural absence for the selected definition, not a claim about every skill or unexecuted callback.

The minion-store dependency remains partial. Flask/Keystone transport, later Player/Party and Actor helper routes, and final native writer membership are separate checkpoints. The final physical consumer must be published with current IDs and checked contributor membership only after those boundaries close. No VM execution, numerical reassociation, aggregate coverage promotion or compatibility code is added here.

## Validation

The dedicated ordinary target is `cargo test -p poe-optimizer-cli --test owned_self_flat_action_factories --locked`. Offline retention uses ignored `retain_selected_action_factories` with `POE_SELF_FLAT_ACTION_FACTORIES_RETAIN=1`. Seven ordinary tests cover the retained proof, positive filter counterexamples, complete source omission/mutation, callbacks/mappings, actual pass cfg/source identity, original parent Flag provenance and the final complete-input seal. Large mutations are serialized.

Offline retention passed in 10.87 s, all seven ordinary tests passed in 14.15 s
(one explicit retention test remains ignored), and strict target Clippy passed
in 1.19 s. Logs are `runs/owned-self-flat-action-factories-retain-01.log`,
`runs/owned-self-flat-action-factories-ordinary-01.log` and
`runs/owned-self-flat-action-factories-clippy-01.log`. No new source process ran.
