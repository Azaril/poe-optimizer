# Authority of saved preset selections when PoB titles trigger a loadout

**Status:** Proposed; requires the project owner's decision. No native guard, selection policy or parity exception is implemented by this document.
**Date:** 2026-10-24.
**Scope:** Import selection and reference comparison. Core and Engine must remain independent of PoB UI behavior.

## Decision

Recommend that explicit saved preset selectors remain authoritative for native import. Native preset titles remain labels; changing a title must not choose different equipment, skills, configuration or passives. Retain the original source and diagnose the narrowly proven PoB title-induced discrepancy when the reference loader selects a different tuple. Do not silently compare numerical results for different selected builds.

The alternative is to reproduce PoB's implicit title-based loadout selection in the Import adapter, then import the resulting tuple. This could improve fidelity to opening a file in PoB, but introduces UI callback/order semantics and allows a rename to change gameplay inputs. It must never move those behaviors into native evaluation.

## Confirmed behavior

At PoB revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, the unchanged Original05 source saves Tree3, Skills4, Items2, Config1 and MAIN group3. Changing only ItemSet2's title gives:

| ItemSet2 title | Actual PoB ItemSet | Weapons | Generated item skill |
| --- | --- | --- | --- |
| Original `Act 1 & 2 (Pre-Ascend)` | 2 | Item28 / empty | Firebolt from Item28 |
| `No shared loadout` | 2 | Item28 / empty | Firebolt from Item28 |
| `Prefix Act 3` | **4** | **Item24 / Item25** | **Skeletal Warrior from Item24** |
| `Act 3` (also the title of ItemSet4) | 2 | Item28 / empty | Firebolt from Item28 |

All saved IDs and every other XML byte are unchanged; the other actual selection axes remain stable. Unchanged Sniper is unaffected. The [original-loader witness](../crates/poe-optimizer-pob/tests/support/outside_config_defaults_source.rs) records six fresh loads per JIT mode, including independent original and substring replays. Both 476,436-byte reports are identical: SHA256 `f4e0e87d18cc15897b92ba3bab43db60f3c46b3e4dc28108c7eae40dfda6d550`. No source selection method, callback or rebuild is replaced or invoked by the observer; provider identities are matched to actual Item/Tree objects.

The cause is specific: `Build:SyncLoadouts` (`Build.lua:637–777`) accepts a substring match on the selected set's title, then `DropDownControl:SetSel` (`DropDownControl.lua:166–174`) invokes the normal callback. `GetLoadoutByName` (`Build.lua:899–954`) selects the first exact title match and `SetActiveLoadout` (`956–980`) changes the presets. This is confirmed source-tool behavior, not a Path of Exile mechanic or nondeterminism.

## Current boundary and risk

`generated_skill_sources::selected` reads the exact saved selectors. Its checked provider joins, quality/usage receipts and archived/live classification therefore describe the saved tuple. They do not establish that PoB's normal loader will evaluate that same tuple after title callbacks. Native `WhenExactSourceSelected` preserves exact target applicability but cannot prove reference-loader correspondence.

The existing source-header ownership passes remain valid: they link source headers to existing presets and keep saved selection Pending. This proposal does not retire that issue, any of Sniper's five selected input obligations, or a numerical coverage gap. A complete imported build must still pass its ordinary semantic checks.

## Options and consequences

| Option | Benefit | Cost |
| --- | --- | --- |
| **Explicit saved IDs; diagnose reference discrepancy — recommended** | Stable, predictable native selection; title edits cannot change a build; no UI emulation in the native model. | Some files intentionally organized through PoB title links may open with a different active tuple. Import/reference reports must expose that difference and preserve its source. |
| Reproduce implicit PoB loadout selection in Import | Matches the selected tuple produced by the reference tool for fully supported source lifecycles. | Must cover callback order, exact/substring/brace matching, duplicates and unsupported cases. Selected equipment can change after a title edit; complexity remains tied to PoB. |
| Keep correspondence unresolved | No new selection interpretation. | A conservative interim refusal can block affected reference correspondence while this decision is pending. It must not be presented as a completed import fix. |

## Implementation after approval

1. State the chosen selection authority in the Import contract. Preserve separate preset axes and exact provider IDs; do not add title dispatch, PoB loadout modes or compatibility paths to Core/Engine.
2. Keep original source and selected-tuple evidence available. A reference report must distinguish saved tuple, actual reference tuple and native selected tuple. Never relabel a receipt for one provider as proof about another.
3. For the recommended option, isolate the demonstrated title-induced discrepancy in the reference-adapter/parity boundary. Compare selected identities before metrics. Classify this exact mismatch; other selector changes, unexplained provider differences, unknown lifecycle cases or numeric mismatches stay failures/unresolved. No blanket build or title-based oracle exclusion.
4. Reuse the existing bounded source and receipt paths. If a private admission guard is needed before a complete reference classification exists, refuse the unsupported correspondence without losing raw source intent. Do not implement a general UI emulator as an interim fix.
5. Retain the real original, substring and exact-title controls, independent JIT replays, full source preservation and exact provider identities. Add native rename-invariance and crossed-provider refusal tests once the policy is implemented. A report or diagnostic must not promote usage, selection, configuration or numerical coverage.

The decision is about which selection the product promises to import. The witness establishes the discrepancy; it does not choose that product contract.
