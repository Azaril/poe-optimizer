# Proposal: checked configuration dispositions

**Status:** Full configuration-disposition authority proposed; not implemented.
The narrower source-presentation precursor and final issue-integrity check are
implemented. Neither closes an inventory; publication status and the current
baseline are recorded in [implementation](implementation.md).
The owner was prompted on the full accounting contract on October 9; that
decision is pending. The current no-backward-compatibility direction below
supersedes this proposal's earlier preservation language.
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

The 2026-10-07 resistance-penalty checkpoint supplies a real Player assumption
and native consumer through the existing external-input contract. Typed
constructor projection now supports Player, Enemy and Environment; the penalty
is one of Original05's 28 non-reward defaults. It contributes to three existing
elemental channels through the once-per-Player Actor rule. Independent fresh
source controls also prove that its numeric Placeholder is unused; an explicit
data permission admits that precedence, without importing the Placeholder value.
This does not complete any configuration, assumptions or usage inventory. See
the [packet](../data/owned/poe2/3887ae68/configuration-resistance-penalty/README.md)
and [current checkpoint](implementation.md) for exact publication and validation.

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
had sixteen known members at that audit and now have seventeen after the
resistance-penalty publication; the inventory remains Pending.

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

### Non-Config fallback audit (2026-10-05)

The read-only follow-up joined the same issue against the current
`runs/owned-occurrence-counts-07/original-05/sidecar.json`, all five unchanged XML
fixtures, and pinned source revision
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. It confirms the 237/35/202 split above.
This is source inspection and saved-evidence analysis, not a fresh calculation or
numerical parity result.

| Family | Proven meaning and disposition boundary |
| --- | --- |
| Calcs Sections | `CalcsTab.lua:217–233` loads only `subsection.collapsed`; `CalcSectionControl.lua:33–64`, `:331–345` and its drawing paths use it for controls and geometry. The five originals have 49/49/49/50/49 saved Section leaves. A checked layout disposition can cover each ordinary `id`, `subsection`, boolean `collapsed` leaf; it cannot cover the Calcs parent or its Inputs. |
| Calcs Inputs | Original05 saves `skill_number=1` and `misc_buffMode=EFFECTIVE`. `CalcSetup.lua:738`, `:806` and `:1890–1891` use them for the CALCS environment, buff-mode flags and selected socket group. These are real reference-query/assumption semantics that need their own correspondence; treating the whole Calcs tree as UI would be incorrect. |
| TreeView | `PassiveTreeView.lua:59–85` loads/saves camera position, zoom, search highlighting and tooltip comparison display. Search and tooltip consumers are at `:830–851`, `:1151` and `:1182`. The ordinary five-attribute leaf can receive a narrowly checked source layout disposition. This does not cover allocated Tree/Spec nodes. |
| Notes | `NotesTab.lua:75–88` loads/saves an editor buffer. All five originals have only whitespace. An attribute-free, whitespace-only leaf has no authored content to convert; nonempty notes remain preserved and unclassified by this narrow slice. |
| Build/Buffs | `Build.lua:1245–1253` saves calculated BuffList/CombatList/CurseList outputs. Its `Load` at `:1138–1178` has no Buffs branch, and the reviewed XML readers do not consume these saved list attributes. This is a candidate for a separately checked cached-output disposition, including nonempty list text; it is not evidence that actual configured buffs are absent. |
| TradeSearchWeights | `ItemsTab.lua:1291–1307` loads weighted stat selections; `TradeQuery.lua:918`, `:927` and `:1119` use them for scoring and generating trade searches. Original05 saves FullDPS weight 1 and TotalEHP weight 0.5. These are search preferences, not cosmetic display, and stay unresolved until their retained/imported intent contract is explicit. |
| TimelessData | `Build.lua:1157–1175` restores settings used by `TreeTab.lua:1049` (`FindTimelessJewel`) and its seed/search criteria. Current originals contain empty search strings and default selections, but the family represents saved search intent. It must not be classified as general layout or confused with an equipped jewel's seed. |
| Party | `PartyTab.lua:518–541` imports actor, aura, curse, warcry, link and enemy modifier data. The current five leaves have only import-display settings (`:568–572`), but any future empty-frame proof must exclude children and authenticate defaults. The general Party family remains semantic. |
| Import | `ImportTab.lua:556–574` includes source/import metadata and sets party export state. `CalcPerform.lua:1221` and its export/link branches consume that state. The source-only meaning of each field needs review; no blanket Import disposition is authorized. |
| Build, Tree, Skills, Items | These containers mix active-preset selection, character level, MAIN selection, source defaults and presentation. Existing selected axes or child mappings alone do not prove every container field. Preserve their fallback links until field-level owner correspondence is checked. |

All thirty Skill/Gem origins in the Original05 fallback are fifteen **generated**
source groups and their children, spread across six saved skill presets: two
Firebolt sources, four item-granted Skeletal Warrior sources, five Sand Djinn
sources and four Water Djinn sources. None of these thirty is a manual Direct
Djinn occurrence. Only the six active-preset rows 208/209, 226/227 and 243/244 have
additional generated-input and preset links; the other 24 have only issue `01f2`.
They require checked generated input, usage, provider applicability and action
correspondence, including dormant presets. A proven quality binding is not a
proof that enabled/count/FullDPS/action fields are all covered. The prior usage
and actor-action source witnesses remain the evidence dependencies; no empty
usage inventory follows from literal `nil` or default-looking saved values.

The implemented bounded precursor is an optional source-bound
presentation policy for ordinary Calcs Section leaves, ordinary TreeView leaves,
empty Notes and independently reviewed empty socket trade URLs. It changes only
the dispositions of individually proven, otherwise unlinked source rows before
the historical fallback pass. Unknown attributes, namespaces, content, malformed
or duplicate frames retain their obligations. It must leave the draft, allocator,
all semantic source links and the live issue `01f2` unchanged. The checked
configuration-disposition authority and global semantic retirement proof proposed
here remain separate work. The implemented final integrity check rejects dangling
issue links; it does not supply that semantic proof. The checked source packet is
`data/owned/poe2/3887ae68/source-presentation-v1/` and contains pinned source
excerpts, not numerical observations or copied calculation outputs.

Publication `runs/owned-source-presentation-03/validation.json` verifies 292
source-only dispositions across the five originals (54/69/54/58/57), identical
canonical drafts and selected requests, all 110 queries and byte-identical package
rebuilds. That presentation checkpoint retained all eleven selected obligations
and left `01f2` with 180 links (35 Config, 145 elsewhere, 139 of the latter
sole-link). The earlier 237-row census and this 180-row result are historical;
the intrinsic range correction below further reduces outstanding source links.

### Cached Build/Buffs accounting (2026-10-07)

