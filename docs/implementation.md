# Implementation plan and resume point

Updated: 2026-10-07 (EDT).

This is the active delivery plan. The [design](domain-architecture.md) defines
the end state; the [execution overview](data-and-evaluation-overview.md) explains
what actually runs. Older checkpoint reports are preserved in
[implementation history](implementation-history.md), not current instructions.

## Current state

**Five supplied builds: 0/5 complete native evaluations.** All five import and
run in pinned PoB. Owned native components work, but unresolved inputs and
mechanics still prevent complete native requests. Component parity, catalog size
and deleted code are not substitutes for this gate.

The owner confirmed that **SQLite, DuckDB and an ORM are outside the plan**.
Use generated owned artifacts and immutable loaded Rust indexes. New features
belong in one native path:

| Subsystem | What belongs here |
| --- | --- |
| Owned Core | Source-independent build, occurrence, scenario and request contracts |
| Owned Data | Injected definitions, tables, typed rules, identities and compilation |
| Owned Engine | General native preparation, effects and metrics; reusable worker scratch |
| Owned Import | External build decoding, source interpretation and checked normalization |
| Offline acquisition | Convert pinned source facts to reviewed owned data artifacts |
| Optional PoB | `mlua`/LuaJIT reference execution and data acquisition |
| Legacy Import/Data/Engine | Only named remaining acquisition, inspection or numerical-reference consumers; retirement work |

`evaluate-owned` is the native CLI entry point. `evaluate` and `metrics` are
optional PoB reference commands behind `--features pob`. Native evaluation has
no PoB subprocess or Lua state. Some legacy Rust modules still compile through
Import's dependencies; full legacy distribution isolation is unfinished.

The obsolete native CLI, NativeBackend crate, class/UI experiments and orphaned
profile-template coordinator are removed. Keep useful independent source and
numerical tests. Delete unused paths together with their exclusive scaffolding;
an exported API or its self-tests alone do not justify retention. The
[retirement inventory](legacy-retirement.md) names the remaining consumers.

**Compatibility direction, 2026-10-06:** the owner does not require compatibility
with earlier development formats or behavior. Prefer updating the current
contract and rebuilding/reimporting data to carrying old and new branches.
Preserve useful source evidence, not obsolete runtime paths. Any semantic cutover
must invalidate affected result identities and caches; verify the regenerated five-build
corpus. This supersedes preservation requirements in older proposals where they
exist only for backward compatibility.

## Latest checkpoint: exact generated source-property ownership

The accepted [ownership contract](owned-generated-source-properties-proposal.md)
is implemented in the existing Core/Data/Engine path. Receiving V3 now declares
an authored occurrence or an exact generated supply, an `OwnerSkill` endpoint,
and assembly bound to `InputOwner` or `ExactSupplyingProvider`. Generated Skills
own their collected properties; the exact declaring provider retains parameter
write authority and its natural Actor, Skill or EquipmentUse context. Current
wire and authoring fragments changed in place; no compatibility path was added.

Ten Engine tests cover repeated item/tree sources, mixed manual/generated uses,
exact natural contexts, absent providers, duplicate and foreign writers,
missing final producers, dependency cycles, disabled participation, and serial/
Rayon scratch reuse. Core wire tests and Data validation suites pass, as do the
existing Engine source-property, participation and preparation-readiness suites.
An independent review found no new correctness defect. The added discovery is
bounded cold-plan work; the hot evaluator and worker scratch model are unchanged.

The five-build publication was replayed with the updated libraries in **35.61s**
at `runs/owned-generated-participation-02/package`. It reproduces the same release
identity, all drafts/sidecars and the 18 participation controls. Complete native
builds remain **0/5**, with selected unresolved issues **107/117/109/123/5**.
This contract supplies no missing game formulas or complete rule-owner inventory.

