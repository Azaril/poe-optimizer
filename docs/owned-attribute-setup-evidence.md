# Attribute setup and cache evidence

This is source evidence for the pending [contribution-stage contract](owned-contribution-stages.md),
not an approved native API or original-build parity result. The reference is the pinned PoE2
revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. Complete native original-build evaluation
remains 0/5. Validation status and counts belong to the implementation checkpoint.

## Actual source groups

`CalcSetup.initEnv` (`src/Modules/CalcSetup.lua:717`) constructs the base player database from
class values and setup constants, then appends configuration modifiers. It calls the complete
`Common.specCopy` at line 914, before override conditions, items and passive modifiers.
On this cold path, those original base/configuration contributions remain in the local store.
When a caller supplies the resulting cached database, line 916 instead attaches that database
as the player store's parent. The parent represents cached contributions for the same player;
it is not a parent actor.

`Common.specCopy` (`src/Modules/Common.lua:542`) copies local modifier-list membership,
conditions and multipliers into a new store. Modifier objects remain shared references.
It does not copy actor output or recursively flatten an existing parent. `mergeDB` at line 530
appends local modifier lists and overwrites local condition/multiplier keys. These operations
are distinct from a semantic regrouping of all actor contributions.

Items enter the local player store through `mergeDB` at `CalcSetup.lua:1605`; passive modifiers
follow at line 1637. Item traversal follows `build.itemsTab.orderedSlots` and the original item
modifier-list order. Passive construction uses `pairs(nodeList)` in `buildModListForNodeList`
(lines 415–464). A numeric node-ID sort is not established by this source. Recording one source
sequence does not authorize treating that sequence as a permanent order after candidate edits.

`ModDB.MoreInternal` (`src/Classes/ModDB.lua:214`) rounds a local product before multiplying the
parent result. Consequently, grouping is observable: two synthetic 1% Strength MORE inputs
placed through configuration and an ordinary parsed candidate ring yield a cold combined factor
of 1.02, but a cached setup has the separately rounded factors 1.01 × 1.01 = 1.0201. Both JIT
lanes verified these exact values on stores produced by complete `initEnv`. Their BASE query
was identical at 100096. Cold setup had all 13 Strength rows local and no parent; cached setup
had 10 local rows and three parent rows. The latter were the class base and the two injected
configuration contributions. These are deliberate source-only component inputs, not
obtainable-roll claims or a claim that any supplied original build has this discrepancy.

## Initialization and reuse

`wipeEnv` (`CalcSetup.lua:466`) clears local player/enemy modifiers, conditions and multipliers.
It selectively retains items, passive allocations and other prepared structures according to
caller-supplied acceleration flags. It does not reset player output. `calcs.perform`
(`src/Modules/CalcPerform.lua:1193`) replaces player/enemy output tables at lines 1214–1215,
then calls the complete actor-attribute function at line 1843. That function prepares additional
conditions before entering `calculateAttributes` at line 488. C0 therefore means the conditions
available at that exact entry, not an empty condition dictionary or merely post-setup conditions.

Ordinary `ModStore.GetStat` (`src/Classes/ModStore.lua:428`) reads actor output, then a supplied
skill-stat fallback, then zero. An existing numeric zero is retained. A fresh setup actor has no
output table; a reused setup actor can still expose old output until `perform` resets it. The
reset, rather than environment reuse alone, justifies missing attribute values being zero at S1.

`GetCondition` (line 409) checks explicit configuration overrides first. Otherwise it resolves
local truthy values, parent conditions, and condition FLAG modifiers. A local false does not
mask a parent true or a FLAG. An explicit `cfg.overrideCond[name] = false` does. In particular,
writing a false comparison after the first or second attribute pass does not erase an independent
true condition FLAG. Native C0/C1/C2 policy must preserve the reviewed query semantics.

The complete `Calcs.getMiscCalculator` (`src/Modules/Calcs.lua:89`) creates cold base caches,
then creates and reuses its accelerated environment. Its first accelerated call has cached
parents even though the environment is new; subsequent calls reuse that environment. A caller
must invalidate changed dimensions: `requirementsItems = true` retains old item assembly even
when a new `repItem` is passed. This is a deliberately invalid caller promise, not acceptable
candidate-edit behavior. Native plan identity/invalidation must bind the changed occurrences.

## Optional reference test

`crates/poe-optimizer-pob/tests/owned_attribute_setup_reference.rs` uses the existing authenticated
complete headless bootstrap. The lightweight item host cannot supply real setup/cache evidence
without fabricating substantial Build/UI state. No source functions or shared hosts are changed.

The carrier is imported original `build-02.xml`; explicit synthetic configuration/ring inputs
are applied afterward. The test exercises cold, cached-fresh and reused environments, repeated
line occurrences and A→B→A candidate replacement, and intentionally incorrect item-cache flags.
A read-only debug call hook observes the unchanged original attribute function during complete
`perform`, including the output reset and C0 resolution. It also checks real-parent condition
fallback and the local-only behavior of `specCopy`.

Separate child runtimes run with JIT off and on. That distinction concerns execution mode;
it is not cache warmness and does not certify that a particular hot trace was compiled. Cold,
cached-fresh and reused setup are explicit states within each lane. Reports are written under
ignored `runs/owned-attribute-setup-01/`. The target passed, and the two reports have identical
`additional_observation` values. Each lane recorded four player attribute entries (cold,
cached-fresh, reused after replacement and reused after restoration): all three initial attribute
reads were zero at every entry. Five condition queries were recorded at each entry, including
true FLAG/parent fallback and removal of transient override/stale conditions. Four candidate
snapshots retained the repeated BASE line orders A=(11,11,3), B=(3,11,11), then restored A;
the deliberately incorrect unchanged-items flag retained A during the requested B edit.

This test establishes source setup/component behavior only. It supplies no native activation,
final-resource or full-build parity credit. A canonical cold-versus-cached correspondence
remains an explicit design decision.