The existing source-presentation policy now has an independent opt-in for the
saved Build/Buffs child. The pinned Load function (1138-1178) never consumes it;
Save (1245-1253) writes calculated buff/combat/curse lists. Only a unique, flat,
non-namespaced leaf under the ordinary unique Build frame is admitted, with the
three known output attributes. Actual configured effects, Party data, the Build
container and other children retain their existing semantic responsibilities.
Unknown fields/content, duplicates and wrong parents keep fallback; an existing
semantic link is never replaced by this proof.

The current-release publication at `runs/owned-cached-build-buffs-01` passes in
20.79s. It changes exactly one source disposition in each unchanged original;
all draft values, allocator state, selections, 110 queries and selected issues
remain exact. Original05's configuration-linked origins decrease from 73 to 72
(35 Config-local and 37 outside Config). The whole configuration authority
proposed below remains unimplemented; this bounded cached-output proof closes
no inventory. The [packet](../data/owned/poe2/3887ae68/source-presentation-v1/README.md)
records source pins, malformed-frame checks and reproduction.

### Item-range ownership correction (2026-10-05)

The next intrinsic Import step reuses `ItemRangeAttribution`, the canonical Item
container census and normalization's actual line/emission-to-Modifier allocation.
It attaches each admitted final winning write to its exact Item and verified
typed Modifier outputs. A valid unresolved target instead retains that same
Item's live `item-modifiers-not-converted` issue. These are two different forms
of correspondence; neither closes an inventory or proves final game mechanics.
Fixed-literal targets retain their fractions without claiming a numeric effect.

Only a whole Proven layout can authorize actual modifier-output correspondence.
Flat source framing and exact attribute/content references remain mandatory;
ambiguous Item identities, unsupported lifecycles, malformed fields, overwritten
writes, out-of-bounds indices and nonmodifier emissions retain fallback. This
is not an optional second ownership policy and introduces no new native model.
Sidecar version 19 explicitly distinguishes corrected provenance from historical
outputs, even when the same normalization package is replayed.

The final replay `runs/owned-item-range-origins-03/validation.json` audits all
486 range origins and changes 481. Known output ownership and retained pending
ownership are distinct results:

| Original | Range records | Proven outputs | Actual pending item owner | Unchanged fallback |
| --- | ---: | ---: | ---: | ---: |
| 01 | 74 | 0 | 74 | 0 |
| 02 | 133 | 4 | 124 | 5 |
| 03 | 89 | 0 | 89 | 0 |
| 04 | 89 | 1 | 88 | 0 |
| 05 | 101 | 13 | 88 | 0 |

Original02's five refusals have Proven source layout but Partial modifier-roll
declarations, so layout alone cannot authorize output ownership. Original05's
thirteen proven rows comprise nine fixed literals and four numerical range
lines. At that range-origins checkpoint, live `01f2` retained **79 origins:
35 Config, 44 outside, with 38 outside sole-link origins**. At that checkpoint,
all eleven selected obligations remained, as did every original draft, saved
selection and query. This proved source correspondence, not numerical consumption,
item completeness or full-build parity. The later selected generated-field proof
reduced this to 73; subsequent cached-output and archived-responsibility
checkpoints are tracked in the current implementation plan.

The five-original checkpoint compares against hard-hashed historical sidecars
and drafts from `runs/owned-source-presentation-03/`, rather than regenerating
expected provenance with the new adapter. Reproduce with a fresh output directory:

```powershell
$env:POE_OPTIMIZER_TEST_RANGE_ORIGINS_OUTPUT = 'C:\code\poe-optimizer\runs\owned-item-range-origins-replay'
cargo test -p poe-optimizer-cli --test owned_item_range_origins_cli --locked -- --include-ignored --test-threads=1
```

This checkpoint requires the immutable historical evidence and its package.
It runs no Lua or numerical oracle. The presentation replay below now applies
the intrinsic range correction equally at both policy endpoints; its historical
237/180 fallback counts are no longer assertions about newly generated V19
sidecars. The separate historical checkpoint verifies that deliberate change.

To reproduce the presentation publication with a new output directory:

```powershell
$env:POE_OPTIMIZER_TEST_PRESENTATION_PRIOR = 'C:\code\poe-optimizer\runs\owned-occurrence-counts-07\package'
$env:POE_OPTIMIZER_TEST_PRESENTATION_OUTPUT = 'C:\code\poe-optimizer\runs\owned-source-presentation-replay'
cargo test -p poe-optimizer-cli --test owned_source_presentation_cli --locked -- --include-ignored --test-threads=1
```

The ignored publication test requires the exact predecessor and pinned source
checkout. Ordinary test runs validate the checked authoring receipts without
running PoB. This command exercises import/accounting, not numerical parity.

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

### Generated-group consumer census: next blocker (2026-10-05)

The V19 Original05 audit finds fifteen generated Skill/Gem pairs: one in saved
SkillSet 2, two in set 3, and three each in sets 4, 5, 6 and 1. Selected set 4
has Sand Djinn, Water Djinn and Firebolt pairs 208/209, 226/227 and 243/244.
Their exact providers, saved quality 0 and requested count 1 are already owned;
their usage inventory remains Pending `0503`. Other sets include Skeletal
Warriors and retain generated-input issues `067a` through `067e`. All thirty
origins still link to `01f2`; the 24 dormant origins have no other links.

Do not infer a complete field consumer from those existing input bindings:

| Saved meaning | Required consumer or proof |
| --- | --- |
| Provider/Skill identity, level and quality | Shared exact-provider correspondence and typed raw inputs; existing selected bindings cover only their reviewed domain. |
| Group/source activation | Execution intent. The existing source witness shows disabled non-main sources losing active effects; preserve the distinction between authored activation and constructed source state. |
| Global-effect switches | Conditional consumers, not general activation flags. Reconstruction can force global1 true; establish actual effect/Vaal applicability before mapping either field to native usage. |
| `includeInFullDPS` | Reporting intent, separate from count and activation. Saved `nil` loads false in the reviewed source behavior. |
| MAIN/CALCS action selectors | Reference-computation selection, not a prerequisite for native raw-input admission. Classify minion identity separately: an identity join is different from an actual actor choice. |
| Empty label, source descriptor and item slot | Presentation or exact provider provenance, with strict frames; not native formulas. |

The usage witness and `SkillsTab:SetActiveSkillSet` establish that dormant
groups do not supply the current runtime group list; their saved intent must
still survive. Exact preset ownership can add a link to a real generated-input
or usage obligation without resolving a provider. It cannot replace global
fallback for a whole row until every field has an adequate retained owner.
Generated-input completion currently checks provider correspondence and quality,
so it must not be used to hide unresolved activation or reference selections.
Dormant set 2 has **Complete empty usage**, while sets 3/5/6/1 have live usage
issues `04f0/052b/055a/059e`; adding a set-2 usage link requires a justified
obligation transition, not an invented issue target.

Next, build a bounded consumer census using existing source framing, the shared
resolver, scalar recipes and authenticated usage/FullDPS evidence. Contrast Tree
and Item grants, dormant presets and a manual same-skill case. Separate reference
selection from native execution/input authority before proposing any contract
extension; do not expand `account_reference` as a native raw-input gate by default.
The original audit added no closure or numerical-parity result. The source-only
companion below now tests the bounded declared-stat part of that investigation.

