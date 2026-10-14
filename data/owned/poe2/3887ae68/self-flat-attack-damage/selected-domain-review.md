# Selected source recipe audit

This packet records the original source recipes available to the unchanged supplied build with selected tree 3, equipment 2, skills 4 and configuration 1. It is test-only offline evidence. The selected XML and every original Lua source file are authenticated; the artifact fingerprint rejects changes until separately reviewed. It grants no native coverage, retires no saved-input obligation and does not authorize an empty flat-damage producer.

## What is reconstructed

The selected Item XML is parsed by the original `runtime/lua/xml.lua`, then executed through the complete original Item branch of `ItemsTab:Load`. This includes attributes, alternate variants, ordered raw-text and `ModRange` children, fallback socket handling and the original `BuildModList`. The original branch bytes and whole file are hashed. Raw XML, attributes, mixed-content order and original parsed XML are retained. This avoids silently replacing saved range metadata with the raw item's defaults. All eight selected Item records have saved ranges. For example, the loaded Solar Amulet supplies 13 Spirit and the Ashen Staff grants level 11 Firebolt under their saved range of 0.5.

Each equipped use retains its slot identity; repeated uses of one inventory record remain separate. The original `GetActiveModListForSlotNum` is called with bonded access both disabled and enabled, and both record lists are retained. Equality, when observed, applies only to these selected item records. It does not infer whether the Player can use bonded modifiers in another build.

The original modifier parser reconstructs selected saved passive lines and quest-reward text. Other configuration values and custom blocks remain retained inputs, not executed configuration callbacks. The saved specification remains explicit; parsing its node recipes does not assert that every saved node is active in an original calculation pass.

The skill inventory includes saved gems, all Player active-effect identities observed in the retained source pass, and every observed supplied minion's child skill. The profile inventory includes every observed supplied Actor profile. The result contains 40 distinct skill definitions and six profiles; eight observed Actor occurrences retain their independent source identities. For each skill the packet retains resolved stat mappings, top-level base modifiers, every stat set's base modifiers, and callback source addresses. Profile modifier lists and callbacks are retained. This matters because `CalcActiveSkill.mergeSkillInstanceMods` adds `set.baseMods` directly and supplied minions can themselves provide buffs or auras.

## What remains unproved

The retained source pass observes sixteen Player active effects and contains no Rallying Cry effect. It also records the selected Sniper's actual flat-damage query inputs and thirty Player minion-modifier records. These facts corroborate the exact saved selection; zero observed physical totals do not prove a complete native supplier domain.

No callback execution, configuration application or supplied-child buff application is reconstructed by this packet. The prior pass did not directly observe every dynamic parent-weapon, Hollow Palm, `ExtraAura` or `ExtraAuraEffect` gate. Missing observations remain unknown. In particular, absence of a Player Aura consumer does not exclude supplied-minion Aura consumers or the separate unconditional `ExtraAura` consumer.

To investigate those remaining gates, extend the existing original source harness for one unchanged case. Capture exact Actor/Action identities and raw candidates alongside original query results at the parent-weapon and Hollow Palm branch sites; capture Player `ExtraAura` and each executed Player/minion `ExtraAuraEffect` query; retain supplied-child buffs, participation and source-to-recipient applications. No live original VM survives in the saved JSON report, so these cannot be obtained by querying the old report. New observations need an independently repeatable original run, not guessed stores reconstructed from a zero output.

## Validation contract

Rust tests reconstruct the entire retained packet from pinned original source. Mutation controls change actual source skill base modifiers, actual stat-set base modifiers, disabled item recipe text and saved Item XML ranges; the changed records must remain visible and cannot inherit the original review fingerprint. Scope, Party inputs, source membership and module-order mutations are rejected. These checks establish the fidelity of this bounded recipe audit, not complete native build evaluation.
