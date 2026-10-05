# Proposal: checked configuration dispositions

**Status:** Proposed; not implemented. No inventory gate is closed by this note.
**Date:** 2026-10-05.
**Scope:** A reusable owned Import contract. Core ownership and native evaluation
remain as described in the [domain architecture](domain-architecture.md).

The next configuration improvement should account for where every source setting
belongs. Known numerical members alone do not prove that configuration choices,
assumptions or usage are complete. A checked source-disposition authority could
complete one inventory while preserving the other inventories' actual obligations.
The first candidate is configuration **build-choice membership**. It is not yet
independently closable by checking Config alone: the current obligation also
guards otherwise unaccounted source outside Config. The evidence below does not
justify completing scenario usage or assumptions.

## Evidence and current boundary

This review inspected the unchanged five XML fixtures, the normalization policy
and Original05 draft in `runs/owned-generated-preset-inputs-05/`, the pinned PoB
source, and these existing reports:

- `runs/owned-configuration-inputs-source-01/source-jit-{off,on}.json`: complete
  catalogue, raw/effective settings, exact modifier records and downstream joins.
- `runs/owned-configuration-reward-source-01/source-jit-{off,on}.json`: reward
  control census, defaults, choices and source correspondence.

These are existing source observations, not new execution or a new parity claim.
The future publication must authenticate their files and pinned source manifest,
not merely trust paths or copy report outputs into production data.

Each original has one ConfigSet and the same 34 numeric Placeholder names. Their
authored Input counts are **17/15/9/9/8**. Non-reward authored Inputs comprise:

| Original | Additional settings |
| --- | --- |
| 01 | Chilled, bleeding, ignited, recent critical strike, recently hit, ordinary and rare/unique nearby-enemy counts, and EHP block-gain policy |
| 02 | Rage, recent melee hit, shocked/chilled, flask use, Twister cold and Whirlwind stages |
| 03 | Enemy dazed |
| 04 and 05 | None |

Original05's eight Inputs are reward selections, but its configuration is not
empty. The source installs **45 effective Inputs: 17 reward controls and 28
non-reward defaults**. Its configuration produces **66 Player and 21 Enemy
modifier records**. These counts describe source observations, not native coverage.

The 28 non-reward defaults include encounter selection, resistance penalty,
resource/repeat/physical-reduction averaging modes, cooldown bypass settings,
conditional state/counts, and child buff/curse enables. Seven enables produce
actual `SkillData` records for Spectre, Companion or Elemental Relic children.
`ConfigTab.BuildModList` does not check widget visibility before dispatching
callbacks. A hidden control or an unrelated selected skill is not an absence proof.

The owned Original05 scenario currently has known encounter/level, fourteen false
raw-override presence flags, a false block-chance presence flag and the injected
`enemyDamageType=Average` default. Its seventeen rewards have complete membership.
Configuration build choices and scenario usage are empty/Pending; assumptions
have sixteen known members but remain Pending.

A follow-up audit of `runs/owned-occurrence-counts-04/original-05/sidecar.json`
corrects the earlier description of "237 configuration origins." There are
**237 origins linked to issue `01f2`**, but only **35 are inside Config**: the
Config container, ConfigSet and 33 Placeholders. Config occupies source ordinals
422-465; its eight reward Inputs and enemy-level Placeholder already have other
provenance. The remaining **202 origins are outside Config**:

| Outside-Config source family | Origins |
| --- | ---: |
| Item `ModRange` | 101 |
| Calcs container, its two Inputs and 49 Sections | 52 |
| Fifteen Skill groups and fifteen Gem children | 30 |
| ItemSet `SocketIdURL` | 6 |
| TradeSearchWeights and its two Stat children | 3 |
| Build, Buffs, TimelessData, Tree, Skills, TreeView, Items, Notes, Import and Party | 10 |

Of those 202 origins, **196 have only this issue link**. These figures come from
joining actual sidecar source ordinals to the unchanged Original05 XML, not from
names inferred from the issue code. The final fallback loop in
`owned_normalize.rs` attaches otherwise unlinked source to `fallback_issues`,
including the configuration role issue. Consequently, even a complete local
Config disposition proof cannot retire `01f2`: doing so would discard the only
current obligation for most of these other origins. Neither element names nor
the presence of some other link prove their semantics are already accounted for.
For example, item ranges, skill inputs and buff selections need their real owner
correspondence; cached layout needs a reviewed source-only disposition.

