# Ownership of imported calculation-view settings

**Status:** Proposed; requires the project owner's decision before Calcs responsibilities can be retired. No implementation is authorized by this document.
**Date:** 2026-10-24.
**Scope:** Preserve PoB's saved calculation-view selection without inventing native measurements or importing PoB execution modes into Core.

## Decision needed

Should imported Calcs settings remain **source-owned reference settings**, with native queries and scenarios chosen explicitly, or should import automatically construct native query/scenario presets from that saved view when correspondence is available?

Recommend source-owned reference settings. This uses the existing import provenance boundary and keeps the original meaning recoverable. It is a deliberate import contract, not a claim that the settings are cosmetic or have no effect.

## Evidence and current gap

Original05 saves `skill_number=1` and `misc_buffMode=EFFECTIVE`. Its MAIN view selects group3; its CALCS view selects group1. These settings choose a real reference action and calculation assumptions. Removing the two saved inputs yields the same fresh constructor defaults, so omission equivalence does not make their meaning disappear.

At pinned PoB revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`:

- `CalcsTab.lua:22–33,199–233` initializes and loads the inputs. `CalcSetup.lua:804–826,1889–1910` consumes their buff/combat/effective mode and selected group.
- Normal `Build.lua:34–51,526–605` creates fresh objects. `CompareEntry.lua:233` instead synchronizes CALCS selection to MAIN; that separate loader is outside the proven frame.
- Config migration at `Build.lua:596–600` and `ConfigTab.lua:1243–1276` can consume other legacy Calcs inputs. A whole-container disposition cannot hide those responsibilities.

The retained [fresh witness](../crates/poe-optimizer-pob/tests/support/outside_config_defaults_source.rs) passes eight independent loads in both JIT modes. The two 1,742,265-byte reports have SHA-256 `47380e2c9bbb0ed67ecb523e9b92cb75171dd6b6aebad97494878d6b5e7804af`. Changing the CALCS group or buff mode has observable effects. This is bounded source evidence, not native correspondence or warm-state equivalence.

Existing contracts provide most of the necessary separation:

- [Normalization](owned-normalization.md) takes caller-authored measurements; Core does not consume a PoB view. [Source action resolution](owned-source-actions.md) can resolve an exact physical or Direct occurrence for an explicitly requested MAIN/CALCS context. Reference selection alone establishes no gameplay usage or activation.
- `QueryPresetDraft` contains actual metric requests with exact Actor/Action targets. It has no metric-free editor-focus entry. `EvaluationSelection` selects Query and Scenario independently.
- `ScenarioInput` contains typed external assumptions and usage. There is no native PoB buff-mode shortcut. Translating EFFECTIVE requires real semantic correspondence, not copying the string.
- `SourceDisposition::SourceOnly` and source occurrence IDs can record a reviewed reference-only classification. They do not prove that classification. Existing Calcs presentation authority covers Section layout only. No checked Calcs query disposition exists today.
- The sidecar contains hashes and occurrence links, not the original XML or a structured saved-view record. The standalone `normalize-owned` command currently emits draft, sidecar and report only. A sidecar hash alone is insufficient durable retention.

## Alternatives

| Approach | Benefit | Cost or limitation |
| --- | --- | --- |
| **Source-owned reference settings — recommended** | Preserves the saved tool view while native evaluation remains explicitly queried; no new Core types or guessed metrics. | Must retain the exact source and report the distinction clearly. Restoring the view as native queries remains an explicit conversion operation. |
| Automatically create native Query/Scenario presets | Makes a fully mapped saved view directly reusable in the native UI. | The source selects a view, not a measurement list. Requires an explicit metric policy, exact target resolution and complete assumption mapping; cannot be completed by a default-only proof. |
| Keep the current Pending responsibility | Makes no new ownership claim. | Continues blocking configuration completion despite otherwise understood reference-tool state. Appropriate until this decision and its checks are implemented. |

The recommended option changes the Import admission contract, not the native evaluator model. If automatic projection is preferred, first establish whether existing QueryPreset/Scenario records express all required meaning; do not add a generic PoB mode to the runtime.

## Recommended contract and first implementation boundary

1. **Retain the source meaning.** Bind the reviewed disposition to the exact immutable source snapshot and Calcs occurrences. Retain original XML through the project's source storage or the standalone import publication. Verification must reject a missing or mismatched source. No replacement XML, source-index rewriting or unverified file-path reference counts as preservation.
2. **Separate requested computation.** The disposition produces no native MetricRequest, Scenario assumption, skill activation, objective or default selection. The import report identifies the settings as retained reference-tool state. Existing explicit native query lists remain exact; future “use this reference view” conversion must request or receive real metrics and prove target/assumption correspondence.
3. **Start with the audited frame.** Admit only the unique ordinary Calcs container, the two exact typed default inputs, and separately checked Section layout. Reuse existing bounded source parsing. Reject duplicate, unknown, malformed, namespaced or legacy input fields and unsupported source lifecycles. The expected49Sections is a fixture assertion, never a production rule. Resolve the saved group against the actual selected source preset; do not substitute MAIN or a matching name.
4. **Keep separate responsibilities.** Prove that the admitted frame contributes only reference-query state under the normal-load contract. Do not retire Build MAIN selection, skill usage, Config migration, Scenario or any other source responsibility. Nondefault settings remain Pending until their own bounded frame and consumer proof are admitted, even though the chosen ownership principle can apply more broadly.
5. **Use existing accounting.** Apply a private checked disposition through the existing provenance/accounting pipeline with preflight before mutation. Retire only the matched Calcs responsibilities. Whole configuration completion still requires every header, Config/default/custom source and other responsibility to be accounted for; this change alone does not produce a working build.

Acceptance must retain the unchanged original, separate MAIN/CALCS identities and existing queries; reject crossed presets, missing source retention, duplicate/malformed fields and one-short budgets; and preserve exact replay. The fresh witness corroborates the source interpretation but is not a runtime certificate.

Until approval and implementation, keep the current unresolved Calcs responsibility. No additional omission or warm-load experiment can replace this ownership decision.