The next bounded proof concerns the global switches. Existing reports
`runs/owned-generated-skill-usage-source-01/source-jit-{off,on}.json` are identical
(64,295,778 bytes, SHA-256
`75ca2ba05fdecc465d4727f43b2a31358ffcd2fb134a80281534bc4d24a867d5`;
manifest `8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675`).
Across 46 cases and three lifecycle stages, 2,265 observed effects have neither
global-effect nor Vaal classification. Both observed Djinn Commands remain
enabled with global2 false. `CalcSetup.lua:1999` gates its switch read on
`hasGlobalEffect`; `CalcDefence.lua:149–174` uses the switches for Vaal count
branches. Reusing the Frost global-effect policy as general generated activation
would therefore misrepresent the observed behavior.

**Those observations are not a non-applicability certificate.** `Data.lua:911–964` lazily
discovers global tags through stat maps and processes base, quality, level and
stat-set modifiers. A cold negative observation is insufficient—the same
structural hazard exposed by Frost. The witness's fifteen-file list does not
explicitly pin `Data/Skills/other.lua` or `Data/SkillStatMap.lua`. The companion
below now includes those dependencies and traverses the declared stat domain.
Complete runtime reach and every switch consumer still need authority before
finite admission. Keep the established Frost exception narrow; do not adopt warm/retry
canonicalization or infer domain-wide absence from these samples.

The reference runtime already verifies the full committed Lua manifest before
execution, including those files. Their absence from the witness's short file
list is a dependency-receipt gap, not execution of unverified source. The main
missing authority is semantic reachability; adding file hashes alone cannot
turn observations into a complete non-applicability proof. A future Import
certificate must also bind its consumer dependencies independently of the
generated catalog-role SourcePin, whose scope omits several calculation readers.

Reuse strict framing, the shared provider resolver and injected source evidence.
A private field-consumer proof can account for the two switches while preserving
activation, reporting, reference-selection and whole-row fallback obligations.
Existing input guards and deferred-usage proofs cover different responsibilities;
do not relabel them as negative semantic authority. If a small Import-only
policy/receipt extension is required, review its concrete contract first. No new
Core/Engine model is needed for field non-applicability. Contrast Tree and Item
grants with a same-definition manual source; reject dormant/unproved providers,
new effects, Vaal/global-effect cases, late-discovered tags and incomplete pins.

Actual activation remains a separate checkpoint: usage programs inherit provider
activation gates and cannot safely write the same gate they require to execute
(`owned_plan/compile/usage.rs:39–60`, `compile.rs:1939–1950`). Establish the
consumer/dependency order before extending activation; preserve one occurrence
graph. The [participation proposal](owned-skill-participation-proposal.md) separates
mechanical supply from an explicit requested execution requirement while reusing
typed usage. It awaits an owner decision. Full DPS likewise needs its own
reporting consumer.

Further source review found an open reachability path beyond catalog stat lists:
`CalcActiveSkill.lua:795` passes `ExtraSkillStat` values from the skill modifier
list into `mergeStatSet` (lines 82–89), which looks up each added key through the
same lazy map. A clean catalog census therefore cannot prove that global flags
never matter for arbitrary builds. The companion witness must record this as
an unproved extra-stat scope until its actual producers/applicability are covered.
Do not publish a field non-applicability certificate from catalog absence alone.

### Declared-stat global-switch witness (2026-10-05)

The optional Rust/Lua source test now traverses the full declared modifier/stat
domain of each requested catalog effect, including root and stat-set maps,
constant and quality stats, base/quality/level modifiers and level tables. It
checks raw fallback entries without invoking lazy map lookup. Importantly,
`CalcActiveSkill.lua:560–561` also reads stat-set names through the root map;
the collector examines that union even when a set-local override is benign.
The complete pinned Lua manifest is verified before execution and twenty
direct dependency hashes are included in the receipt.

Four source cases cover unchanged Original05, a manual Firebolt alongside the
item source, dormant preset 2 and an Original05 repeat. Each records cold data
and fresh/once-rebuilt/twice-rebuilt diagnostics; every observation is repeated
and checked for mutation. Sand/Water Djinn and Firebolt have no reachable global
tag in this declared catalog scope. Cold Frost metadata is false, but its raw
map already exposes a global tag, so the witness correctly refuses it. This
does not choose a new canonical reference lifecycle or enlarge the Frost exclusion.

Both JIT reports at `runs/owned-generated-global-switch-census-02/` are
**8,796,127 bytes**, SHA256
`3f0d7838a7b57357f448da6807b20466a7ab2653b76c32e4e5e2d1651da1cdf8`.
The collector hash is
`17fd956a6c1c5d636f517d80705f52820f155e7a83413ab0d90d4383bffaead8`.
The finite negative test rejects hidden metatables, Vaal consumers, malformed
or dynamic source shapes, extra effects, root-map override hazards and exhausted
work budgets. Both tests passed; logs are
`runs/owned-generated-global-census-{fast,source}-02.log`.
After final lint cleanup, the independent run at
`runs/owned-generated-global-switch-census-03/` reproduces those exact report
bytes. Both tests pass in `runs/owned-generated-global-census-final-03.log`.

Reproduce with the complete pinned source and a fresh output directory:

```powershell
$env:POE_GENERATED_GLOBAL_CENSUS_OUT = 'runs/owned-generated-global-switch-census-reproduction'
cargo test -p poe-optimizer-pob --test owned_authored_skill_membership_source generated_global_switch_census:: --locked -- --include-ignored --nocapture
```

The parent refuses an existing evidence directory. The ordinary finite negative
test needs no complete PoB load; the ignored lifecycle test uses bounded optional
reference children and compares both JIT reports exactly.

The result deliberately retains `extra_stat_scope_unproved`,
`calculation_non_applicability_proved=false` and `native_inventory_authority=false`.
This is reusable optional reference evidence, not a native interpreter or an
Import non-applicability certificate. Next prove actual `ExtraSkillStat`
producers/applicability and the separate field-consumer dependencies before
considering a versioned Import proof. No source link or selected obligation
is retired by this witness. Requested participation and reporting remain
independent contracts; the [participation proposal](owned-skill-participation-proposal.md)
also records the selected-group preview bypass and fallback-identity hazard.

### Bounded extra-stat parser controls (2026-10-05)

The optional `generated_extra_skill_stats_source` witness follows two real
`ModParser` paths through saved `CustomModifierBlock` records. The controls
produce a Purifying Flame global-debuff stat and a Lightning Trap ailment stat,
each tagged with its actual SkillName filter. These are diagnostic custom inputs,
not claims about obtainable PoE2 items or valid optimizer edits.