The existing scalar channels cover four resistances, armour/evasion, five incoming
damage types, three penetration types, block chance and incoming damage type.
Enemy level has a separate proof. Nineteen saved Placeholder names still require
other dispositions/consumers:

| Family | Source controls |
| --- | --- |
| Player state/count | `conditionCorruptingCryStages`, `multiplierWarcryUsedRecently`, `multiplierStunnedRecently`, `multiplierCurrentEnergyShield`, `sigilOfPowerStages`, `multiplierDifferentAmmoFired`, `multiplierDifferentGrenadeFired`, `configBossFaceBroken`, `multiplierWitheredStackCountSelf`, `demonFormStacks` |
| Enemy state/count | `ShockStacks`, `ScorchStacks`, `ChillStacks`, `enemyCriticalWeaknessStacks` |
| Distance | `enemyDistance` |
| Direct defence inputs | `enemySpeed`, `enemyCritChance`, `enemyCritDamage`, `enemyDamageRollRange` |

Relevant pinned source boundaries are `Classes/ConfigTab.lua` load lines 878–978,
dispatch 1169–1241 and set construction 1317–1334. Direct readers include
`CalcSetup.lua:847` for resistance penalty, `CalcDefence.lua:2268`, `:2273` and
`:3412` for critical chance, critical bonus and hit time, and `CalcTriggers.lua`
for trigger policy. An absent `apply` callback is therefore not an inertness proof.
The catalogue contains **663 ordered entries, 564 named entries and 563 distinct
names**; `conditionEnemyExitedPresenceRecently` occurs twice. Preserve catalogue
occurrence identity and order in the offline evidence.

## Proposed Import contract

Add an optional, versioned `ConfigurationDispositionPolicy` to the existing
normalization policy. This is an Import-facing contract with a private checked
proof type, not a second runtime configuration model. Its final Rust spelling and
serialization version should follow implementation review.

The policy binds the source revision, authenticated catalogue/constructor evidence
and exact dependency identities for the existing reward, encounter, enemy-level
and scalar-input adapters. Its injected rows describe source selectors, admitted
lanes and value recipes, presence/default behavior, and **all** applicable owned
destinations. A source setting may affect more than one destination.

| Destination | Required authority |
| --- | --- |
| Known reward, encounter, level or scalar input | Reuse the respective adapter's checked scope/output correspondence proof; a matching field name or a provenance link alone is insufficient |
| Owned build choice | Identify its real owner and declared typed choice slot; an unresolved choice keeps choice membership Pending |
| Scenario assumption or usage | Identify the semantic responsibility; retain its scoped obligation until actual typed values, targets and membership are proven |
| Source presentation/overwritten observation | Authenticated reason showing why this source representation has no independent owned input; preserve relevant origin correspondence |
| Unreviewed or ambiguous meaning | Keep an obligation; do not authorize completion for a potentially affected inventory |

A deferred disposition is not an implemented value or a claim of numerical
parity. It can justify completing a *different* inventory only when reviewed
evidence excludes that inventory's role and the actual remaining obligation is
retained. Do not route all unknown controls to assumptions merely to remove a
choice or usage blocker.

For example, a reviewed reward row whose exact output was proven by the reward
adapter does not additionally require a build-choice record. Conversely, a
default that enables a child effect still needs usage/activation semantics and
applicability; no name-list entry may silently classify it as irrelevant.

Construction must distinguish authored input, saved placeholder, constructor
default, callback-derived observation and genuine absence. The certificate must
cover applicable missing/default behavior over the full pinned catalogue, not
only the names present in Original05. Unsupported controls introduced by a source
update require a new review, even if current fixture results do not change.

## Integration and completion rules

1. Reuse `owned_normalize::source_shape::fresh_config_sets` for the complete
   structural census. Do not add a second XML/configuration parser. Preserve its
   bounded work, original-attribute inspection, duplicate detection and rejection
   of malformed sibling sets.
2. Compile policy identities, selectors, recipes and disposition coverage once
   with the other normalization policies. Charge all scans and proof joins under
   existing limits. A failed or absent proof must not become a cached success.
3. Collect a private per-ConfigSet proof from existing reward/scalar/encounter
   results and the actual source rows. Extend those private proof interfaces
   where necessary; do not infer correctness from already-emitted member counts.
   `ProvenConfigurationInputs` can contain only a successful subset of its
   configured channels, so the mere presence of this token cannot discharge all
   scalar dispositions. Require selector-wise coverage and emitted correspondence.