The older numerical components now use the current checked release, retaining
exact authored program/table equality and explicit finite-test boundaries.
Ice Nova's eight tests pass in **3.96s**, Offering's eight in **9.50s**, and
Sniper's six in **11.52s**. Historical publication steps were not replayed through
retired importer formats. Their provenance assertions remain distinct from the
current numerical component checks. No compatibility loader was restored.
Strict workspace/all-feature/all-target Clippy passes, as do owned-only library
Clippy, both WASM library checks, compiled-source boundary checks, default CLI
dependency checks and formatting for every changed package. No fresh full
workspace test run or fresh source VM run is claimed. CI `c3ec029` was queued
behind the still-running `47f3560` validation; its Engine/Import/Workspace shards
passed on both platforms. The last observed complete green run remains
[37552883848](https://github.com/Azaril/poe-optimizer/actions/runs/37552883848).

Local evidence: `runs/owned-generated-source-*` records the focused builds/tests,
the corrected current-release component runs (`*-04.log`), strict lint (`clippy-03`),
WASM and dependency/source-boundary checks. Initial failures were test migration
issues: removed importer formats, old stage metadata, fixture provider roles,
and changed Partial diagnostic inventories. No native authority check was relaxed
to make a fixture pass.

**Next blocker: real generated preparation data.** Sand and Firebolt still have
raw-only Partial inventories. Author Sand's ordinary preparation and shared
prepared Skill stats for its manual/generated forms, with exact provider
projection where a real consumer needs parameters. Keep provider-specific support
eligibility, final-domain validation and incoming coverage explicit. The separate
[Boolean contribution proposal](owned-boolean-contributions-proposal.md) remains
pending; the Player slot read is already accepted and implemented.

## Checked baseline and original-build results

Use `runs/owned-generated-participation-02/package` as the integration baseline. Its
`original-01` through `original-05` siblings are the checked imports. Exact Direct
normalization, provider/raw-input authority and source issue correspondences
are retained. All freshly regenerated sidecars use V21;
native schema V6 and operations V21 remain independent contracts.

Its immediate predecessor is `runs/owned-global-energy-shield-03/package`, input
`21b327c2364f5f5d9ae7c47c5c2293ac310d5e5c113e536d5bde25d93b410ff9`.
Historical endpoint paths authenticate useful stored evidence; they are not
separate production evaluator modes.

| Identity | Current value |
| --- | --- |
| Source revision | `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` |
| Release input | `03504a9f21dee158483d990ddad77d56c60559b35ac6ecef1fe89664ee75c8ae` |
| Registry | `77170e2cd8220b5fbc2bb6907c8fc506325c3abc490acd426b71a374310ffb3a` |
| Definitions content | `9d6ee09d44c1eb022b706bc51a61b28239c7f572647c25e9c15ce2f19e48ff94` |
| Rules | `9cc36e0d2e5bd48c303f7a91f9d2f2fe0f401fa38b9ed61c3c203aa6e3ab89a3` |
| Compiled rules | `bf97c711b4554d5e81b1654aba5d3a129ae5df496d299bf84b91c4a211ec744f` |
| Routing | `5b9286007c9d5b415cc6c83c22d4f4f373f2941651123f80253bb042af8d4e28` |
| Mapping | `e923bc662644414a25789c39d5b9268dba8293e3849c3f8d7b9ef6cbb836fe70` |
| Skill roles | `ef9f89fa061f0acaad37051e30d02bac80a6a562f480bb2d5d86861dde257eac` |
| Normalization | `e097b3719b7d34a314736626ef496170819e11f476f476551196f0fd3050de92` |
| Rewards | `9aaa507de9513904e76f0306f934a08c22d795f2ada2bbc4c8b28544e592c859` |
| Items | `44cb569177decf3c749521cd508df4107b5cb8bb0cfef74203dc941642905ed7` |
| Item source | `dd16e916712dcbcac94fa4da8bb7740a89d831ba14eafa6e11f4212efe6a1651` |
| Tree policy | `93e1602eb70c094a1ddce9ea3a5162c2c05593213e64f46a0eb40cf24d2fc9e7` |
| Authoring commitment | `18626f50ce65e7a93ce58548fb7cdbce7f76a0c34fd6d39550b6afbcf605bfa5` |
| Schema / operations | V6 / `owned-domain-operations-v21` |

The eighteen package files total **63,149,319 bytes**, with 132 provenance rows.
Definitions use `pob-3887ae68-player-offhand-facts-v1`; the independent rules release
remains `pob-3887ae68-sniper-population-readiness-v1`. The registry ends at `334c`.
Mechanics/integration remain Partial, with no evaluation bundle. Authoring stage,
source-property and readiness fragments are not independently complete evaluators.

| Original | Unchanged saved selection | Selected unresolved issues | Native evaluation |
| --- | --- | ---: | --- |
| 01 | Kelari / Sand Djinn, Kelari's Deception | 107 | Not run: Pending |
| 02 | Twister, skill set 6 | 117 | Not run: Pending |
| 03 | Whirling Assault, average-damage mode | 109 | Not run: Pending |
| 04 | Crossbow Shot | 123 | Not run: Pending |
| 05 | Skeletal Sniper, Basic Attack, skill set 4 | 5 | Not run: Pending |

Original05 retains support-origin discovery `01de`, preset usage `0503`,
configuration `01f2`, external assumptions `0207`, and scenario usage `0208`.
Resolving these input obligations will not by itself complete numerical owners.
Crossbow's saved reference selection has no hit-damage output; preserve its
availability result rather than choosing another skill. Original source bytes
and all 110 query identities remain unchanged. Local `runs/` files are
reproducible evidence, not distributed game data.

## Next executable work

**Immediate blocker: real generated preparation data.** The accepted ownership
contract now supports exact generated sources. Tree-generated Sand and
item-generated Firebolt still lack their authored preparation and assembly
rules. Their Partial two-slot raw-input lists must not be closed as final
inventories. Real source contrasts include
Sand raw level 1 becoming 3 in Original05 and 12 in Original01, and Original04
Firebolt raw 17/0 becoming prepared 26/21. These are evidence of missing assembly,
not universally applicable formulas.

The [generated-source contract](owned-generated-source-properties-proposal.md)
was **accepted and implemented on 2026-10-07**. Cold discovery, exact assembly
authority and dependency checks are covered by the native contract tests. Use
these existing bindings for real producers and consumers before readiness; do
not add fake authored roots, a second raw-input store, self-parameter writes or
a raw-as-final shortcut. Validate actual source contrasts, provider-specific
support eligibility, inactive loadouts and serial/Rayon consistency.

**Real authoring after the contract.** The actual Sand owner `0322` currently
contains only Command/Actor supply; Firebolt `0134` has no rule owner or output
members. Neither has a final-parameter consumer to preserve. Prefer common
prepared-level/quality Skill stats for manual and generated uses, using existing
InputOwner authority; use exact-provider projection only where a real consumer
requires final parameters. Reuse pre-support channels `30ac/30ad`, Player Minion
level channel `30ab`, actual `30ca` direct/copy programs and snapshot reducer
`32e4` where their proven applicability fits. Do not copy physical Gem readers,
corruption assumptions or incomplete contributor closure into these forms.

**First authoring gate: capture separated incoming property evidence.** Existing
`djinn-support-preparation-source-02` reports show manual Sand 20→31 and tree
Sand 1→12 in Original01, and manual 20→22 / tree 1→3 in Original05. Both source
forms have PoB catalog Gem metadata even though they are not physical native
Gem instances. Catalog metadata must stay acquisition data; do not manufacture
native physical Gems to reproduce it. The reports establish final values and
metadata flags, but do not enumerate ordinary `GemProperty` separately from
once-per-source `SupportedGemProperty`. Capture both inventories and before/after
values, including zero-support generated Sand, manual support removal and
independent provider changes, before asserting complete channel `30ab` coverage
or authoring a closed raw-plus-bonus formula. Endpoint differences alone are
insufficient. Reuse retained controls rather than repeating unrelated acquisition.

Retained `owned-djinn-raw-inputs-01` evidence already distinguishes raw-domain
validation from final-domain validation: manual Sand raw 0/−1, 41 and fractional
levels trigger source editor recovery before property addition. Initially admit
the proved integral raw range 1–40 through data-defined guards, then independently
check final level after ordinary/supported additions. An invalid raw value must
not become supported merely because bonuses bring its final value into range.
Keep unproved recovery and quality domains unresolved rather than copying Lua
fallback behavior. These Sand constraints do not define Firebolt's grant domain.

The pinned source clamps raw level plus corruption to at least one only when
corruption is present, then adds ordinary properties. Firebolt's admitted raw
grant domain includes zero: an unconditional physical-Gem clamp would turn
raw zero plus one ordinary level into two. Preserve the ordering and explicit
final-domain validation; no Lua recovery fallback is authorized. Source quality
controls retain fractional `12.5`, so do not silently choose integer quality.
Retained Sand observations show skill/Command levels 12 and 3 with actual minion
levels 24 and 6. Its `levelRequirement`/stat-set `actorLevel` are different values;
author the actual minion-level table/branch as injected data instead of treating
those source metadata fields as minion level or inferring a two-times formula.
All contributor inventories and full numerical owners remain separate gates.
Sand's `noSupports` belongs to the generated provider, not its whole Skill
definition: manual Sand has real support assignments. An empty generated support
census also does not eliminate actor-provided supported properties. Use explicit
source applicability and relation inventories; the existing ten Djinn type/
admission definitions do not provide complete support origins, effective support
inputs or numerical delivery. Firebolt requires its own Spell/Fire/quality
coverage; the Minion channel must not be applied to it.

The source census covers Sand's summon and declared Command, which share one
source occurrence. The three actor-child skills have no source occurrence in
this source-property path and start at level 1 / quality 0; they must not each
add another source-property contribution. Their separate support admission and
delivery still require coverage.

Concrete authoring map: existing support count `32e0` and supported-level channel
`32e1` complement pre-support `30ac/30ad`; Sand must not read Spell channel
`32e2`. Bind tree supply `32d2`, Command supply `32a5`, Actor population slot
`32a3` and actor-level stat `001c` through their actual declarations. The existing
`sniper.actor-level` table has the applicable 1–40 → 2–80 values; reuse checked
table data, not Sniper's unrelated quality/damage formula. Preserve source
population override/clamp evidence before claiming the full actor-level domain.
Sand catalog tags include Minion/Persistent/Command/Physical, but not Spell;
the acquisition witness must record tags and requirements alongside the actual
applied ordinary property rows. After that proof, use Skill-owned ordinary
preparation, source census, shared final stats and declared Command/Actor
projections in that order. No further public model change is currently indicated.

Saved participation transport now uses existing `332b` independently. Manual
Sand needs its own binding before any Skill-definition-wide gate is admitted.
Generated bindings retain `WhenExactSourceSelected`; Original04's unresolved
Firebolt provider and archived cross-axis joins stay unresolved. Reuse the
[generated evidence packet](../data/owned/poe2/3887ae68/generated-participation/README.md)
and retained source controls. A selected PoB group can still preview a disabled
generated effect; this supplies no native force-enable authority. Global switches,
FullDPS/reporting intent and the other selected issues remain separate; do not
close all usage `0503` on the strength of one enabled Boolean.

The next input evidence work should classify the retained
`owned-extra-stat-consumption-source-05` suppliers and filters, which already
include inherited-node and Amulet-copy transport. Do not repeat that acquisition
or treat a new scalar grammar as progress on these selected obligations.

**Deferred coherent item-layout work.** Original04's selected Gold Amulet needs
Paragon's granted passive, remaining Rarity accounting and all-skills quality;
its global ES, Life and Spirit category inputs stay Pending. Original05's
selected Solar Amulet has no anoint and already has Proven layout. Dominion is
on its dormant Stellar Amulet, so completing it will not solve the current five
selected issues. Paragon (`20686`/owned `0d5e`) and Dominion (`26214`/`0ef4`) are
anoint-only disconnected nodes, not ascendancy nodes. Current
`GrantTarget::AllocationAccess { pools }` authorizes a pool without an exact
node, and Engine rejects granted allocations/access providers as
`UnsupportedRelation`. A future anoint checkpoint must review exact target
permission, provider lifetime, deduplication and point accounting before
implementation; it is not a data-only grammar extension. Paragon's +5% skill
quality and the separate +5% item line are distinct contributions.

**Accepted generality checkpoint (2026-10-04).** The [review and gates](build-generality-review.md)
record concrete limits in generated-source ownership, application stacking,
physical/reference import coupling, coverage scope, normalization compilation,
owned objective binding and legacy isolation. The initial read-only review is
complete; the cross-family, integration, holdout and CI gates remain open.
The generated raw-input publication passes for its reviewed families. Prioritize
remaining preset/scenario usage and actual contributor coverage next. Reuse the
checked exact-provider join rather than adding another source matcher. Before
changing a public contract, use contrasting real cases and record
its data-only extension boundary. Preserve the five original requests and their
110 query rows at every publication. Neither this review nor producer coverage
changes the 0/5 complete-build result.

**Numerical resume:** the Strength-to-Life receiver, Count cutover, six ordinary
attribute consumers and guarded empty-MORE producers are implemented and tested.
Use the explicit query producers instead of fixture factor literals. V21 rejects
an unmatched potential effect as `PlanError::Invalid`, even if inactive; retain
this conservative admitted domain. The six current INC memberships are now
Complete. BASE/global inventories, Class coverage and original builds remain open.

Next establish real BASE contributor closure and semantic ordering, including
all current item donors and inactive alternatives. Reuse the importer's explicit
modifier-order proof; raw saved line order is not source modifier-list order.
The executable BASE audit now proves a declared-domain signed-cancellation
counterexample and real post-passive bonus copies. Resolve destination-complete
ordering and grouped bonus projections before promoting those memberships;
the class/item-prefix plus passive-suffix shortcut is insufficient. Nonempty
MORE grouping remains unresolved; there is no native cache-dependent mode.
Comparison snapshots and stage-dependent donors remain on the accepted finite
stage plan. Resolve inherent-bonus controls before connecting final Player Life.

**Class coverage follow-up:** Original05's Class `0a23` still has its inherited
`tree-game-rules-not-converted` marker. Its Count bases and intrinsic attack row remain class-owned; intrinsic Life
now belongs to shared Actor `332a`. The shared initialization inventory stays
Partial. The complete residual class-specific inventory and all separate gates
must be reviewed before the Class marker changes. Shared action selection, resources and conditions retain their
own coverage. The shared implicit root `1790` is now complete for its default
intrinsic inventory; do not reopen its proven empty lists or use that result to
close Class behavior, neighboring modifiers or external transformations.

The owner accepted [existing-actor rule ownership](owned-existing-actor-rule-ownership-proposal.md)
on 2026-10-06. Reuse Actor definitions with explicit applicability to the existing
Player, then move shared intrinsic Life out of all eight Class owners. Keep the
unimplemented shared initialization inventory Partial. This supplies honest
ownership without duplicating universal rules across classes or manufacturing
a Player grant. Contract implementation, the first data migration and source/native
validation pass; no additional whole-build closure is claimed.

**Class audit result:** [closure is withheld](owned-class-coverage-audit.md).
All eight residual class program inventories and the real separate action gates
were reviewed. The actual loaded class-table branch and all eight rows are now
authenticated, and the accepted slot relation has published structural off-hand producers.
Before a data-only closure, give the remaining effective equipment-derived Actor
conditions explicit producer ownership and source evidence. Reuse existing equipment delivery
and Actor rules through the [Player slot relation](owned-equipment-slot-state-proposal.md); do not
substitute Action query selection or template identity for hand state. Keep all
eight Class markers and shared Actor `332a` Partial. The audit records exact
source readers, current consumers and the smallest proof/control work.

The six Add query inventories still require actual membership and ordering
proof; the six Increase inventories are Complete for the current rule package.
Inherent-control Stats3315–3319 currently have no producers; review the proposed
[typed Boolean aggregation contract](owned-boolean-contributions-proposal.md)
before implementing multiple-source flag producers. Do not add five false
defaults. No final Player Life recipe exists.
These are distinct production blockers, not further finite-fixture exercises.

**Input resume:** keep Original05's five selected issues and 73 origins linked
to configuration issue `01f2` open. Its 101 item-range records already have exact
item/output or unresolved-item ownership; the remaining origins are 35 Config
and 38 outside Config, including 38 outside origins with only this issue.
The [generated-source consumer census](owned-configuration-dispositions-proposal.md#generated-group-consumer-census-next-blocker-2026-10-05)
must distinguish execution/reporting intent from PoB reference selectors for
the remaining Skill/Gem origins. The current accounting proof has retired six
selected generated rows from configuration fallback without closing usage. Reuse the shared resolver and field-level proofs;
selected-context evidence cannot close dormant preset rows.

The declared-stat/global-switch census and actual-parser SkillName controls
already pass. The [item SkillId control](owned-configuration-dispositions-proposal.md#item-granted-skillid-control-2026-10-05)
now adds exact item/grant/receiver transport, mapped output, supplier-line removal
and strict repeat/JIT comparison with retained raw diagnostics. It supplies no
native closure authority. The [actual consumer witness](owned-configuration-dispositions-proposal.md#actual-extra-stat-consumer-witness-2026-10-05)
now captures the extra-stat argument at `CalcActiveSkill.lua:795`, bound to unchanged
Original05's source bytes, selected axes and contemporaneous ancestry. It preserves
nine equipped uses over eight items, attribute-override projections, support/skill
records and Config input/default evidence. Source04 corrects inherited passive modifiers and
adds actual original node-builder returns; use that effective view rather than
source03's local-only node lists. Source05 additionally proves exact Item23 Amulet ScaleAddMod/AddMod transport,
including both retained zero-valued records and their receiver/slot identity.
Next classify the captured supplier/filter dispositions and their transport to
the actual consumer before authoring field exclusions. Complete NodeModifier and Amulet-copy laws,
empty-source exclusions and dormant presets are not proved by the selected frame.
Preserve unknown nested records and transforms. Post-load absence is insufficient;
an exact mandatory filter must survive transport before it can justify an exclusion.
Keep this finite original-context proof separate from arbitrary candidate coverage.

The Magnified Area I/II contributor slice and its actual Action eligibility
producer now pass source, publication and native checks for the reviewed fourteen
contexts. Use the published producer rather than fixture-supplied Area facts in
new integration work. Primary Summon delivery, final radius/resource-cost/damage
consumers and complete receiving remain open. Continue that native integration
alongside the bounded Original05 input proof and participation coverage
for remaining skill families.
Do not expand generic source-proof infrastructure as a substitute for native progress.
Rapid Casting I/II is now published on physical Ice Nova's output `3280`
across both stat sets, with unchanged prepared-input prefixes, cast-specific
Increase channel `32fc`, exact physical support controls and parallel replay.
Spell membership is not the source's Cast predicate; no general timing classifier
or final cast rate is supplied. Neither tier authors a cost/reservation entry.

Encroaching Ground Gem `0740`/Skill `03f8` now publishes factor 1.1 through
ordinary-cost channel `32fa`, with exact source and native composition controls.
Its ground growth remains unresolved. The source's 1.43 support subtotal and the
native unrounded Product are explicitly different contracts; do not hide this
rounding question behind tolerance or generic source emulation.

**Validated input work:** the [generated-source accounting contract](owned-generated-skill-dispositions-proposal.md)
uses the existing Import policy and one current implementation. Its private
proof accounts for admitted fields against the exact provider and emitted
raw/count bindings while retaining existing Pending usage. All 287 normalization,
184 library, 16 Direct-disposition and eight Direct-source tests pass. The CLI
reimport of all five unchanged originals passes: four origins change in Original01,
six in Original05, and none in the other three. Draft values, IDs, allocations,
selections and other proof dependencies remain exact. Sidecar21 and exact CLI
proof hashes identify the rebuilt evidence. No old importer branch is retained.
Unknown fields keep configuration fallback without discarding successful values.
Evidence: `runs/owned-generated-field-accounting-01/validation.json`.

Numerical integration should use the published real Offering output/recipient
topology and complete actual incoming contributor and selected-owner coverage
below. Settle ordinary-cost reduction and rounding with contrasting valid-domain
inputs. Do not invent a parent Action or use source absence to close ground
growth. The native participation consumer and first Sniper input/rule packet
are implemented; other skills retain their explicit inventory gates. Other new
recipient contracts require review.

Selected Pain Offering uses Prolonged Duration **I**, while II occurs in saved
alternatives. Its duration/cost factors now use real player output `32fd`;
final duration/cost consumers and complete contributor inventories remain open. Final OrdinaryTiming requires
all eight explicit inputs, branch/rounding authority and complete contributors.
These are broader integration gates, not defaults supplied by Rapid's channel.

The Offering final-input source, publication and seven-test native component
now pass, together with the public Engine, historical native, authoring,
strict workspace Clippy, formatting and WASM compilation checks recorded above.
The real item programs,
canonical rolls, shared source census and final projections `3223/3224` execute
in one plan; do not restore final22/0 literals or pass intermediate values through
a second evaluator. Stages V4's bounded local facts preserve production ownership
and dependency checks.

The [Amulet snapshot reducer](../data/owned/poe2/3887ae68/amulet-bonus-snapshot/README.md)
is now published and integrated. The next numerical blocker is **actual selected
item/source contributor and owner coverage**, not another producer for 32e4.
Keep using the real reducer, existing copy program and explicit stage boundary.
Establish ordinary item, external and supported-property inventories before
replacing finite test closure with actual release authority. The global
selected-owner gate still refuses scalar/reduction reads if another selected
owner is Partial; a measured empty source bucket cannot override that gate.

Selected Modifier `30ca` now has the ordinary Talisman recipient guard. Prioritize
the remaining branch/applicability certificate and actual template closure. Focus preprocessing at `CalcSetup.lua:1473–1484` now has a passing original-call witness. The late non-Amulet copy witness proves
bounded delivery after preparation; Talisman's dedicated witness proves ordinary
Amulet diversion separately from early copying. Use these consumer observations;
final ModDB presence alone does not establish an earlier level bonus. Completing
a real selected owner is more useful now than
adding an absent jewellery-bonus family. Final Offering duration, payable cost
and recipient scaling remain subsequent consumers with their own original-call,
incoming-coverage and rounding evidence. Any new public contract remains a
reviewed design decision.

Historical `3225/3226` descriptors now have no executable consumers or writers.
The published structural primary-supply activation remains exact; post-census
source assembly projects final parameters directly from computed `30ac/30ad`.
Preserve historical receipt replay and allocated IDs without restoring that
redundant intermediate path. The proved level domain is exact integral lookup
1–40; source quality input preservation supplies no additional quality-effect
recipe or supported-quality producer.

Classify `validateGemLevel` recovery separately under the Lua cleanup gate.
Original05's integral `20 + 0 + 1 + 1 + 0 = 22` path does not justify natural-20
fallback, arbitrary table-entry selection or treating table extent as a game cap.
Use supported integral lookup and explicit coverage refusal until broader valid
candidate semantics are established. Fractional controls test reachability;
they do not establish game legality merely because the wire type represents them.

Do not route Magnified's ordinary-cost factor `32fa` into Spirit reservation
input `326e`: their source fields and consumers are distinct. Current support
receivers are Actors or Actions; `ReceivingSkill` names an admission context,
not a third receiver kind. Do not synthesize primary Summon Actions, invoke a
parent contribution once per child Action, or repurpose source-property
preparation as execution authority. A genuine parent demand/recipient extension
needs contrasting real cases and review before changing the public contract.
The valid-but-unreviewed part/stat-set control now passes: the existing guard
produces Inactive and the demanding contribution reports UpstreamUnavailable,
without a Boolean default. MissingProducer remains the separate absent-program
case. Future receiving/selection expansion still needs actual producer evidence;
these test-only declarations do not extend the published game domain.

Do not reuse Frost switches as general activation. Existing usage programs inherit
activation gates, so activation writing also needs a dependency-order review.
The [participation proposal](owned-skill-participation-proposal.md) now makes that
decision concrete: retain mechanical supply, derive requested participation from
existing typed usage, and declare its execution consumers on the same graph.
The native consumer and first Sniper input/rule packet are validated; other
families still need their own complete input and source authority.
It also records PoB's selected-group enabled/loadout bypass and default-skill
fallback. Preserve exact reference identities and prove selected/nonselected
controls; native query selection must not silently activate a skill.
The [bounded generated-source accounting follow-up](owned-configuration-dispositions-proposal.md#bounded-generated-source-accounting-follow-up-2026-10-05)
is implemented for the admitted generated Skill/Gem pairs. It proves every field
against the exact provider, actual emitted quality/count bindings, reference
correspondence and existing same-preset Pending usage. Six Original05 rows now
carry the existing usage obligation instead of configuration fallback; 73 origins
and all five selected issues remain. Reuse the same proof seams for further
coverage and retain fallback if any field or reference remains unaccounted;
source05 cannot certify a universal absence of extra stats. No new public
participation contract or global-switch non-applicability follows from this plan.

Dormant set 2 currently has Complete empty usage,
so it cannot receive an invented Pending link. Known quality or count alone does
not establish enabled/global/reporting/action semantics. Handle mixed containers through their
actual consumers under the [proposal's](owned-configuration-dispositions-proposal.md)
private correspondence and global retirement rules. Preserve every live selected
obligation until its actual inventory/consumer proof passes.

The breadth replay also identifies five Original02 overlays with Proven source
line targets but Partial modifier-roll declarations (Fire, Lightning and Evasion).
They deliberately keep fallback; source layout alone is not complete output
authority. A later ownership refinement must bind their actual pending roll
obligations, or supply the missing declaration proof, without weakening the
known-output gate. Overwritten/out-of-bounds and nonmodifier-emission range
ownership also remain explicitly unimplemented. These are Import coverage
limits, not separate native item systems.

Local Config coverage is still needed over the full pinned catalogue, not just
Original05's eight authored reward Inputs. PoB constructs 28 non-reward defaults;
seven hidden child-effect enables emit actual modifiers. Reuse reward, encounter,
level and scalar proofs with exact selector/output correspondence. A partially
successful scalar token is not proof for every channel. The full configuration
policy remains proposed; the implemented integrity gate only rejects dangling or
missing issue correspondence. Do not move every unknown to assumptions or usage
to manufacture closure. Preserve all five remaining selected obligations until their
actual consumer/inventory proofs pass.

Then return to generated activation, global switches and reporting inclusion.
Requested counts already share exact provider joins and typed decoding. The
original-function [Full DPS witness](owned-full-dps-aggregation-proposal.md)
authenticates contributor and display-row identity; native aggregation and its
tie/actor/action contract remain separate work. Selected-action `TotalDPS` keeps
its meaning. No fake SkillUse, synthetic union build, default quality/count or
implicit retargeting may close an input obligation. At each publication rerun
all five unchanged selections and fix the next measured blocker.

The [second retirement audit](legacy-retirement.md#second-pass-audit-resistance-terminology-and-end-state-ownership-2026-10-05)
distinguishes old application adapters from shared resistance arithmetic and
older owned modifier representations. Fixed Elemental still has a live early
conversion rule; migrate that semantic path before deletion. Genuinely orphaned
Import equipment/Mace parsing is now removed and its retained library tests pass. Preserve shared
source loading, formatter laws and useful numerical/reference tests.

**Lua cleanup gate (requested 2026-10-05).** Run the
[behavior/type inventory and classification](legacy-retirement.md#lua-compatibility-cleanup-gate-requested-2026-10-05)
alongside D0/D5 and the original-build integration work. Prioritize source
truthiness/defaults, numeric formatting, rounding/min/max, ordering/aliasing and
cache lifecycle in shared helpers. Retain only justified domain laws; isolate
external format conversion and optional reference behavior. Each proposed removal
needs its valid-input impact and evidence recorded. Do not reproduce a Lua quirk
merely because a compatibility test asserts it, and do not silently change
numerical outputs when game intent remains unresolved.

1. **Complete generated usage and remaining source accounting.** Original05
   now has five selected obligations. Eleven selected preferences already import:
   four skeletal counts, Offering and Frost Bomb switches, and five Direct/Tree/Item
   occurrence counts. Remaining activation/global/reporting usage needs the
   [generated applicability contract](owned-generated-skill-usage-proposal.md).
   The versioned Core contract, all-stored-row proofs, V6 permissions and V19
   input producers pass focused validation. Use that checked boundary;
   do not relax the legacy field's supplying-preset scope. Preserve unknown
   source fields and all existing query identities.
   The reviewed Tree Djinn and item-granted Firebolt now have exact selected
   saved-input/provider correspondence. A source tree-node ID is not a project occurrence: independent
   allocation presets can contain distinct allocations of that same definition.
   Prove the saved selection's exact provider join before projecting preferences;
   never choose the first match, fan out to every allocation or silently retarget
   archived combinations. Generated quality12.5 survives source reconstruction; a
   provider-wide zero is invalid. On 2026-10-05 the owner accepted
   [preset-owned exact generated input bindings](owned-generated-skill-inputs-proposal.md):
   raw quality uses shared typed slots and explicit producer permission, separately
   from usage. Tree Djinn and item Firebolt raw levels already have provider
   projections; preserve those writers instead of adding redundant level rules.
   Generated usage alone cannot fill the quality it requires to execute.
   The source join investigation confirms that Original05's `Tree:13289`
   occurs in Specs 2–6 and `Tree:32705` in Specs 3–6; SkillSets contain no
   implicit Spec association. Its active axes are `Tree.activeSpec=3`
   (position), `Skills.activeSkillSet=4` (key) and `Items.activeItemSet=2`
   (key). Bind the saved active SkillSet only through those independently
   verified axes. Archived combinations need explicit source correspondences
   or remain Pending; titles and coincident set IDs confer no relationship.
   Firebolt's Item28 is equipped in ItemSet2/Weapon1; its provider must join the
   actual EquipmentUse and unique granting Modifier, including source slot and
   item identity. The shared importer now performs these joins after real records
   exist and rejects stale, ambiguous or missing correspondence. Reuse this
   validated boundary when adding usage dispositions.

   The published slice authorizes only quality on tree supplies `32d2/32d4`
   and shared slots `3262/3264`, plus Firebolt Skill`0134`/supply`31c9`/quality`32ef`.
   Level`31ca` remains provider-produced. Opt-in normalization explicitly converts
   existing usage rows to the versioned envelope while preserving unresolved
   completion/issue IDs. It does not close the independent usage inventory.
   Requested primary counts now resolve through UsageV3 for the reviewed Direct,
   Tree and Item occurrences; only the source-proved generated `nil` spelling
   maps to one. Count/group transport does not complete enabled/global/Full DPS
   semantics. Account for those settings across authored and generated contexts
   without unreviewed defaults. The configuration-role issue retained 79 origins
   after presentation/range accounting (237 in the original census); the current
   generated-field proof reduces that to 73, including
   global fallback records outside Config. Their complete
   disposition census and actual consumers must be proved; additional numeric
   controls alone cannot certify an empty scenario-usage inventory.

2. **Connect selected receiving and property contributors.** The two single-line
   Minion Damage nodes, four Life/Damage nodes, two plain Life nodes and twelve flat/permanent reward
   owners now have complete default producer behavior. The Command slice also
   closes six selected passive owners and exercises Player-to-Actor-to-Action
   semantics using existing contracts. The Fire/Lightning
   input channels `32e6/32e7` are separate from final resistance metrics; completed
   reward producers do not supply their final reducers or contributor closure.

   The additional selected Reward owners AilmentThreshold `0036`,
   FlaskLifeRecovery `003b` and Charm `002d` now have complete producer programs
   through channels `32f0..32f3`. Both Charm effects remain together, with exact
   option/value provenance and Reward roots. Their receiving, final formulas
   and other incoming contributors remain open; producer closure does not make
   whole-plan reductions available. Growing Swarm now closes its complete default
   passive owner with distinct unconditional cooldown and Area inputs. Resume
   remaining selected passive/item/support owner gaps next.
   Component sums never authorize full pools, absent contributors or defaults.

   **Published native component: Magnified Area I/II.** The selected manual
   Sand and Ice Nova supports are Gem`082a`/Skill`047f` and
   Gem`082b`/Skill`0480`. The authored packet converts both Area increase (35/45)
   and support resource-cost multiplier (1.3); area alone is not the complete
   producer. The source's mana-oriented name does not limit its resource consumer.
   The pinned
   `sup_int.lua:6684/6716` families differ in the zero damage key: I has no map,
   while II maps an Area-conditioned zero MORE modifier. Preserve that distinction
   without inventing an I-tier producer. The packet reuses ordinary support
   preparation and receiving programs, with Action channels `32f9/32fa` and
   Boolean eligibility input `32fb`; `32f7` remains Actor-scoped. The shared Bidding
   fixture now proves exact receiving, removal, duplicate-family selection and
   occurrence isolation, including physical Ice Nova and fourteen source-observed
   action/stat-set contexts. Keep owner coverage Partial until every actual producer/receiver
   inventory is proven; final radius and cost formulas are separate gates.
   The actual Area eligibility producer is now published for eleven outputs and
   fourteen stat sets, using the reviewed base/support/part/map writer census.
   Use those programs instead of fixture facts; other domains need their own
   producer evidence. Primary Summons have no owned Action
   output in this fragment; their delivery and reservation consumers remain open.
   This uses existing public contracts. Defer Muster's participating-minion
   population and Frost Nexus's ground-effect semantics until their dependencies
   have deliberate ownership.

   The published Command receiving slice uses existing APIs: six conditional
   cooldown passives -> Player carrier -> exact Sniper actor slot
   `Skill0012/ActorSlot001f` -> guarded Basic/Gas Action rules. Its subtotal is
   Gas 72 / Basic 0. The independent source04 witness observes the full seven-source
   inventory 92 / 20; Growing Swarm now contributes the separate unconditional 20
   through channels `32f4/32f5` into existing Action channel `32ea`, retaining the
   conditional path. Area20 reaches the Actor input through `32f6/32f7`.
   Its default declaration inventories close, while both actual Action owners
   remain Partial. Basic's 20 is a diagnostic store query,
   not an observed original cooldown call. Do not compare the conditional native
   subtotal to the unfiltered source total or publish a full cooldown formula.

   Gas output `0025` now uses its three authenticated alternatives (Impact,
   Poison Cloud, Explosion), replacing placeholder `0009` through the checked
   migration. No extra Command root or Direct Skill is introduced, and no existing
   query targets Gas. The seven IDs are `32e8..32ee`. Exact Actor receivers and
   ordinary guarded Action rules implement the path; effect applications do not
   target Actions. Existing Gas/Basic Partial mechanics, parent final-input
   requirements and whole-plan completeness gates remain. Audit:
   `runs/owned-command-cooldown-native-audit-01.md`. Existing source04 and Sniper
   actor-action receipts cover these facts; no new runtime model is proposed.

   The ordinary Minion-level Amulet copy and finite placement facts have authored
   fragments.
   Source02 now extends the historical snapshot census to all five originals
   and actual Ritualist controls. The native reducer is published and joins
   item arithmetic to source preparation in the finite component; actual
   contributor and selected-owner coverage still gate real-build authority.
   The current compiler uses a whole-selected-plan completeness flag for scalar
   Stat reads, contribution reductions and modifier transforms. Any selected
   owner/topology gap makes these reads `IncompleteContributors`; completing the
   Amulet contributors alone cannot unlock the real Sniper request. Audit and
   close actual selected owner behavior alongside input work. Do not remove a
   Partial label merely because one of its programs now exists.
   Audit the selected Sniper request first: its two Minion item occurrences are
   already canonical. Neither absence of explicit Mystic Attunement nor an
   observed zero proves all implicit, item, configuration or earlier-copy paths
   absent. Earlier Kalandra/Quiver copies can affect the general snapshot. The
   real 50% source control is one physical rings-and-amulets modifier line
   emitting four stat effects (Ring1/2/3 and Amulet). Existing source-line
   admission can represent that without a new parser or four physical records.
   Implement its full owned meaning and source/numeric admission before claiming
   a complete factor item. Keep its own copy out of the pre-Amulet aggregate.
   Generic non-Amulet slot copies run later in `CalcPerform.lua:1490–1539`;
   its dynamically named query includes Ring bonuses. That later phase groups
   numeric BASE/INC records before scaling and is not the Amulet per-record
   algorithm. It must not be included in the earlier snapshot census.
   The follow-up source audit separates three routing questions for Modifier
   `30ca`. Focus's `CalcSetup.lua:1473–1484` branch retains the original LIST
   property but drops its scaled duplicate through `MergeMod(..., true)`;
   static inspection therefore does not justify another native level multiplier.
   The later `CalcPerform.lua:1490–1539` copies follow ordinary GemProperty
   consumption. The passing late-slot Source02 witness proves actual copy
   transport without re-preparation in its bounded controls; final ModDB
   records alone do not prove pre-support level contributions. Ring controls
   are diagnostic only because Ring templates are outside this family's
   six-template admission domain. They do not prove the Focus branch or
   establish intended game behavior for every possible slot-copy mechanic.

   **Published routing correction:** the original Amulet/Talisman exclusion now
   guards direct delivery through `3305/3306`, independently of `32e3/32e4`.
   Crown/Amulet, allocation removal, independent copies, missing facts and actual
   Partial-owner controls pass in one native graph. Generic later granted
   Keystones have no unproved early authority. Keep the minion consumer, intended
   copy semantics and remaining routing/placement inventories separate.

   **Source routing findings:** the Focus witness now passes for the actual
   Sacred Focus and Instruments of Power control. Its original +2 property reaches
   preparation unchanged; the scaled duplicate is rejected. Preserve this bounded
   evidence rather than implementing another level multiplier. The name-only
   Kalandra discrepancy has a concrete native-admission and source-execution
   counterexample, with an accepted reference-defect disposition limited to
   synthetic title controls. It is not evidence of an obtainable corrupted unique.

   The current six-template modifier membership is not complete equipment
   admission: only Crown/Solar are in the equipment-membership policy. Four
   Helmet/Amulet placements are Complete; Sceptre/Focus placements and all six
   skill-grant inventories remain Partial. Energy Blade's weapon-data fallback
   retains these non-weapon Sceptre/Focus lists; exact base suffixes cannot match
   Iron Mass, whose branch also keeps Player delivery. The existing constructed
   skill catalogue executes original data loading and contains 1,436 Skills and
   966 Gems, with no UniqueAnimateWeapon or Energy Blade. Use that authenticated
   catalogue, but include cached ExtraSkill producers before certifying Dervish
   absence. Do not replace producer coverage with a failed text search or a
   native presentation-name rule. Full fresh item construction and actual
   other-hand placement predicates remain independent proof obligations.

   **Published minion Life components:** intrinsic base and passive Increase now use
   canonical Life `311a`; Gigantic contributes its Multiply to the same stat.
   Historical `330a` has no live references, while Damage uses `330b`. The exact
   injected 100-row allied table and 0.55 profile scale produce base1615 at Actor
   level44, and original delivery validates the six-passive Increase44 stream.
   Native integration uses actual physical final-input assembly and population
   programs. Exact dense tables remain valid during the lossless segment study;
   hostile/replacement profiles still need separate admission and data.

   Player intrinsic Life is now published independently for all eight Classes.
   Original05's saved Life query targets Player, so final Player attributes and
   inherent bonuses take priority over a minion-only final total. The attribute
   grouping structure is accepted; its numerical grouping policy remains open.

   Complete Life must account for extra/total contributions, Increase, More,
   conversion, overrides, relevant branches, rounding/minimum and full ownership.
   Observed zeros alone do not establish universal absence. Solar's seven intrinsic
   declaration inventories are now Complete, while numerical programs and modifier
   inventories remain Partial. The shared local Armour/ES composition
   is now published for Crown and Leggings, before per-level/override stages.
   Prioritize actual selected-item declarations and remaining Solar/Modifier30ca
   numerical ownership next. Crown/Leggings socket layouts require the accepted
   configuration model; the nineteen defence families are absent from these
   selected items, as detailed in the latest
   checkpoint and the item-ownership retirement audit. Physical
   final-input programs exist; unresolved selected providers and incoming coverage
   still prevent a real complete request. Query-independent resource demand is
   a separate pending design decision.

   Before final damage aggregation, settle the concrete precision/grouping
   question: Source02's Gigantic plus quality-one factors yield 1.21, while
   unrounded multiplication is 1.212. Individual contributions and native
   Product arithmetic are separate evidence. The [Lua/numerical cleanup gate](legacy-retirement.md#lua-compatibility-cleanup-gate-requested-2026-10-05)
   requires a valid-domain rounding law rather than source-store ancestry or
   a tolerance that masks the difference. No new PoB-defect exception is approved.

   A targeted 2026-10-06 lookup of [PoE2DB's Gigantic entry](https://poe2db.tw/us/Gigantic)
   corroborates the two 20% More benefits and also lists increased size of 20%.
   This is supplemental, unpinned game-description evidence, not a bulk import
   or proof of numerical grouping. Keep size/geometry applicability as a separate
   breadth obligation; do not infer melee range or area from model size. The
   Life/Damage packet does not claim to implement every effect of the status.

   Reservation input `3271` remains Action-owned and unresolved. Parent Action
   discovery follows query/usage/choice/support demand; no Action StatReceiver
   target exists. Do not fabricate a query or support to reach that consumer.
   Keep the -25 contributor available while the ownership decision below is
   pending; migrate or deliver it only under the reviewed resource contract.

   The [resource-obligation proposal](owned-resource-obligations-proposal.md)
   recommends exact Skill-owned sustained reservation with explicit Action
   dependencies where a mechanic needs a selected part/mode/stat set. It also
   identifies the missing Skill support-recipient contract. This is pending
   owner review, not an implemented migration or authorization to manufacture
   Action demand. After approval, validate contrasting reservation/cost shapes,
   migrate the existing live consumer, and prove query-independent results.

   PoB applies the derived Gigantic Life/Damage bonuses only in its full combat
   calculation scope. Existing buffed/unbuffed diagnostic frames can retain the
   flag without those derived records. This does not prove a game condition of
   being recently in combat, and must not become a native PoB display-mode input.
   Bind numerical comparisons to the appropriate source scope, preserving the
   diagnostic differences and deliberate native scenario semantics.
   Original05's actual MAIN and saved CALCS both use the full combat calculation
   scope. The concrete audit and source locations are preserved in
   `runs/owned-gigantic-native-contract-audit-01.md`.

   Keep the nine parser-limited defaults explicit: seven Puppet/Archon owners and
   two Djinn unlocks. Djinn child-action presence
   needs selected/unselected unlock controls; rejected Puppet/Archon text cannot
   become empty Complete native owners. At your Command and Muster should share
   a real distinct participating-type census. Jewel Socket 7960 / owned `1b48`
   still owns Socket `320e`; lack of a selected jewel does not erase that mechanic.
   A final minion Life pool also needs base-life and incoming coverage, not the
   finite 44-point producer subtotal. Complete builds remain 0/5.

   Keep ordinary `GemProperty` before support admission and actor-provided
   `SupportedGemProperty` after census. Named unkeyed supported records retain
   their value even at zero copy scale, unlike keyed level records. Spell/Nova/
   Fire item families and the actual Original04 Rune source need separate proofs.
   No unchanged original contains the plain untagged Spell-level family; do not
   add it merely to improve a controlled Ice case while a real selected blocker
   has a clear fix. Evidence: `runs/owned-item-property-import-audit-01.md`,
   `runs/owned-pre-amulet-contributor-audit-01.md` and the checked packet README.
   The newer audit corrects the older ignored scope audit's false assertion
   that no Ring bonus-copy consumer exists; the published packet remains Partial
   and its Amulet arithmetic is unaffected.
   The selected owner census and justified closure boundaries are in
   `runs/owned-minion-property-owner-closure-audit-01.md`.

   Prove complete real external/support inventories and all relevant owner
   programs before publishing receiving V3/stages V3 or claiming a working original.
   The current operations V21 Partial release does not weaken that gate. Establish real
   admission predicates and supporting mechanics for all six observed Ice support
   candidates; the finite arithmetic component's already-admitted positions are
   not that proof. Supported/final quality and broader ordinary property families
   remain explicit. Validate nonzero real item/support mutations, independent
   sources, exact requests and unchanged originals at the next checkpoint.

3. **Connect the remaining numerical consumers.** Real source-property
   integration is step 2. Existing minion-level item producers alone do not prove
   completeness of incoming properties or all final inputs.

   The three selected Command Skill Damage defaults (`0edd/139d/10e4`) are
   now published and validated through the actual native contribution programs.
   Continue from channels `3302`–`3304` and existing Command eligibility; do not
   rebuild their parser/producer or substitute unconditional damage, cooldown
   or More channels. Final damage reduction must join actual unconditional and
   conditional contributions with complete receiving/owner coverage. The finite
   Basic 0/Gas 55 component does not provide that closure. Prioritize the ordinary
   remaining routing proof above and the next measured selected-owner/input blocker;
   further source harness work must enable a concrete native consumer.

   The Djinn preparation sidecar already proves ten supports and 1,296 admission
   contexts, including Original01. Integrate it only with complete real owners,
   inputs, stages and receiving metadata. Exact Direct source correspondence
   now retires the six selected target-identity issues: Bidding II twice,
   Magnified Area I, Muster twice and Frost Nexus. This is separate from complete
   mechanics. Bidding II/III have bounded numerical programs in the release
   and a checked receiving authoring fragment. Magnified Area I/II now also have
   published Partial numerical owners; Muster and Frost Nexus numerical owners
   remain absent. Admission and finite delivery do not supply
   complete owner, contributor, stages or receiving inventories. Integrate the
   [Bidding packet](../data/owned/poe2/3887ae68/bidding-support-delivery/README.md)
   only after those actual inventories are proved. Final damage/cooldown
   metrics remain separate work. The [support catalogue/supply correction](legacy-retirement.md#support-catalogue-entries-versus-executable-supply-2026-10-05)
   is now published across 568 Known Gems, with mixed active/support and unresolved
   counterexamples retained. Bidding tests consume the corrected actual release.
   Complete potential-supply declarations and real activation remain open; do not
   copy finite fixture closures into production. Rebase the older Ice Nova finite
   source-property fixture before retiring its historical membership isolation.
   A new preparation-scoped completeness model would need
   a design discussion. Muster still needs actual parent PersistentMinionTypes
   authority. Magnified Area I's Area Increase and resource-cost factor are
   implemented together; its zero damage stat has no matching I-tier map and
   supplies no damage producer. Final consumers and full closure remain open. Frost Nexus also
   adds ground-effect type and mana cost, while its ground radius/duration need
   separate consumer/gameplay evidence. A missing PoB handler is not proof that
   the game mechanic is inert. Keep these gaps explicit rather than closing a
   support from admission alone.
   Sniper reservation needs build-driven parent Action contexts without
   hidden reference queries. Current Action contexts come from real queries,
   usage/choices and receiving declarations; merely owning a parent Action program
   does not request that context. Establish genuine build-derived demand and review
   its selection contract rather than adding fake usage or queries to make it run.
   Quality, infusion, Gigantic, physical-range damage
   and complete offence/defence follow their measured input dependencies. Finalize
   and evaluate the exact unchanged request before claiming a working build.

4. **Continue bounded retirement and Lua semantic cleanup.** The orphaned Import
   equipment/Mace closure and its nineteen exclusive tests are removed in the
   earlier retirement checkpoint; all six general formatter tests remain and all 175 Import
   library tests pass. Next audit shared skill identity and source-number
   formatting consumers before removing broader profile preparation. Migrate
   the live fixed-Elemental early representation to canonical raw data before
   deleting its owned definition path. Retain independent numerical references
   and acquisition consumers; no compatibility facades for abandoned APIs.
   Classify Lua-specific behavior by domain purpose and evidence using the debt
   inventory, with reviewed numerical changes and the five-build gates intact.

The remaining enemy critical chance, critical bonus and attack/cast-time inputs
can reuse the checked numeric-override seam. Source `ConfigOptions.lua:1984–1986`
resets their placeholders, and boss selection at `2235–2240` can overwrite them.
They therefore need injected encounter defaults and explicit override presence,
not saved-placeholder fallback. Before delivery, test stale placeholders, zero,
fractional values and boss changes against the actual consumers at
`CalcDefence.lua:2268`, `2273` and `3412`. This is input work, not completed EHP or
configuration-inventory coverage; no public model change is currently needed.

Enemy distance is a distinct future input: its count-style zero fallback differs
from the raw override lane. More numeric controls alone cannot close the current
configuration or usage inventories. Do not add unrelated catalog families while
an identified selected-request blocker has a clear fix.

The usage implementation reuses `UsagePolicySelection`/`UsagePolicyDraft`, shared
validators and `RuleOrigin::Usage`. Preserve this single composition/execution
path when extending Import projection. Bind each actual source row to its newly
allocated SkillUse and containing preset only for authored skill roots. Generated
preferences bind to already materialized Allocation, EquipmentUse or ItemModifier
providers and their exact declared effect paths; they do not create a replacement
SkillUse or Gem. Policies must validate typed slots, source/domain guards and
work limits before traversal. Omission must preserve prior normalization bytes;
known preferences may coexist with Pending inventories. No inventory closure
follows from persistence alone.

The earlier source audit in `runs/owned-usage-import-next-audit.md` identifies the
Original05 rows. New complete-source witnesses now cover the four skeletal
families' occurrence/action/count inputs and Ice Nova's actual constructed stat
sets, in both JIT modes. See `runs/owned-minion-occurrence-inputs-01/` and
`runs/owned-spell-stat-set-source-01/`. Minion count has a real reservation
consumer; Full DPS uses separate aggregation rules. The minions' global switches
are inert in this source domain, so do not repurpose Offering's Boolean policy
as minion activation. Ice Nova has two constructed stat sets, not three inferred
from aliases. Ice and Sniper action selection/physical disposition, plus Sniper count/reservation, have
native consumers or checked projection. Other typed consumers remain unfinished; none of this
evidence closes scenario usage by itself.

### Accepted and pending owner decisions

The owner accepted these contracts:

- **Accepted:** [skill-preset usage composition](owned-skill-usage-proposal.md), typed
  preferences on the supplying skill preset composed with exact scenario overrides.
  Composition, native execution and initial Boolean source projection are
  implemented and tested. Full inventory proofs still do not follow from them.
- **Accepted:** [preparation versus execution readiness](owned-preparation-readiness-proposal.md),
  one occurrence topology/effect graph with explicit phase dependencies. Its
  versioned declarations, compiler gates and positive public support-plan proof
  are implemented and the component, compatibility and portable checks passed.
  The bounded [stages V4 local-item extension](owned-preparation-item-facts.md)
  is implemented and passes the current real-item integration, compatibility,
  dependency, scratch/Rayon and WASM compilation checks. Complete contributor
  inventories remain a separate gate.

- **Accepted:** [Direct SkillUse inputs](owned-skill-occurrence-input-proposal.md),
  shared typed slots with explicit authored/provider authority. Complete/draft
  persistence, binding and native reads pass component tests. Reviewed manual
  Djinn raw inputs and complete intrinsic inventories now import into the V5/V17
  release, with declared Command/Actor/child topology. Final input producers,
  support admission and complete numerical coverage remain unfinished.

- **Accepted:** [source-property preparation](owned-source-property-preparation-proposal.md),
  reuse existing Skill occurrences with exact source membership and aggregation
  permissions. Bounded original-source evidence and the executable native relation
  component pass. Complete real property producers and final assembly remain open.

**Accepted 2026-10-04:** [generated-skill usage ownership](owned-generated-skill-usage-proposal.md).
The chosen model keeps intent in the skill preset and adds explicit applicability
for exact tree/item providers selected by other build axes. Proven nonselection
can leave a preference dormant; stale, unknown or partial providers remain
obligations. Versioned storage, full-content data-aware validation of all stored
resolved preferences, including overridden and dormant records, and checked
composition are implemented and tested. One bounded source-to-provider resolver
now serves raw inputs and requested-count preferences. Enabled/global/reporting
dispositions and whole usage closure remain open. The legacy preference field
retains its stricter local ownership contract.

**Owner decision 2026-10-05; determinism required:** classify the Frost cold
MAIN/CALCS discrepancy as an upstream PoB initialization bug. Exclude only its
affected comparison for this build, preserve the evidence and validate other
originals with independent fresh replays. An exclusion is not a parity pass or
permission to omit unrelated queries. Do not change the backend to the proposed
rebuilt protocol, encode source cache state in native rules, retry until matching
outputs or accept flaky parity. See the [exact source issue](owned-frost-bomb-usage-evidence.md#upstream-issue-disposition).
The [determinism contract](execution-and-interfaces.md#reproducibility-and-throughput)
separates native calculation, exact oracle replay, fresh import identity,
deterministic search, timed search and telemetry. Current Frost evidence shows
repeatable source lifecycle differences, not demonstrated same-protocol randomness.

**Accepted 2026-10-04:** separate [socket configurations](owned-socket-configurations.md)
from rolled descriptors and physical inventory copies. Versioned records,
projection, host-local receiving and migration remain unimplemented; the choice
does not certify existing socket inputs or effects.

**Accepted 2026-10-05:** [generated raw-input ownership](owned-generated-skill-inputs-proposal.md).
Use preset-owned bindings to exact generated Skill occurrences, shared typed
slots and explicit supplying-declaration permission. Preserve provider-projected
levels and unique producers within the selected preset; alternative presets can
store different values. Reuse usage applicability checks without turning raw
quality into usage or adding another graph. Concrete codecs, binding and native
input producers pass focused validation. Bounded source-bound Import
correspondence and game-data publication now cover the reviewed Djinn/Firebolt
families; other generated sources and full usage/mechanics remain open. Source preservation is distinct from game
legality and permitted optimization edits.

**Accepted and implemented 2026-10-06:** [existing Actor rule ownership](owned-existing-actor-rule-ownership-proposal.md).
Shared Player rules bind once through injected Actor applicability; no actor is
allocated and no class-specific default is inferred. The checked game packet
moves intrinsic Life out of eight Classes, preserving remaining Partial coverage.
Future non-Player targets still require ancestry, readiness and activation proof.

**Accepted and implemented 2026-10-06:** [shared Player equipment-slot reads](owned-equipment-slot-state-proposal.md).
Expose validated occupancy and the exact selected EquipmentUse's computed
Stat/Capability to existing-Player Actor programs. Reuse one bounded relation
resolver with Action routing. Empty, unresolved and ambiguous slots stay distinct;
raw item parameters and game equipment legality retain their existing owners.
Core/Data/Engine validation passes, including actual invocation authority, stage
and cycle dependencies, query independence and parallel worker reuse. Game-state
producer publication and source evidence remain the next shared-state work.

**Accepted and first real packet validated 2026-10-06:** [requested Skill participation](owned-skill-participation-proposal.md).
The consumer preserves mechanical supply for preparation and uses typed usage
to gate execution and descendants on the same graph. Physical-primary usage now
shares one multiple-policy compiler with Direct/generated occurrences; the old
usage variants and Boolean/numeric split are removed and all five inputs rebuilt.

Published Sniper policy `332b`, parameters `332c/332d` and Stat `332e` derive an
early conjunction for Skill `0012`. Count `326a/326b/326c` and physical supply
`0011` → `0016` → `0012` are unchanged. All-five input/source inverses, explicit
source controls and typed preset/scenario native integration pass. Readiness is
an authenticated authoring fragment; the release still has no evaluation bundle.

**Next participation blocker:** Djinn, Firebolt, Offering, Frost Bomb and other
skeletal skills retain Partial parameter inventories. Resolve actual missing
inputs before adding execution readiness; importing enabled flags alone is not
closure. Extend family integration without changing the common gate or native
query semantics. Keep broader generated-provider dormancy/source controls and
complete-original evaluation as separate acceptance gates.

**Accepted 2026-10-06:** [explicit generated-source field accounting](owned-generated-skill-dispositions-proposal.md).
Use the existing Import policy and a separate private proof pass, preserving
actual successful raw/count outputs and unresolved usage obligations. The owner
subsequently clarified that backward compatibility is unnecessary: prefer one
current behavior and rebuilt data over parallel V1/V2 paths. The checked cutover
keeps declarative policy identities, emits current sidecar schema21 whenever
that policy runs, and reports the actual proof-file hash. Reimport all five
against stored previous evidence; no old importer mode is needed. Account for every
field of an admitted source pair; partial evidence retains its fallback.

**Accepted 2026-10-06:** classify the reproduced rare-item title routing as a
[PoB presentation-name defect](legacy-retirement.md#name-based-item-routing-source-discrepancy-2026-10-06).
The owner first clarified whether this was a corrupted unique; it is a synthetic
rename of an existing rare Amulet, with no obtainable-item claim. Retain a narrow
exception for the affected comparisons, never a parity pass. Native names remain
presentation. No unchanged original, independent copy behavior, actual unique
semantics or numerical owner closure is covered by this exception.

## Delivery plan and gates

### Session follow-up register (2026-10-04/05)

This register captures the review work without creating another competing resume
plan. Details and evidence live in the linked design/debt documents; completion
requires their gates, not merely adding the task here.

Reconciled against the recent owner decisions and implementation audit on
2026-10-05. Accepted direction, implemented components, pending integration and
unresolved design contracts are distinct states. The [retirement inventory](legacy-retirement.md)
remains the single cleanup register; the historical checkpoint text in other
documents must not reopen accepted decisions or promote unvalidated work.

| Work | State and next action |
| --- | --- |
| Shared-contract generality | Initial review complete. Before the next public extension, exercise contrasting authored/generated sources, exact repeated occurrences and applicable stacking/receiver shapes. [Generality gates](build-generality-review.md) remain open. |
| Generated source-property owners | Contract investigation pending: current relation owners support authored SkillUse and PlayerScenario only. Exercise a real generated owner using exact provider identity and the accepted shared input slots/producer authority. Review the owner/context extension before implementation; generated endpoints alone do not establish owner support. |
| Effect-application breadth | General application model accepted and Maximum component implemented. Select a real contrasting stacking/grouping case and prove duplicate, cap, source and recipient semantics before extending its bounded policies. Do not replace the shared graph with effect-specific paths. |
| Physical-input/reference separation | Design refinement pending: classify fields without requiring successful MAIN/CALCS selection or a live Pending usage issue to preserve known intrinsic facts. Preserve independent unknown-field/usage obligations. |
| Immutable normalization preparation | Compile existing checked policies once per exact release; separate construction/traversal budgets and prove identical outputs. Implementation pending; no storage-layer change. |
| Scoped numerical coverage | [Proposal](owned-coverage.md) pending concrete reviewed proof contract. Unknown effect reach remains blocking; no known-edge-only pruning or relaxation of original-build acceptance. |
| Exact objectives and native optimization | D4 pending: bind objectives to owned actor/action/stat-set requests and execute one legal locked candidate mutation with a fixed scenario through the native path before expanding search. |
| Independent breadth and CI | Add provenance-bound holdouts by mechanic intersections and an explicit reproducible integration subset. Existing five examples are development cases; important ignored tests are not automatically exercised by ordinary CI. |
| Historical replay retirement | Reimport with the one current implementation and retain useful stored source evidence. Seventeen older ignored publication harnesses need consumer review: rebase useful assertions to current semantics or remove obsolete scaffolding, rather than add compatibility paths. Declaration integrity and source parity remain required. [Retirement/migration inventory](legacy-retirement.md#historical-publication-replay-migration-2026-10-05). |
| Generated usage | Versioned preferences, all-record data-aware proofs and explicit applicability diagnostics pass focused validation. Shared exact-provider proof now serves raw quality and requested-count import independently. Same-definition allocations in different presets remain distinct. Enabled/global/reporting dispositions and usage inventory closure remain open. [Contract and gates](owned-generated-skill-usage-proposal.md). |
| Requested participation | [Common gate](owned-skill-participation-proposal.md), unified typed Import and first Sniper packet are validated. All-five/source/native checks preserve count, effect controls and mechanical availability. Next close genuine parameter inventories for remaining skills before readiness publication; no PoB preview bypass or default is imported. Full-original coverage remains open. |
| Resource ownership and demand | [Resource-obligation proposal](owned-resource-obligations-proposal.md) awaits owner review. Ordinary sustained reservation should be independent of queries, with exact Skill/payer/resource identity and explicit Action dependencies where required. Define Skill support recipients and contrast cost conversion, mines and stance/toggle selection; then migrate the current Action reservation bodies and validate query invariance. Gigantic's producer/status and isolated Life/Damage-factor packets are published; final reservation delivery remains open. |
| Generated raw inputs | Accepted exact preset bindings now have V6 permissions, V19 producers, source-bound quality joins and checked publication for the reviewed Djinn/Firebolt families. Existing provider levels survive. Archived cross-axis correspondence, other generated families and final mechanics remain open. [Contract and gates](owned-generated-skill-inputs-proposal.md). |
| Generated source-format breadth | Replace finite Item-name frames with proved general derivation over admitted layouts, and add evidence for normalized-only level correspondence. Census competing saved representations before scalar admission. Original04's Firebolt provider still has unresolved rune/layout/grant-line authority; raw quality cannot bypass it. [Bounded adapter follow-up](legacy-retirement.md#bounded-generated-source-import-coverage-2026-10-05). |
| Configuration/source dispositions | Presentation and exact item-range ownership are implemented; full semantic closure is not. Original05 issue `01f2` retains 35 Config-local and 38 non-Config origins after the six-row generated accounting proof. Account for generated-group fields and remaining actual consumers before retirement; partial scalar success or local Config proof cannot clear global fallback. Keep scenario usage/assumptions separate. [Evidence and gates](owned-configuration-dispositions-proposal.md). |
| Global-switch source reachability | Declared-stat and two actual-parser negative controls pass cold/lifecycle/repeat and JIT checks with exact generated/manual occurrence identity. Independent review and execution disproved the suggested missing-name alias. Full `ExtraSkillStat` producer/filter/transform reach remains unproved; no field non-applicability certificate or inventory closure is published. |
| Reward contributor coverage | Three additional selected producer inventories and four typed Actor channels are published and tested, including both Charm effects. Receiving and final threshold/recovery/capacity formulas remain open. Continue actual selected owner/contributor gaps; no whole-build closure follows from these producer programs. |
| Selected passive receiving | Growing Swarm's complete default two-effect owner and Sniper receiving are published through existing contracts. Conditional/unconditional cooldown sums stay distinct; final duration, area/radius, other minion receivers and complete contributor coverage remain open. [Packet and limits](../data/owned/poe2/3887ae68/growing-swarm/README.md). |
| Selected support delivery | Magnified Area I/II contributions and actual per-Action Area eligibility are published and validated for fourteen contexts, with scratch/Rayon isolation. Rapid Casting I/II now publishes cast-speed contributions and passes both-stat-set/independent-root checks using the same contracts. Encroaching now delivers factor 1.1 on existing channel 32fa with exact mixed-family source/native checks; ground growth and source subtotal-rounding authority remain unresolved. Prolonged I/II now supplies duration/cost factors on real Offering and Ice actions; Offering final-input production is published; complete contributor coverage, other final-input domains, final timing and application-lifetime transfer remain open. Primary Summon delivery, final radius/resource costs and complete owner/receiving inventories remain open. Keep ordinary resource-cost factors separate from reservation factors. [Delivery](../data/owned/poe2/3887ae68/magnified-area-support-delivery/README.md) and [eligibility](../data/owned/poe2/3887ae68/action-area-eligibility/README.md). |
| Support catalogue/supply boundary | Corrected one offline producer and its validators; published 568 Known Gem replacements from a full 966-Gem census. Primary associations stay in Import, non-support candidates and Partial closures survive. Bidding consumes actual corrected data; the historical Ice Nova component retains named finite isolation until rebased. Hidden-helper semantics and complete genuine supply remain open. [Retirement/integration gate](legacy-retirement.md#support-catalogue-entries-versus-executable-supply-2026-10-05). |
| Exact Direct support targets | Shared V2 source proof resolves 40 saved targets/eleven order sequences, retiring eight selected Original01 and six selected Original05 issues. Absent source is required; explicit-empty raw records stay Pending. Input values, usage, activation, full origin inventory and numerical readiness remain independent. [Evidence](owned-djinn-provider-evidence.md#exact-manual-support-targets). |
| Socket configurations | Separate descriptors/configurations/copies accepted. Implement identity, persistence, per-host projection, migration and exact local receiving; independently prove host-local aggregation before rounding, saved-effect reconciliation exactly once, physical-copy/current-setup feasibility and locks. Unused item setups must survive. [Contract and gates](owned-socket-configurations.md). |
| Determinism/source exception | Strict per-candidate determinism required. Frost initialization behavior is an upstream bug with a narrow excluded comparison. All five unchanged originals pass independent fresh replay across both JIT modes; no warm/retry canonical protocol adopted. |
| Legacy and resistance cleanup | Orphaned Import equipment/Mace closure removed; 175 Import library tests and strict Clippy pass. Fixed Elemental still needs canonical raw migration, and Engine profile adapters/default legacy dependencies remain. Follow the [retirement inventory](legacy-retirement.md). |
| Lua behavior/type audit | Requested and planned: classify source-language behavior by game/format/reference purpose and valid-input impact, then isolate or remove it. Preserve justified numerical laws; discuss uncertain semantic changes. |
| Attribute grouping and source caches | Six ordinary attribute receivers and guarded complete-empty MORE producers are published. Every potential Multiply effect rejects that bounded domain before activation; six current INC memberships are now Complete; BASE membership and the global query inventory remain Partial. The BASE audit now proves a signed-scaling counterexample, real post-passive bonus copies and missing socket-origin ordering; resolve these concrete domain gaps and review typed Boolean flag aggregation. Nonempty MORE grouping still needs a reviewed domain law: the synthetic 1.02/1.0201 setup discrepancy needs legal-input reachability. Earlier cache-mode options are historical; no native cache-dependent mode or extension of the Frost exception is approved. [Evidence](owned-attribute-setup-evidence.md). |
| Reference reporting order | PoB `Build.lua:2393` mutates the calculated `SkillDPS` array into display order. Record exact row identity across that reorder in the optional witness; preserve calculation contribution identity and keep presentation sorting outside native numerical semantics. [Reporting evidence](owned-full-dps-aggregation-proposal.md). |
| CI regression, timeout and infrastructure | Earlier V18/V20 expectation regressions and the combined Windows compile bottleneck are fixed. Run [37409208353](https://github.com/Azaril/poe-optimizer/actions/runs/37409208353) at `230494e` completed all twelve Windows/Linux jobs successfully. Windows PoB's combined compilation/test step took 3h28m40s, within its prior successful duration range; no hang or failure was found. At the BASE audit check, run37552883848 at `0ff9210` was pending behind active run37529126901 at `7b7fb1b`. The latter exposed stale unified-usage asset paths in both engine jobs; the current main test correction passes all six ordinary target tests locally. Hosted success for that correction and this checkpoint is not yet confirmed. Read the latest [main workflow runs](https://github.com/Azaril/poe-optimizer/actions/workflows/rust.yml) before treating a newer revision as green; pending runs can be superseded by the next checkpoint. Main retains running validation and coalesces pending revisions. Separate compile/test timings before choosing further sharding. Keep every package/target and required aggregate check; infrastructure delays do not justify dropping coverage. |
| Source validation diagnostics | Accumulate bounded per-original failures and run both JIT modes before failing overall; the successful current run skipped no comparisons. Investigate the optional Windows PoB test's `LNK4098` CRT link warning as reference-runtime packaging debt. |
| Authoring/execution simplification | Retain [A1-A4](rule-execution-model-investigation.md): measure maintenance/update cost and real evaluation workloads before choosing compact DSL, declarative, injected-native or compiled execution changes. Work inside the accepted owned boundary; no revival of Lua/UI emulation. |
| Tables, curves and segmented storage | [S1–S4 investigation](owned-scaling-data-investigation.md) added at the owner's request. Initial scan covers 16 tables/835 cells: Offering has two exact affine segments, while several damage tables do not compact usefully that way. Distinguish resolved samples, actual formulas and item-tier thresholds. Prototype lossless encodings in Rust, benchmark against compact arrays/compression, review any format change, then migrate with exhaustive domain and native/WASM parity. No runtime or artifact change yet. |

The authoritative architecture phases are [D0-D6](architecture-migration.md#phases-and-exit-gates).
Do not reuse the older archived D1-D5 profile milestones as current instructions.

| Phase | Current state and next gate |
| --- | --- |
| D0: boundaries and retirement | Owned architecture accepted; delete obsolete paths continuously, retaining named useful references |
| D1: semantic input | Owned model/import exists; unresolved original-build input contracts remain |
| D2: offline data/compiler | Owned packages and typed programs work; mechanic coverage/conversion is incomplete |
| D3: native evaluation | Component execution works; first complete Twister/Sniper requests, then all five originals |
| D4: search integration | Generic search/objectives exist; connect owned candidates and evaluator across all six dimensions |
| D5: retirement and breadth | Active alongside D1-D4; finish legacy isolation and independent whole-build holdouts |
| D6: performance and applications | Measure real native workloads, then shared reports/events, UI and web delivery |

Stable product milestones remain: M0 bootstrap complete; M1 evaluator/fixtures
in progress; M2 generic search contracts partly implemented; M3 first usable
joint optimizer unfinished; M4 broader catalogs/upgrades and M5 richer objective
policies follow. Retired Spark/Mace profile benchmarks do not satisfy current
native performance or optimizer gates.

The first usable optimizer must jointly search classes, ascendancies, passives,
equipment, support gems and supporting skills within explicit finite catalogs.
Preserve 1..N required skills and exact item locks, configurable objectives and
constraints, bossing/mapping scenarios and configurable 5-30 minute thorough
runs. Test coordinated changes, tiny exhaustive domains, deterministic parallel
execution, cancellation, recovery, deduplication and verified exports. CLI/core
contracts come before the GUI; Tauri remains a candidate. PoE1 is a later distinct
versioned adapter.

M4 must rank conditional upgrades and bundles with explicit inventory, cost and
baseline semantics. M5 adds unit-checked derived/composite/ordered objectives,
soft preferences, Pareto selection and explicit robust aggregation while
preserving hard constraints. The GUI must reproduce CLI results through the
same libraries, support responsive cancellation and package the native evaluator
reproducibly; optional reference tooling stays separately installable.

### M1 checklist: native calculation and optional reference parity

- [x] Pin original PoB, preserve caller sources and retain optional reference hosting.
- [ ] Complete the owned request/scenario boundary across all five originals.
- [ ] Produce final metrics with explicit units, availability and contributor coverage.
- [ ] Match complete native originals and independent component references.
- [ ] Match legal mutations and interacting changes in every search dimension,
  including export/reimport and multiple exact locks.
- [ ] Prove fresh/reused/parallel isolation, bounded failures and cancellation.
- [ ] Measure real native preparation, candidate evaluation and memory separately
  from optional reference-worker costs.
- [ ] Complete versioned data conversion, update/reconciliation and broader mechanics.

### M2 and M3 acceptance details

Objectives are generic registered metrics with explicit units and coverage;
unsupported policies, contradictory constraints and nonfinite thresholds reject.
Richer policy implementations must not require redesigning candidate search.
M3 still requires all of the following through the owned evaluator:

- One joint run across all six dimensions, with legality/repair and every requested lock.
- Complete-candidate verification independent of optimization caches, with reserved
  verification budget and exact realized export/reimport comparisons.
- Shared CPU/memory limits, bounded queues, in-flight deduplication, cancellation,
  bounded retries and consistent attempt accounting.
- Preflight assumptions/coverage, baseline comparisons, grouped changes, constraint
  shortfalls, distinct alternatives, JSON artifacts and offline HTML reports.
- Atomic recovery with actual verification state; recovered seeds are freshly
  checked. A warm start gets a new budget; exact search-state continuation is separate.
- Quality against random, greedy and alternating-domain baselines across several
  seeds and equal budgets. Measure scaling and 5/15/30-minute end-to-end quality
  on recorded hardware for bossing and documented mapping proxies.

Saved-result reranking is analysis of recorded values, not a new search. Legal
infeasible exploration remains distinct from verified feasible incumbents.
The M2 synthetic suite must include an exhaustively checkable domain where every
single-change improvement path stalls but a coordinated change wins.

## Fixture ledger and technical unknowns

The [fixture ledger](../tests/fixtures/builds/README.md) preserves source identities;
the [five-build inventory](breadth-validation.md) preserves the wider working set.
Original and independently hosted numerical fixtures remain useful after profile
code retirement. Preserve selected minion/action identity, Full DPS membership,
count/usage assumptions, grants, separate point pools and nonfinite availability.
Do not silently repair unknown skills or interpret labels/tree versions as verified
game-patch identity. Ask for product direction when evidence leaves a real choice;
source-internal IDs and mechanics audits are implementation work.

## Breadth of validation and data-driven build admission — next phase

[Breadth validation](breadth-validation.md) and the
[mechanism inventory](breadth-mechanism-inventory.md) define the corpus.
All production inputs are caller supplied and data driven; fixture names must
never select calculation behavior.

- B1 caller-configured intake and provenance: implemented; preserve exact originals.
- B2 coverage inventory: partial; report mechanism gaps separately from first errors.
- B3 general input seam: owned model accepted/implemented in part; finish real inputs.
- B4 independent corpus: add held-out attacks, spells, minions, DoT, conversion,
  triggers, recovery/reservation and supporting-skill interactions with explicit scenarios.
- B5 identity/legality/interactions: preserve export/reimport, view changes, multiple
  locks and separate ordinary/ascendancy roots and point pools. Source-missing nodes
  are distinct from disconnected ascendancy trees and exceptional allocations.
- B6 usefulness/performance: measure native preparation, parallel evaluation and
  complete search on admitted real builds. Unsupported cases remain visible.

The five development originals are not holdouts. Never simplify them to manufacture
parity, regenerate expected outputs from native results or count unknown outcomes
as matching numbers. Store perturbations separately.

## Execution-model investigation: A1-A4

The [investigation brief](rule-execution-model-investigation.md) remains a follow-up
to the owner's concern about loader/parser/interpreter cost. The later owned-domain
decision controls current work; this investigation does not reopen PoB UI emulation.
A1 inventories current consumers and maintenance/runtime costs; A2 compares bounded
DSL, declarative, injected-native, interpreter and offline-lowering approaches;
A3 discusses measured results and a proposed decision with the owner; A4 implements
the agreed simplification and removes superseded paths. Compare the same real
mechanics, cold preparation, changing candidates, steady evaluation and update effort.
Keep the native/WASM path and independent parity. Unneeded experiments can be
deleted now; total retirement does not require completing every prototype.

## Definition storage and UI search

Storage decision complete: generated owned packages plus loaded Rust indexes;
no database/ORM milestone. The [storage assessment](definition-storage.md) is
historical context. At the discovery/UI phase, add a bounded query API over the
selected snapshot with stable IDs, labels, filters, coverage and a derived
prefix/substring index. Measure latency/memory before adding specialized search.

The owner requested a [table/curve/segment investigation](owned-scaling-data-investigation.md)
on 2026-10-05. Its initial pinned-source and sixteen-table audit is complete;
the broader S1 census, S2 Rust prototype/benchmarks, S3 reviewed decision and S4
migration remain open. Separate semantic functions from file encoding and hot-path
layout: lossless segments may compile once to shared dense arrays. Fitted curves
must not replace exact values or create interpolation/extrapolation authority.
Storage optimization remains D6 work unless a real D3 mechanic requires a curve.

## Seeded jewel opportunity: J1-J5 follow-up

The [design opportunity](seeded-jewel-search.md) and
[feasibility study](seeded-jewel-feasibility.md) remain opt-in future work.
J1 must establish game/version applicability, compatible seed data, provenance
and costs; the pinned PoE2 seed branches and available PoE1 data are not assumed
interchangeable. J2 adds an injected provider with radius/overlap/removal parity;
J3 adds bounded parallel seed/socket/path search with explicit budgets and caches;
J4 adds reproducible item/tree reporting and trade handoff; J5 validates independent
families and compares search quality/cost under equal budgets. These gates do not
displace the five-build MVP. No native seed provider/search is implemented.

## Tooling and checkpoint workflow

New maintained tools and tests use Rust. Existing Python exporter/intake tests
remain useful; the owner explicitly deferred converting them. Their eventual
coherent replacement is [T1](architecture-migration.md#t1-rust-tooling-and-test-consolidation).

At session start, read this resume point and inspect Git state. At each coherent
checkpoint:

1. Name the subsystem changed and the actual selected-build blocker addressed.
2. Rerun unchanged saved selections for all five originals; evaluate when admitted.
3. Record exact data/source identities, commands, results and retained failures.
   Distinguish component evidence from whole-build and hosted-CI results.
4. Update this compact current state and next action. Append detailed completed
   checkpoints to history; do not accumulate competing current plans here.
   Reconcile new discussion findings with the session register and the existing
   design/debt entry. Every unfinished finding needs a next action, acceptance
   evidence and any actual decision dependency; an accepted recommendation is
   not an implemented feature. Replace stale "awaiting owner" wording when a
   decision arrives, keeping the historical context explicitly historical.
5. Update design only for changed architecture/contracts, discuss significant
   direction changes, review the diff and publish a coherent tested checkpoint.

Keep Cargo invocations serialized in the shared workspace; independent tests
within a target can use bounded parallelism unless they share mutable fixtures.
For heavy legacy numerical oracles, use CI's optimized test profile while retaining
debug assertions and overflow checks; snapshot reconstruction is very slow in an
unoptimized test build. Format per package on Windows to avoid command-length
limits. Do not edit
`crates/poe-optimizer-engine/src/owned_allocations.rs` or its tests during this work.
The owner authorizes commits/pushes to main and CI-log inspection.

Deferred product decisions: distribution license before a public release,
frontend/packaging before GUI delivery, and acquisition sources before live
trade ingestion. They do not block current native build support or cleanup.