The initial missing-name alias hypothesis was rejected during independent source
review: `ModStore.lua:895` converts an unresolved candidate name to an empty
string, while the tag's unresolved name remains nil at line 904. They do not
match. The witness checks real parser records and actual `List` results for
Sand/Water Djinn primaries, both Commands and Firebolt. Original saved switches
and explicit true/false controls must leave these unrelated effects unchanged.
It inspects raw maps without warming them and uses the existing fixed fresh and
two diagnostic rebuild stages. No alternate oracle lifecycle or native bug
compatibility is introduced.

The test passes six cases in both JIT modes, each with cold metadata and the
three fixed calculation stages. It retains both manual and tree-granted Djinn
occurrences by exact source ordinal; the definition alone does not identify a
unique skill instance. Both reports at
`runs/owned-generated-extra-skill-stats-source-02/source-jit-{off,on}.json` are
**8,411,590 bytes**, SHA256
`87b5347bbf2a06f564692d786db9f2c726ddb1eb00ba8de21ce2b5a6ab5a0357`.
The observer hash is
`d57b91727fb5e376b6e1c2a7fc7d609f03099a4d9967b1c86ff463bc73a4860e`;
seventeen explicit source pins supplement the verified full manifest. Log:
`runs/owned-generated-extra-skill-stats-02.log`.

Reproduce with the pinned optional checkout and a fresh evidence directory:

```powershell
$env:POE_GENERATED_EXTRA_STATS_OUT = 'runs/owned-generated-extra-skill-stats-reproduction'
cargo test -p poe-optimizer-pob --locked --test owned_authored_skill_membership_source generated_extra_skill_stats_source::actual_custom_mod_skill_name_controls -- --ignored --exact --nocapture
```

This result covers only these two parser paths and exact source contexts. It cannot prove all
modifier origins, supplier ancestry or applicability after arbitrary candidate
edits. Complete producer/applicability scope and field-consumer correspondence
remain prerequisites for an Import non-applicability certificate. Keep the
existing `extra_stat_scope_unproved` refusal and all live input obligations.

### Cache authority and remaining supplier proof (2026-10-05)

The authenticated ordinary lifecycle **loads the stored ModCache**. The comment
in `HeadlessWrapper.lua:37` is not effective cache disablement in this revision:
`Main.lua:123–130` loads it unless dev-mode Ctrl or `REGENERATE_MOD_CACHE=1`
selects regeneration. The reference host clears that variable and `CI`.
The existing data-ready hook runs after Main and bounded initialization finish,
immediately before the original XML load. It is therefore the appropriate place
to distinguish a parser cache hit from a real cache miss.

The bounded extension in `generated_extra_skill_stats_source` is **validated**.
It checks both exact custom lines are absent at that hook,
the selected control's complete cache entry appears afterward, and the other
line stays absent. It authenticates the original parser and the identity of its
cache upvalue with the public cache, preserves that state during observation,
and adds an explicit ModCache pin. The earlier source-02 receipt above remains
unchanged; it does not validate these new assertions.

Both JIT reports at `runs/owned-generated-extra-skill-stats-source-04/` are
**8,421,533 bytes**, SHA256
`3f92d29d33d3814b346f524462692435d4312e0381bb19ebc6b0f4c0922a626d`.
The observer hash is
`92b3e387167196d403590be0c5b5ee7d32e2c496a6016ae2f55a3fa3b66fc03d`;
the test passed in 26.39 seconds. Log:
`runs/owned-generated-extra-skill-stats-04.log`. Source-03 remains a failed
authentication attempt: the public parser begins at `ModParser.lua:7404`, not
7403. Source-04 validates that corrected exact source binding; no source or
native behavior changed.

The [bounded producer/transform audit](../runs/owned-extra-stat-producer-audit-01.md)
finds fourteen literal emissions across eleven parser patterns, with thirteen
distinct stat keys, plus three cached Bloodbarrier materializations. Each
literal producer has a SkillName or SkillId filter excluding the reviewed
Djinn/Firebolt identities. That inventory alone does not prove every supplier
preserves those filters. The next proof must cover:

- Admitted item, allocation, support and configuration suppliers, including
  nested SocketProperty, GroupProperty, NodeModifier and actor-modifier records.
- Modifier ancestry, copy/scaling and tag/name transformations at the point
  `CalcActiveSkill.lua:795` consumes extras; a later active-skill snapshot alone
  cannot establish what was available at that point.
- Formatted Party modifiers and opaque callbacks, with an explicit refusal for
  unknown records rather than inferring absence from the current empty input.
- Every effect variant and applicable preset/provider context. Each reachable
  extra stat needs a preserved identity exclusion or inclusion in the complete
  lazy-map check. Current-context evidence must not authorize arbitrary search
  candidates or dormant presets.

### Item-granted SkillId control (2026-10-05)

The existing `generated_extra_skill_stats_source.{rs,lua}` witness now exercises
an item-derived matching receiver. It derives the equipped Boots from the saved
active ItemSet and inserts a diagnostic granting line into that exact item's
text. It preserves the XML element structure, source ordinals, other equipment
and saved selectors. This is a transport control, not evidence that the item is
obtainable or the changed build is legal in game.

The cached 5s/50% line (`ModCache.lua:8324`) and uncached 7.125s/53% variant each
produce one `ExtraSkill` and three `ExtraSkillStat` records through the original
parser (`ModParser.lua:3670–3675`). The payloads retain their exact
`SkillId=BloodbarrierPlayer` filters. Observation follows parsed/base and
slot-specific item records, item/player stores, the source item and generated
group, and the actual Player receiver's parent stores and effect configuration.
The same records are present in the Djinn primary/Command and Firebolt ancestry
but filtered out for those exact controls. Removing only the inserted granting
line restores the original XML and original observed states; it does not remove
the Boots or retarget another source.

The source's slot-specific copies acquire `sourceSlot` in `Item.lua:2439`.
`ModStore.lua:66` uses `false` for a terminal parent, whereas an extracted plain
list has no raw parent. The optional observer records these separately from a
table parent edge. Neither representation becomes a native domain convention.
Source-05 failed because the first observer counted that false sentinel as an
edge; its raw report is retained. This was an observer assertion error, not an
observed PoB calculation discrepancy or a determinism exception.

Source-06 then exposed a different, real source-order property: the uncached
repeat swaps only the duration and PhysicalDot entries in `emitted_skill_data`
across MAIN/CALCS and all three stages. Complete records, tags, payloads, maps,
ancestry, final skill data and numerical outputs agree. `CalcActiveSkill.lua:87`
iterates `pairs(stats)` before appending records; lines 896–900 later assign
these distinct SkillData keys separately. The failed raw comparison is retained.
The bounded comparison contract preserves a raw report and projects only this
leaf into the three unique keys `duration`, `PhysicalDot` and `debuff`, retaining
their complete records. Duplicate or unknown keys fail. Every other field and
array remains exact, including all calculated outputs. This follows the existing
Bidding diagnostic/channel distinction; it is not retry-until-pass, tolerance,
general modifier sorting, or an additional reference exception. Native plan and
numerical determinism requirements remain unchanged.