4. After materialization, check exact owner, scope, ordered output and retained
   obligation correspondence. The policy must account for every actual row,
   source-only observation and applicable default. Custom blocks, unknown names
   and unsupported lanes preserve the affected obligations.
5. Complete an inventory only when every possible contributor to that inventory
   has a proven disposition and all required members are present. Initially,
   scenario usage and assumptions remain Pending. Empty build-choice membership
   is a result to prove, not a configured default.
6. Before retiring any inventory issue, audit **every live origin link to that
   issue**, including fallback links outside the ConfigSet. Each needs checked
   output correspondence, a reviewed source-only disposition, or an actual
   surviving obligation on its semantic owner. A local Config certificate may be
   collected while this global retirement gate remains blocked; it is not a
   complete choice inventory. Do not blanket-reroute outside source to scenario
   assumptions/usage. `source_shape::retire_membership` removes a link on only its
   supplied source row and is insufficient for this shared fallback issue.
   Retire only the proven inventory issue, preserving allocated IDs/watermarks,
   selected queries and other pending issues. Never attach unresolved
   configuration to another preset's fallback issue.

Omitting the optional policy retains previous normalization behavior. Successor
and migration paths must validate old commitments before rebinding dependencies;
a changed reward/scalar policy invalidates the old disposition proof.

## Owned semantics versus source quirks

Owned concepts remain rewards, typed mechanic choices, external assumptions,
usage policies and exact occurrence targets. Game constants, applicability,
domains and numerical rules remain injected owned data. Configuration widget
types, source spellings, constructor order and XML lanes stay in offline
acquisition/Import.

PoB's `count` zero fallback, constructor mutations, duplicate catalogue names and
placeholder overwrites are source decoding/lifecycle facts. They are not new
native value types or implicit evaluator behavior. Translate a demonstrated
meaning into existing typed inputs/rules; preserve uncertainty when a quirk has
not been shown to represent intended game behavior. Do not transfer cached
source outputs or Lua truthiness into a native rule to make a fixture pass.

The accepted preset/scenario ownership stays intact. Global source controls that
eventually affect exact skills must use reviewed target/applicability semantics.
The importer cannot expand only the currently selected skill preset and declare
all other alternatives irrelevant. Any new ownership requirement belongs in a
separate, concrete design decision.

## Validation and delivery gates

- Replay all five unchanged originals and preserve all 110 queries, selected
  axes, resolved values and unrelated issue IDs. Demonstrate that a proven
  choice-inventory retirement leaves usage/assumptions Pending and still prevents
  incomplete native evaluation.
- Add contrasting authored condition/count/mode controls from Originals01–03;
  unsupported controls must remain obligations even when Original05 is neutral.
- Cover independent ConfigSets, reordering, archived activation and malformed
  siblings. No source-name or target resolution may borrow authority from another
  set or a different selected skill/tree/equipment combination.
- Reject unknown controls, duplicates, wrong lanes, malformed quantities,
  extra attributes/children, stale source pins and mismatched dependency proofs.
  Include the duplicate catalogue-name case without collapsing its occurrences.
- Distinguish absence, explicit false, explicit zero and saved placeholders.
  Exercise count-zero fallback and zero-retaining controls separately. Default
  or overwritten-placeholder claims need source mutation evidence, not only one
  saved/effective snapshot.
- Cover empty, disabled, enabled and legacy custom-modifier sources. Unknown or
  nonempty unsupported text must not receive an empty membership certificate.
- Check exact emitted-member/provenance correspondence and failure recovery.
  Work limits, reordered source inputs and A/B/A normalization must preserve
  deterministic IDs and diagnostics; exhaustion must fail without partial proof.
- Include a locally complete Config with unresolved source elsewhere in the
  document. Its global fallback issue must remain. Check all 237 Original05 links,
  including the 196 sole-link outside origins; no orphaned source, stale issue
  reference or unrelated-inventory closure may result from local completion.
- Publish with a minimal authenticated artifact delta and reproduce the package
  byte-for-byte. Test inherited-policy rebinding and verify that dropping the
  opt-in policy restores the predecessor behavior.

New checks should be Rust tests. Reuse the existing optional PoB source witnesses
where their assertions suffice; extend focused source controls only where a new
semantic claim needs evidence. Membership completion and numerical parity remain
separate assertions.

No user decision is required for the accounting seam itself: it realizes the
accepted separation of build choices, scenario assumptions and usage. If review
finds a control whose correct ownership conflicts with that separation or needs
a new public native capability, retain its obligation and present that specific
decision before implementation. This note does not pre-approve such changes.