The receiving effect emits duration and `PhysicalDot` SkillData through the
original merge. Its local physical-damage map overrides a different global
`PhysicalDegen` map; a stat name mentioning “per minute” does not by itself
establish the effect's units. The third payload, the corrupted-blood stack
marker, has no local or global modifier mapping. Independent `baseMods` supply
`debuff=true` and `dotIsCorruptingBlood`. Preserve the unmapped input and its
uncertainty; this observation proves neither stack count nor on-block mechanics,
and does not classify the marker as game-inert or an upstream defect. The
[Lua cleanup register](legacy-retirement.md#lua-compatibility-cleanup-gate-requested-2026-10-05)
records the corresponding native-conversion requirements.

Source-07 **passes** in `runs/owned-generated-extra-skill-stats-07.log` (44.34
seconds). Both semantic reports at
`runs/owned-generated-extra-skill-stats-source-07/source-jit-{off,on}.json`
are 15,881,807 bytes, SHA256
`6a26b99fd750b72a12aa5b392a7a44add7e63a16a31eb3dc268868b67a2bcaa5`.
Raw reports are retained as `source-jit-{off,on}.raw.json`; they agree in this
run but their cross-key ordering is not a required invariant. The observer hash
is `7a2967312e78d965df4294245403c9ffa8d4be29e24f56540f17bf659158e964`.
The two Rust comparison-boundary tests pass in
`runs/owned-generated-extra-skill-stats-projection-01.log`: they preserve all
other arrays/values and reject unknown, duplicate, altered or incomplete records.
Source-05 and source-06 remain failed diagnostics, not passing receipts.

The test retains the earlier six cases and adds cached, uncached, removal and
uncached-repeat controls, with three lifecycle stages and independent JIT modes.
It explicitly pins nineteen source files in addition to authenticating the full
manifest. A Config custom modifier alone cannot replace this control: actual
item/node grant inventories construct the generated receiver.

This is a source-only producer, transport and filter test. It does not certify
universal extra-stat absence, close any original's global-switch field or retire
any of the five remaining Original05 issues or the then-current 79 configuration
origins. The remaining supplier and transformation obligations above still apply.

**Required scope for the bounded proof below:** capture the actual `extraStats` argument consumed at
`CalcActiveSkill.lua:795` for unchanged Original05, with contemporaneous
store/config/actor/source-instance and stat-set joins. Authenticate the original
call, preserve arguments/results and check observer noninterference. Bind exact
source bytes, manifest and selected `Skills4 / Items2 / Spec3 / Config1`; changes
to those inputs invalidate the receipt. The frame has nine equipped uses over
eight items, including two distinct ring uses of Item26. Retain source-slot
attribution and Item23's zero-factor Amulet-copy path. Include effective allocated
nodes and attribute overrides, retained support/skill records, and the actual
Config defaults/callbacks. Empty selected tree sockets/rune suppliers and Party
need authenticated source-shape exclusions, not assumed global absence.

Post-load empty lists remain diagnostics. A preserved mandatory SkillId filter
can justify an exact receiver exclusion; item/source names or a sampled absence
cannot. Unclassified nested records, callbacks or transforms remain unresolved.
This is one finite original-context consumer proof, not completion of all parser
families, arbitrary search candidates or dormant presets. Reuse the existing
exact resolver and private disposition machinery. Even a successful selected
proof cannot close all thirty Skill/Gem origins or retire issue `01f2`.

Reuse the existing catalogue/map witness, exact generated-source resolver,
source shape and typed value recipes. There is no new interpreter or native
producer/filter language in this work. A future checked Import disposition may
consume a finite receipt only after those dependencies are complete. It cannot
close a whole Skill/Gem row, retire shared issue `01f2`, invent a Pending usage
link for an already Complete preset, or settle activation and reporting. The
accepted generated raw-input ownership remains independent of the accepted
[participation contract](owned-skill-participation-proposal.md), whose implementation
remains open.

### Actual extra-stat consumer witness (2026-10-05)

The optional Rust test `generated_extra_stat_consumption` now observes the actual
original merge call at `CalcActiveSkill.lua:795`, without wrapping a business
method or requerying the list. Source03 passes in 58.22 seconds. It uses unchanged
Original05 in two independent instrumented VMs and an uninstrumented control,
retaining three lifecycle stages per case and both JIT modes. It verifies caller,
effect/config objects, original return observation and noninterference with the
captured numerical outputs. The fresh instrumented state contains 189 calls.
Both MAIN and CALCS include tree-generated Sand/Water Summons and Commands and
item-generated Firebolt, with exact saved-source attribute correspondence.

The supplier frames preserve the nine equipment uses, exact item/source slots,
Config inputs and placeholders, allocated-node modifier data, support/effect
records and contemporaneous store ancestry. Attribute overrides retain node
identity, scalar override values, stats, modifiers and exact effective-node joins;
unrelated graph topology is explicitly outside scope. Original selected XML,
including Overrides, remains source evidence. Source01 failed by trying to encode
the entire override node graph; that observer error is not a source or native
calculation defect.

The allocated-node projection in source03 is **local storage only**, not complete
effective modifier data. A follow-up audit found 30 of the first environment's
57 nodes had no local `modList`, three had local empty lists and 24 had nonempty
local lists. Ordinary spec nodes inherit their tree node via `PassiveSpec.lua:65-68`
and `PassiveTree.lua:223`; actual `CalcSetup.lua:208-215` and `:425` reads use that
inheritance. The observer's `rawget` must not be used as evidence of absence.
The exact extra-stat call arguments remain valid observations, but supplier
classification needs the effective lookup and original node-builder output.

Source02 passed per-mode validation but failed cross-JIT raw equality. All 2,016
differing leaves were the same two Bidding II records interchanged in 504 local
ancestry containers. The pinned `pairs(stats)` emission at `CalcActiveSkill.lua:87`
is consumed into separate named ModDB channels at `CalcPerform.lua:1161–1166` and
`ModDB.lua:31–37`. Source03 reuses this reviewed distinction: only the exact
Damage/MORE/30 and CooldownRecovery/INC/30 records at occupied positions `[1,2]`
receive a keyed comparison view, bound to manual Sand/Water groups 5/9, their
Summon/Command effects, stat set 1, ancestry depth 0 and the retained Bidding II
support at level 1/quality 0. Every complete record is retained. Other records,
same-channel ordering, values and numerical outputs remain exact. This is neither
a generic sorting rule nor a new reference exception.

Both compared reports are 62,779,929 bytes, SHA256
`ba78f8c250fac46ffce671eec6522480637d8c0928a861908abce48fcf5c7795`, in
`runs/owned-extra-stat-consumption-source-03/source-jit-{off,on}.json`.
Unmodified raw reports are retained separately, each 62,740,349 bytes: JIT-off
`09b53dd070208032e0bd34d06490b042758b8d74e4935905a1423fd82eb0245b` and JIT-on
`4020b35f753786d6ab334ea1973bb024592bff3ed7ad1aafa56574a6c42f6a93`.
Three focused projection tests and the shared bounded-diagnostic tests pass in
`runs/owned-extra-stat-consumption-projection-01.log`. Unknown, duplicate, moved
or altered Bidding records cannot acquire this comparison treatment.

Source04 **passes** in 78.29 seconds (`runs/owned-extra-stat-consumption-source-04.log`).
Schema 3 retains local diagnostics and adds authenticated effective `modList`
and `keystoneMod` lookups, exact spec/tree-node correspondence and the original
`buildModListForNode` return at `CalcSetup.lua:411`, called by the original list
builder at line 435. It captures both the include-keystone and ordinary passes
before any CALCS scratch reuse. The observer copies the per-node return sequence
at supplier capture; later calls cannot append to an earlier snapshot. Each
returned modifier list and per-node call sequence retains its order. Cross-node
invocation order is not captured or certified as interchangeable arithmetic.

The first fresh state has 189 extra-stat calls, 21 environments and 2,394 node
returns. Every environment retains the same 57 selected nodes and two returns
per node. Thirty effective lists are tree-inherited, containing 33 records;
27 lists are local. Forty-five lists are nonempty. Local absence is now visibly
distinct from an inherited empty or nonempty list. Numerical outputs, actual
extra-stat calls, other supplier fields and the old local node projection agree
with source03. The new evidence remains return/consumer-time observation, not
pristine pre-transform input or a general transformation law.

Both compared source04 reports are **104,176,889 bytes**, SHA256
`134f3399d90c8a819a9774cd89e4d2b0d630d1f165b1b67287d6d4ac2c05c00e`;
raw reports are **104,137,309 bytes**, SHA256
`fc1237b7c54e1fd92ba063c5c39e4b33524c31fe28d240e3fb98ff02c8099d96`.
They live in `runs/owned-extra-stat-consumption-source-04/` under the same
`source-jit-{off,on}[.raw].json` naming. Raw reports also agree in this run;
the established bounded Bidding comparison contract remains unchanged.
The observer hash is
`304973af16a8b1dbe31e70ea5cc678e30d53c80b74f418132fd2f806b3a75483`.
`PassiveTree.lua` joins the pinned dependency list. The diagnostic artifact cap
is explicitly 128 MiB, with no truncation; the existing four-million work,
2,048 consumer-call and 32-environment limits remain, with at most 4,096 node
returns and eight returns per node. Seven focused Rust checks pass, including
inheritance/identity/return-proof rejection controls and bounded diagnostics
(`runs/owned-extra-stat-consumption-projection-02.log`).

Reproduce with a fresh directory and no child selector:

```powershell
Remove-Item Env:POE_EXTRA_STAT_CONSUMPTION_CHILD -ErrorAction SilentlyContinue
$env:POE_EXTRA_STAT_CONSUMPTION_OUT = 'C:\code\poe-optimizer\runs\owned-extra-stat-consumption-source-fresh'
cargo test -p poe-optimizer-pob --test owned_authored_skill_membership_source generated_extra_stat_consumption::unchanged_original_extra_stats_reach_the_actual_consumer --locked -- --exact --ignored --nocapture
```

This establishes consumer-time evidence, not a complete supplier/transform law,
field non-applicability certificate, dormant-preset proof or native parity.
Item23's copy transformation and arbitrary NodeModifier chains remain separate
obligations. Next classify the captured fields and mandatory filters against
their exact producers and targets before authoring any Import disposition.
At this source-only checkpoint, all five Original05 issues and its then-current
79 configuration-linked origins remained open. The later accounting result is
recorded below.

**Amulet transport follow-up, source05:** the original `CalcSetup.lua:1667`
call is now joined to original `ModStore.ScaleAddMod` and its actual
`ModDB.AddMod` call/return at `ModStore.lua:117`. Item23 supplies Spirit/BASE/13
and GemProperty/LIST with `key=level`, `keyword=minion`,
`keyOfScaledMod=value`, value 1. Both copied payloads become zero at the saved
factor zero, but both records are actually inserted. The observer retains the
complete original, rewritten-copy argument and delivered record separately,
with exact selected Item/slot/list/receiver identity, sourceSlot, insertion
position and preserved original item records. It does not re-query the accessor
to manufacture the observed call arguments. Existing bucket objects are checked
by identity; this does not claim a complete receiver-store snapshot.

Schema 4 passes in 77.89 seconds across two independent unchanged Original05
instances, the uninstrumented control, all three fixed lifecycle observations
and both JIT modes (`runs/owned-extra-stat-consumption-source-05.log`). Every
observed supplier environment has both ordered deliveries. The fresh frame has
42 Amulet returns across 21 environments, 189 extra-stat calls, 2,394 passive
returns and 1,820,829 charged work units. All source04 actual calls, numerical
outputs and prior supplier fields remain exact. The four-million work,
32-environment and 128 MiB artifact limits stay unchanged; Amulet returns are
additionally bounded to 128 per frame and eight per environment.

Compared reports in `runs/owned-extra-stat-consumption-source-05/` are
104,791,002 bytes, SHA256
`d0d93a4a98e7b1b97aa78aa5147e2c7a276913b2d7f91ba9b89db1e7130e4ada`.
Raw reports are 104,751,422 bytes, with JIT-off hash
`c41cfe7d0a68aedd965b91783053dd7d7587517fc10ffefc799067e771d56617`
and JIT-on hash
`b1e2225ed9f343efda6542599018988761eaee68e0bae893d5b4e010e13e58f6`.
The existing narrowly checked Bidding ordering projection remains the only
comparison normalization; this is not a new tolerance or oracle exception.
The Amulet observer SHA256 is
`3fd06d0919665f86889d61d1c38673173bc3a960d3957dc2c9fc499efba9cd38`.
Source04 remains immutable historical evidence.

**Next:** classify these exact supplier/filter dispositions and prove their
transport to the captured consumer before closing source obligations. The
selected zero-factor observation is not a general Amulet-copy law, permission
to drop zero records in acquisition, or a requirement for native zero-record
storage. A node's effective fields observed at return/consumption are not
pristine pre-transform input: its builder can mutate shared records, including
weapon-set tags. Keep arbitrary NodeModifier transforms, dormant presets and
all five selected input issues open until their own evidence is established.

### Bounded generated-source accounting follow-up (2026-10-05)

**Accepted and implemented; all-five reimport validated 2026-10-06.** This bounded
Import step accounts for source fields of exactly resolved generated occurrences.
It preserves unresolved usage semantics on their actual preset obligation;
it does not prove that global switches are inert. Source05 authenticates one
unchanged selected context; it must not become a source-hash allowlist or a general `ExtraSkillStat` absence
rule for imported builds and search candidates.

The implementation reuses the existing private seams without adding a policy:

- `generated_skill_sources::resolve` supplies the exact provider, source pair
  and preset correspondence. Do not independently match provider strings.
- `generated_skill_inputs::materialize` and
  `usage_inputs::occurrences::materialize` return private receipts for the
  actual emitted quality binding and requested-count policy on that same target.
  Existing origin links or matching member counts alone are insufficient.
- `skill_input_disposition::attach_pending_usage` supplies the accounting
  precedent. The shared `pending_intent_usage` lookup inspects
  `preset.intent.usage`, requires an existing, attached, same-preset Pending
  obligation and allocates no issue.
  Do not attach this obligation to item/tree provider rows outside the SkillSet.
- A private disposition proof runs after those materializers and removes
  only individually proven fallback links. Reuse source framing, scalar recipes,
  bounded work and exact output checks; do not add another build model.

Every attribute and descendant of a Skill/Gem pair needs an explicit disposition
before either row can lose its configuration fallback:

| Field family | Required correspondence or retained responsibility |
| --- | --- |
| Provider identity, source/slot, saved level and quality | Existing checked provider/raw-input outputs, including provider authority over level and exact preset-owned quality. |
| Count and group count | Exact existing requested-count projection; no invented count-one value or empty-usage default. |
| Group/Gem enabled, global1/global2 and FullDPS inclusion | Reviewed usage/reporting responsibility retained as Pending on the same preset. This does not resolve participation, reporting, switch values or applicability. |
| Minion identity and MAIN/CALCS action/stat-set selectors | Exact singleton Actor correspondence and authenticated finite reference mapping. A numeric index range or matching display name is insufficient. |
| Empty label and generated-only corruption sentinels | Narrow source-representation proof for the admitted frame; no general corruption default or blanket presentation classification. |

Generated reference correspondence remains a concrete gate. The current
accounting implementation reuses the already-compiled topology/selector checks
through a private inspection seam after exact generated-provider proof. It does
not construct a Direct request or relax public manual-source authority. Unproved
reference fields keep the whole pair's fallback. All-five accounting validation
passed; the separate participation contract is accepted with implementation open.

Acceptance tests must cover Tree and Item positives; repeated/manual/generated
identity separation; absent versus empty or literal `nil` source; unknown or
duplicate attributes/children; malformed values; missing, dormant or ambiguous
providers; absent, Complete, foreign or detached usage obligations; mismatched
raw/count outputs; and unknown Actor/action/stat-set selectors. The five-original
historical comparison must retain draft values, allocator state, all 110 queries,
issues and every unrelated origin exactly. No runtime usage or numerical rule
changes are part of this accounting step.

The all-five historical comparison passed in **5.89s**. Evidence:
`runs/owned-generated-field-accounting-01/validation.json`.

| Original | Changed source ordinals |
| --- | --- |
| 01 | 210/211 and 214/215 |
| 02, 03, 04 | None |
| 05 | 208/209, 226/227 and 243/244 |

These ten origins replace only configuration-fallback links with their existing
same-preset Pending usage issue links. All draft values, IDs, allocator state,
saved selections, 110 queries and unrelated origins remain exact. Every sidecar
uses V21, with the CLI SHA verified against its exact written bytes. Original05's
six links now reduce `01f2` from the historical **79** origins to **73**:
35 Config and 38 outside, with all 38 outside retaining `01f2` as their only Issue
link. Usage issue `0503` and shared issue `01f2` both remain live. All 24 dormant generated
origins remain unchanged; set 2's Complete empty usage receives no invented
Pending obligation. **That checkpoint had 73 configuration-linked origins, five
selected Original05 issues and 0/5 complete native evaluations.** This does not
complete support-origin discovery: local assignment order is not proof of all
contributing sources.

### Archived generated-source responsibility (2026-10-07)

The subsequent audit found that seven archived Djinn pairs already have exact
same-preset Pending owners for generated inputs, usage and support discovery.
Their provider correspondence remains unresolved, but their reviewed source
fields do not represent global configuration choices. The existing generated
accounting path now recognizes that distinction without creating a target or
known input. The [accepted accounting design](owned-generated-skill-dispositions-proposal.md#archived-source-responsibility-2026-10-07)
records the strict source-frame, type, selector, duplicate and retained-owner
gates; the implementation plan records executed all-five results.

This does not generalize to every archived source. Unknown Warrior correspondence
and Firebolt's absent Pending usage witness retain fallback. No Config field,
container, external assumption or scenario usage inventory is completed by this
change. Global `01f2` remains live until every remaining contributor has its real
owner or a proven source-only disposition.

### Overwritten raw-override placeholders (2026-10-07)

The raw-only numeric rows of the existing configuration-input policy now carry
a private receipt for a strictly framed saved Placeholder. Successful projection
alone does not retire its configuration fallback. The accounting pass checks the
same ConfigSet, exact resolved encounter, unique Scenario/Choice owners, attached
Pending configuration issue and actual emitted Enemy presence/optional-value
rows. Those rows must match in type, value, target, multiplicity and order.

Only a source row linked solely to that configuration issue can become
`SourceOnly(overwritten-config-placeholder)`, retaining its exact ScenarioPreset
relationship. No draft value, ID, allocator state, issue or completion changes.
Unknown names, other links, malformed or duplicate fields, string lanes, failed
projection and unmatched encounters retain their obligations. Fallback/default
recipes are separate; enrolling a field in a raw-only recipe is a reviewed claim
about source semantics, never a workaround for an unimplemented consumer.

The pinned Pinnacle callback overwrites four resistance, two rating, five incoming
damage and three penetration placeholders (`ConfigOptions.lua:2071-2096`).
`ConfigTab.lua:883-920` loads Input and numeric Placeholder into separate lanes;
string Placeholder instead writes Input and is outside this proof.
`countAllowZero` keeps explicit zero for resistance/rating callbacks. Later
`CalcOffence.lua:686-698` also reads raw resistance Input for caps, so equal BASE
values cannot justify erasing presence. Incoming damage/penetration readers use
Input first, then the effective overwritten Placeholder (`CalcDefence.lua:2279-2288`).
Their calculated defaults remain injected native rules, not imported saved values.

Current configuration-policy imports identify this changed proof with sidecar
schema/domain 23. Historical schema21/22 receipts remain evidence; there is one
current importer. See the implementation checkpoint for executed validation.

The expanded complete-source witness passes 29 cases in both JIT modes in
121.88s. Its six added controls pair changed/absent placeholders with absent,
zero and signed/fractional explicit Inputs for all four resistances. Both fresh
reports in `runs/owned-raw-override-source-01/` are 37,752,151 bytes, SHA-256
`b16d8d172a2f9b60f78d3efe082c40bfbee1913d1b5eb2f797574fb3631b47f7`.
The ordinary test remains active; it retains a fresh output directory rather
than overwriting old evidence. Set `POE_CONFIGURATION_INPUTS_SOURCE_OUT` to a
fresh directory to reproduce the `owned_configuration_inputs_source` target.

The other ten fields reuse authenticated, unchanged source controls: both rating
reports are 15,196,149 bytes with SHA-256
`3fc1dd33273abc188dcd81b231f6bf43f275a89885d96dd158816eacc58c7112`;
both incoming-damage reports are 6,885,033 bytes with SHA-256
`2bcde65841703d5072e0ccb67f4c334aac76fe31e9cfcab103b9b59d93cbbef7`.
All 23 explicit source pins in each retained report were checked against the
manifest and normalized checkout. These are source/component facts; this proof
does not establish final resistance, EHP or complete build parity.

### Remaining consumer priorities (2026-10-07)

The nineteen other saved placeholders need their actual consumers:

- Twelve `count` callbacks treat Input zero as fallback to Placeholder and skip a
  resulting zero. Existing numeric fallback recipes deliberately preserve zero;
  reusing them unchanged would be incorrect. `ValueRecipe` chooses a present tier
  before decoding and cannot express this through numeric aliases. Add reviewed,
  injected source-decoding semantics in Import when implementing these controls;
  do not put PoB widget truthiness in Core or Engine.
- Three `countAllowZero` controls (`sigilOfPowerStages`,
  `multiplierWitheredStackCountSelf`, `ScorchStacks`) retain zero, but still need
  target, constructor-placeholder and numerical-consumer coverage.
- `enemySpeed`, `enemyCritChance` and `enemyCritDamage` are overwritten by boss
  dispatch, then read directly. The next selected numerical slice is critical
  chance/damage transport followed by the actual `EnemyCritEffect` consumer.
  Speed feeds survival time, which is outside the current selected queries.
- `enemyDamageRollRange` is read within the boss-skill branch; investigate a
  narrow non-applicability proof for the exact no-boss-skill encounter.

For critical effect, `CalcDefence.lua:2268-2274` consumes Never/AlwaysCrit,
configured and overridden chance, Player/Enemy chance modifiers, configured
evade chance, unlucky critical chance, Enemy critical multiplier and Player
extra-critical-damage reduction. Importing two inputs is not completion of that
formula. Add real typed contribution channels and prove their producers before
connecting the result to incoming damage/mitigation. Preserve source branch
order, clamps and unlucky squaring; baseline zeros are not absence proofs.
Multiple-source flags also depend on the accepted
[typed Boolean contribution contract](owned-boolean-contributions-proposal.md),
approved and implemented on 2026-10-07. The published passive flag inventories
remain Partial; critical-specific producers and shared Actor reads of computed
Enemy results are separate gates. Do not replace them with numeric flag counters
or assumed false values.

Distance needs special care: its count callback falls back from Input zero,
whereas direct reads in `CalcActiveSkill.lua:674,688` retain zero. One effective
scalar must not erase both meanings. Existing distance source controls can be
reused. The shared configuration frame proves structure, not semantic closure:
it admits well-framed unknown names and custom blocks. Config/ConfigSet and
complete assumptions/usage remain unresolved until every responsibility is owned.
Support-origin composition and FullDPS reporting remain separate design decisions.

### Default-encounter branch dispositions (2026-10-07 investigation)

The `enemyDamageRollRange` consumer is inside the pinned boss-skill callback's
non-`None` branch (`ConfigOptions.lua:2190,2203`). Its widget has no apply callback.
All five saved originals contain a numeric Placeholder for this field and no
boss-skill selector. Widget visibility alone is not the proof: actual callback
branch execution and downstream output controls must establish non-applicability.

The current encounter policy proves an exact identity from injected absent
selectors. It does **not** encode which other fields are inactive in that branch.
Its catalog variant is opaque; a matching encounter ID or absent-selector name
is not new disposition authority. The existing numeric-placeholder accounting
requires actual emitted raw-input correspondence, and presentation-only
accounting cannot cover a mechanical field. Do not hardcode this field's name,
its value 70, a boss-selector relationship, or an encounter ID in Rust; do not
fabricate an unused native input to obtain an accounting proof.

If this becomes the next useful input-closure step, review a small injected
numeric-placeholder disposition permission on the existing encounter policy.
Each row would carry a recipe for one `PlaceholderNumber` source selector and
its finite numeric grammar, plus required absent-selector guards. Every required
guard must belong to that policy's authenticated encounter preconditions. Source
evidence must establish the row's branch relationship; the recipe is not a
general ignore list. This is a proposed public Import permission, not an
implemented private optimization or a new Core/runtime rule model.

Reuse the complete fresh ConfigSet census, exact scoped Encounter proof and
unique Scenario/Choice/live-issue accounting. Retire only a proven leaf's sole
configuration-issue link, retaining its Scenario link. Preserve authored Inputs,
string aliases, unsupported or malformed rows, all draft values and every
inventory obligation. Test changed finite values independently of saved 70,
missing/wrong guards, selector writes in every lane (including explicit `None`),
multiple ConfigSets, ambiguity, source updates and bounded-work failures.

For current Original05, such a proof would account for one source leaf only;
the five selected input obligations would remain. The critical consumer and
already approved Sand preparation remain more useful immediate implementation
work. This investigation does not authorize full configuration closure.

## Proposed Import contract

Add `ConfigurationDispositionPolicy` to the existing normalization contract.
This is an Import-facing contract with a private checked proof type, not a second
runtime configuration model. Update the current format in place and rebuild
affected packages and imports; retain no old-format reader or parallel policy
implementation. A package without a proven policy keeps the relevant obligations.
Its final Rust spelling should follow implementation review.

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

An absent policy supplies no completion authority. Publication must authenticate
the predecessor commitments before rebinding dependencies; a changed reward or
scalar policy invalidates the prior disposition proof. This is content-identity
validation, not a compatibility requirement for obsolete development formats.

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
  document. Its global fallback issue must remain. Account for the full historical
  237-link Original05 census, including the 196 former sole-link outside origins,
  through proven dispositions or retained obligations; no orphaned source, stale issue
  reference or unrelated-inventory closure may result from local completion.
- Publish with a minimal authenticated artifact delta and reproduce the package
  byte-for-byte. Test policy rebinding and verify that removing its authority
  preserves unresolved obligations rather than falsely completing an inventory.

New checks should be Rust tests. Reuse the existing optional PoB source witnesses
where their assertions suffice; extend focused source controls only where a new
semantic claim needs evidence. Membership completion and numerical parity remain
separate assertions.

The generic accounting seam realizes the
accepted separation of build choices, scenario assumptions and usage. If review
finds a control whose correct ownership conflicts with that separation or needs
a new public native capability, retain its obligation and present that specific
decision before implementation. This note does not pre-approve such changes.

The later [generated-source accounting contract](owned-generated-skill-dispositions-proposal.md)
is accepted. It updates the sole current generated-input policy in place and
runs field accounting independently of successful raw-quality/count imports.
The owner explicitly declined a compatibility branch: reimport affected builds
and regenerate evidence instead of preserving old V1 behavior beside a V2.
Current generated-policy output uses sidecar21; the CLI reports its version and
SHA256 of the exact written bytes. Declarative policy identities and draft values
remain separate from that evidence contract. The completed all-five comparison
and exact changed origins are recorded above. Preset-owned input bindings and the separately accepted participation contract
retain their existing responsibilities.
