# Implementation plan and resume point

Updated: 2026-10-05 (EDT).

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

## Latest implementation checkpoint: real Offering actions and Prolonged Duration

The accepted preset-owned exact generated-input bindings remain implemented,
including provider-level authority. Requested participation and the newly concrete
[V2 generated-source accounting proposal](owned-generated-skill-dispositions-proposal.md)
are separate pending decisions. No dependent public policy change is implemented.

The [Prolonged Duration packet](../data/owned/poe2/3887ae68/prolonged-duration-support-delivery/README.md)
adds Pain Offering's real player Action to its existing Skill and physical supply.
The source's minion-related types do not create a minion actor for that action.
Its separate minion buff applications remain distinct graph entities. New IDs
`32fd`–`3301` cover one output slot, part, mode, singleton stat set and a generic
Action duration-factor Stat. Existing supply/grant `3221/3222`, final-input
projections `3223/3224` and producer authority `3225/3226` remain unchanged;
both existing output inventories retain their Partial closures.

Two new Partial support owners publish prepared-input, applicability and delivery
programs. Prolonged I/II contribute duration factors 1.30/1.35 and ordinary-cost
factor 1.20 on existing channel `32fa`. Receiving is finite to the Offering action
and both Ice Nova stat sets. Existing schema V6 / operations V20 suffice; there
is no new Core/Engine operation, public runtime API, evaluation bundle, final
duration/cost formula, reservation rule or application-lifetime transfer.

The optional source witness passes in **398.88 seconds** over 24 cases, three
fixed lifecycle observations and both JIT modes. All five unchanged originals
and focused Offering/Ice controls have independent replays. All four raw/compared
reports are byte-identical at **36,246,047 bytes**, SHA256
`e5d529b3bd11f10252b55001a7ecc26e8395fa7de3b521759363fc4bb35c14c7`.
Exact source objects, original methods and physical XML origins establish both
tiers, removal/disablement, family ordering, raw level/quality controls and two
distinct Offering occurrences. The committed 484,253-byte projection omits only
unrelated final-output payloads; publication authenticates the complete reports.
Source01 failed because the expected local mapping omitted its source annotation.
The corrected source02 off report is exactly the same as source01 raw evidence;
no source value, observer, tolerance or lifecycle was changed to obtain a pass.

Seven native tests pass in **6.28 seconds** with unchanged published bodies and
the real Offering supply. They cover both tiers, two independent Offering roots,
both Ice stat sets, raw level 20 versus explicit final level 22 projection, removal,
disablement, duplicate/family selection, actual Partial refusal, missing final
producers, A/B/A scratch reuse and four-worker Rayon replay. The negative
admission control now follows the declared ReceivingSkill contract: changing only
the physical input owner's types leaves delivery intact; a rejected generated
receiver retains exact false applicability and Inactive contribution records.
Removal has no such rows. The initial assertion conflated those cases; no Engine
or game-data change was needed. Finite preparation inputs, final22/0 producers
and surrounding closure remain labelled fixture boundaries, not original readiness.

Publication passes in **27.10 seconds**, reproducing all eighteen artifacts and
preserving prior provenance, five original source files, all 110 queries, selected
issues and source links. The opt-in topology migration helper proves exact new
allocations/owners, two output-only replacements and the full inverse recipe;
historical Stat-only staging and commitment order remain unchanged. Shared
fixtures gained only explicit caller-provided readiness/stage boundaries.

The CLI stack-size build script now watches only its own file. It reads no
package files; Cargo target configuration and Rust source/include dependencies
remain independently tracked, and the Windows stack flag is unchanged. This
avoids unrelated documentation/authoring edits retriggering that build script.
The audit also corrected stale pending-contract and current-count wording in the
historical effect-application proposal. Duration rounding, application lifetime,
and malformed-level recovery are recorded in the existing Lua cleanup register.

Receipts: `runs/owned-prolonged-duration-source-02.log`,
`runs/owned-prolonged-publication-01.log`, `runs/owned-prolonged-native-02.log`.
All **25 historical native regressions** pass across Action Area, Magnified,
Rapid, Encroaching and Bidding, alongside the seven new native tests. Bidding
requires the checked `runs/owned-gem-executable-memberships-02/package` successor;
the earlier package failed its provenance precondition before evaluation.
Eight ordinary authoring tests, strict workspace/all-target/all-feature Clippy,
formatting checks for all eight packages, local links/anchors in fourteen
documents and `git diff --check` pass. Regression receipts are
`runs/owned-prolonged-regressions-01.log`,
`runs/owned-prolonged-bidding-regressions-02.log`,
`runs/owned-prolonged-authored-regressions-01.log` and
`runs/owned-prolonged-clippy-01.log`. Hosted success for the new pushed head is
a separate gate; the last checked f11a57c run is pending.
**Complete native original builds remain 0/5**, with Original05's five selected
issues and 79 configuration-linked origins unchanged.

## Checked baseline and original-build results

Use `runs/owned-prolonged-duration-support-delivery-01/package` as the integration baseline.
Current imports are its `original-01` through `original-05` siblings. The checked
publication retains the exact Direct-target normalization authority and historical
provider/raw-input receipts. Sidecars are V20 for Originals01/05 and V19 for the others;
this is distinct from native schema V6 and operations V20.

Its immediate predecessor is `runs/owned-encroaching-ground-support-delivery-01/package`,
input `3200e616c4c4de00733c0cd064dfd4dc8cf11b56519fe348e4d03787f9943f18`.
Run `publish_prolonged_duration_preserving_five_originals` in
`tests/owned_prolonged_duration_support_delivery.rs` with explicit
`POE_OPTIMIZER_TEST_PROLONGED_PRIOR` and fresh `POE_OPTIMIZER_TEST_PROLONGED_OUTPUT` paths.
The [packet's reproduction instructions](../data/owned/poe2/3887ae68/prolonged-duration-support-delivery/README.md#evidence-and-reproduction)
identify the separately required source evidence.

| Identity | Current value |
| --- | --- |
| Source revision | `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4` |
| Release input | `a9c7e639ca97d18646d3e91adbb4201f93ff40a906c8e5278498208ab60895e2` |
| Registry | `c423d01fd9cc7638800feeb47e36ddbf15162bb9882e11ad374490e2a03c306e` |
| Definitions content | `85c6887b460ebf22e3865f32769e151ffa30a423c021c8d96e845328d2c63dc3` |
| Normalization | `f33119790fa09a457e4e629019f3ee287212e465d8ce357c270e31bede041f7e` |
| Tree policy | `51d6b76a54859e52c991cc445823e0b9e4ac4814c4a599905e29cd72d383bada` |
| Schema / operations | V6 / `owned-domain-operations-v20` |

The eighteen package files total **60,838,825 bytes** with 101 provenance rows.
The definitions release is `pob-3887ae68-prolonged-duration-support-delivery-v1`.
Rules hash is `246c871dd515c48291df1469f759a5bdc671eb4ee63ca4fa956488728d51461e`; compiled rules hash is `dfacb25d508a6b3b88916a24ebd5d6249a9cac18c431ee0b70c9cf3c6b84c938`.
The registry ends at `3301`. Mechanics and integration remain Partial;
there is no evaluation bundle. Reviewed receiving fragments remain authoring
data, not separately loadable complete evaluators.

| Original | Unchanged saved selection | Selected unresolved issues | Native evaluation |
| --- | --- | ---: | --- |
| 01 | Kelari / Sand Djinn, Kelari's Deception | 106 | Not run: Pending |
| 02 | Twister, skill set 6 | 117 | Not run: Pending |
| 03 | Whirling Assault, average-damage mode | 109 | Not run: Pending |
| 04 | Crossbow Shot | 122 | Not run: Pending |
| 05 | Skeletal Sniper, Basic Attack, skill set 4 | 5 | Not run: Pending |

Original05 retains support-origin discovery `01de`, preset usage `0503`,
configuration `01f2`, external assumptions `0207`, and scenario usage `0208`.
Resolving these input obligations will not by itself complete numerical owners.
Crossbow's saved reference selection has no hit-damage output; preserve its
availability result rather than choosing another skill. Original source bytes
and all 110 query identities remain unchanged. Local `runs/` files are
reproducible evidence, not distributed game data.

## Next executable work

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

**Resume here:** keep Original05's five selected issues and 79 origins linked
to configuration issue `01f2` open. Its 101 item-range records already have exact
item/output or unresolved-item ownership; the remaining origins are 35 Config
and 44 outside Config, including 38 outside origins with only this issue.
The [generated-source consumer census](owned-configuration-dispositions-proposal.md#generated-group-consumer-census-next-blocker-2026-10-05)
must distinguish execution/reporting intent from PoB reference selectors for
thirty Skill/Gem origins. Reuse the shared resolver and field-level proofs;
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
alongside the bounded Original05 input proof while participation awaits a decision.
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

**Next concrete work:** review the explicit Import opt-in in the
[generated-source accounting proposal](owned-generated-skill-dispositions-proposal.md)
before implementing that proof. V1 must preserve historical outputs; the proposed
V2 uses the same input fields and an independent private accounting pass. Keep
successful quality/count imports when full-field accounting refuses a row.
This new serialized policy decision is pending, separate from accepted exact
preset-owned raw-input bindings and the pending participation model.

After approval, implement that bounded proof using the existing private Import
seams. In parallel, numerical integration should
use the published real Offering output/recipient topology and prioritize its
final-input producer chain described below. Settle ordinary-cost reduction and
rounding only with contrasting valid-domain inputs. Do not invent a parent Action or use source
absence to close ground growth. A required new public recipient/participation
contract must remain a reviewed design decision.

Selected Pain Offering uses Prolonged Duration **I**, while II occurs in saved
alternatives. Its duration/cost factors now use real player output `32fd`;
final duration/cost consumers and complete contributor inventories remain open. Final OrdinaryTiming requires
all eight explicit inputs, branch/rounding authority and complete contributors.
These are broader integration gates, not defaults supplied by Rapid's channel.

After the Prolonged/Offering topology checkpoint, prioritize **real Offering
final-input assembly** before final duration or payable-cost formulas. Authenticate
the ordinary and supported Gem-property queries at their actual consumption
points, including exact independent copies and selected/archived support tiers.
Reuse existing pre-support channels `30ac/30ad`, final producers `3225/3226` and
the unchanged primary-supply projections `3223/3224`. The intended finite native
join replaces the fixture's 22/0 literals with actual Helmet/Amulet `30ca`
producers and the existing copy program, then checks generated inputs and the
existing damage-table result. Keep the unresolved pre-Amulet aggregate `32e4`,
Partial contributor owners and complete-build refusal explicit. Do not allocate
a supported-quality channel without evidence of a real additional producer.

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
This public consumer contract is proposed, not approved or implemented.
It also records PoB's selected-group enabled/loadout bypass and default-skill
fallback. Preserve exact reference identities and prove selected/nonselected
controls; native query selection must not silently activate a skill.
The [bounded generated-source accounting follow-up](owned-configuration-dispositions-proposal.md#bounded-generated-source-accounting-follow-up-2026-10-05)
identifies a private next step: prove every field of each selected generated
Skill/Gem pair against its exact provider, actual emitted quality/count bindings,
reference correspondence and existing same-preset Pending usage. This could
remove six source-row links, not an issue or a gameplay requirement. Current
79 origins and all five selected issues are unchanged. Reuse existing proof
seams and reject a whole pair if any field or reference remains unaccounted;
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
   without unreviewed defaults. The configuration-role issue retains 79 origins
   after presentation/range accounting (237 in the original census), including
   global fallback records outside Config. Their complete
   disposition census and actual consumers must be proved; additional numeric
   controls alone cannot certify an empty scenario-usage inventory.

2. **Connect selected receiving and property contributors.** The two single-line
   Minion Damage nodes, four Life/Damage nodes and twelve flat/permanent reward
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
   The new original-source snapshot census is proved, but native aggregate
   authority still requires complete actual contributors and owner coverage
   before joining item arithmetic into source preparation.
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
   Modifier `30ca` also still needs the applicable Focus pre-skill route at
   `CalcSetup.lua:1473–1485`. Observe the late slot-copy delivery phase before
   deciding which prepared inputs it can affect; final ModDB records alone do
   not prove pre-support level contributions. These are actual remaining
   semantics, distinct from old gap labels that now overstate missing arithmetic.

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
   The current operations V20 Partial release does not weaken that gate. Establish real
   admission predicates and supporting mechanics for all six observed Ice support
   candidates; the finite arithmetic component's already-admitted positions are
   not that proof. Supported/final quality and broader ordinary property families
   remain explicit. Validate nonzero real item/support mutations, independent
   sources, exact requests and unchanged originals at the next checkpoint.

3. **Connect the remaining numerical consumers.** Real source-property
   integration is step 2. Existing minion-level item producers alone do not prove
   completeness of incoming properties or all final inputs.

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

**Pending:** [requested Skill participation](owned-skill-participation-proposal.md).
The recommended consumer contract preserves mechanical supply for preparation
and uses existing typed usage to gate execution and descendants. The owner has
been prompted; the generated-input approval above does not approve this separate
contract. No dependent Core/Data/Engine implementation has started. Independent
reward coverage and source evidence can continue while the decision is pending.

**Pending:** [explicit generated-source field accounting](owned-generated-skill-dispositions-proposal.md).
Recommend a V2 variant of the existing Import policy, with the same configured
fields and a separate private proof pass. Preserve historical V1 behavior and
actual successful raw/count outputs. The owner has been prompted; no dependent
public policy or source-accounting implementation has started.

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
| Historical replay compatibility | V19 intrinsic provenance deliberately changes old-policy source links. Ordinary breadth/extension assertions and the current five-original replay are updated. Seventeen older ignored publication harnesses need explicit exact-link compatibility before reuse; preserve their historical evidence and consolidate repeated checks rather than allowing arbitrary versions. [Retirement/migration inventory](legacy-retirement.md#historical-publication-replay-migration-2026-10-05). |
| Generated usage | Versioned preferences, all-record data-aware proofs and explicit applicability diagnostics pass focused validation. Shared exact-provider proof now serves raw quality and requested-count import independently. Same-definition allocations in different presets remain distinct. Enabled/global/reporting dispositions and usage inventory closure remain open. [Contract and gates](owned-generated-skill-usage-proposal.md). |
| Requested participation | [Public consumer proposal](owned-skill-participation-proposal.md) awaiting owner decision: keep mechanical supply separate from whole-Skill execution intent, using existing typed usage and one graph. Audit descendant/delivery/query gates and participation-dependent preparation. PoB selected-group preview bypasses/fallbacks require explicit reference controls, not native defaults. |
| Generated raw inputs | Accepted exact preset bindings now have V6 permissions, V19 producers, source-bound quality joins and checked publication for the reviewed Djinn/Firebolt families. Existing provider levels survive. Archived cross-axis correspondence, other generated families and final mechanics remain open. [Contract and gates](owned-generated-skill-inputs-proposal.md). |
| Generated source-format breadth | Replace finite Item-name frames with proved general derivation over admitted layouts, and add evidence for normalized-only level correspondence. Census competing saved representations before scalar admission. Original04's Firebolt provider still has unresolved rune/layout/grant-line authority; raw quality cannot bypass it. [Bounded adapter follow-up](legacy-retirement.md#bounded-generated-source-import-coverage-2026-10-05). |
| Configuration/source dispositions | Presentation and exact item-range ownership are implemented; full semantic closure is not. Original05 issue `01f2` retains 35 Config-local and 44 non-Config origins. Account for generated-group fields and remaining actual consumers before retirement; partial scalar success or local Config proof cannot clear global fallback. Keep scenario usage/assumptions separate. [Evidence and gates](owned-configuration-dispositions-proposal.md). |
| Global-switch source reachability | Declared-stat and two actual-parser negative controls pass cold/lifecycle/repeat and JIT checks with exact generated/manual occurrence identity. Independent review and execution disproved the suggested missing-name alias. Full `ExtraSkillStat` producer/filter/transform reach remains unproved; no field non-applicability certificate or inventory closure is published. |
| Reward contributor coverage | Three additional selected producer inventories and four typed Actor channels are published and tested, including both Charm effects. Receiving and final threshold/recovery/capacity formulas remain open. Continue actual selected owner/contributor gaps; no whole-build closure follows from these producer programs. |
| Selected passive receiving | Growing Swarm's complete default two-effect owner and Sniper receiving are published through existing contracts. Conditional/unconditional cooldown sums stay distinct; final duration, area/radius, other minion receivers and complete contributor coverage remain open. [Packet and limits](../data/owned/poe2/3887ae68/growing-swarm/README.md). |
| Selected support delivery | Magnified Area I/II contributions and actual per-Action Area eligibility are published and validated for fourteen contexts, with scratch/Rayon isolation. Rapid Casting I/II now publishes cast-speed contributions and passes both-stat-set/independent-root checks using the same contracts. Encroaching now delivers factor 1.1 on existing channel 32fa with exact mixed-family source/native checks; ground growth and source subtotal-rounding authority remain unresolved. Prolonged I/II now supplies duration/cost factors on real Offering and Ice actions; final inputs, final timing and application-lifetime transfer remain open. Primary Summon delivery, final radius/resource costs and complete owner/receiving inventories remain open. Keep ordinary resource-cost factors separate from reservation factors. [Delivery](../data/owned/poe2/3887ae68/magnified-area-support-delivery/README.md) and [eligibility](../data/owned/poe2/3887ae68/action-area-eligibility/README.md). |
| Support catalogue/supply boundary | Corrected one offline producer and its validators; published 568 Known Gem replacements from a full 966-Gem census. Primary associations stay in Import, non-support candidates and Partial closures survive. Bidding consumes actual corrected data; the historical Ice Nova component retains named finite isolation until rebased. Hidden-helper semantics and complete genuine supply remain open. [Retirement/integration gate](legacy-retirement.md#support-catalogue-entries-versus-executable-supply-2026-10-05). |
| Exact Direct support targets | Shared V2 source proof resolves 40 saved targets/eleven order sequences, retiring eight selected Original01 and six selected Original05 issues. Absent source is required; explicit-empty raw records stay Pending. Input values, usage, activation, full origin inventory and numerical readiness remain independent. [Evidence](owned-djinn-provider-evidence.md#exact-manual-support-targets). |
| Socket configurations | Separate descriptors/configurations/copies accepted. Implement identity, persistence, per-host projection, migration and exact local receiving; independently prove host-local aggregation before rounding, saved-effect reconciliation exactly once, physical-copy/current-setup feasibility and locks. Unused item setups must survive. [Contract and gates](owned-socket-configurations.md). |
| Determinism/source exception | Strict per-candidate determinism required. Frost initialization behavior is an upstream bug with a narrow excluded comparison. All five unchanged originals pass independent fresh replay across both JIT modes; no warm/retry canonical protocol adopted. |
| Legacy and resistance cleanup | Orphaned Import equipment/Mace closure removed; 175 Import library tests and strict Clippy pass. Fixed Elemental still needs canonical raw migration, and Engine profile adapters/default legacy dependencies remain. Follow the [retirement inventory](legacy-retirement.md). |
| Lua behavior/type audit | Requested and planned: classify source-language behavior by game/format/reference purpose and valid-input impact, then isolate or remove it. Preserve justified numerical laws; discuss uncertain semantic changes. |
| Attribute grouping and source caches | Finite-stage architecture accepted; ordered attribute groups and numerical integration unfinished. The synthetic 1.02/1.0201 setup discrepancy needs legal-input reachability and a reviewed domain law. Earlier cache-mode options are historical; no native cache-dependent mode or extension of the Frost exception is approved. [Evidence](owned-attribute-setup-evidence.md). |
| Reference reporting order | PoB `Build.lua:2393` mutates the calculated `SkillDPS` array into display order. Record exact row identity across that reorder in the optional witness; preserve calculation contribution identity and keep presentation sorting outside native numerical semantics. [Reporting evidence](owned-full-dps-aggregation-proposal.md). |
| CI regression, timeout and infrastructure | V18 and the later stale V20 future-version assertion are corrected; the earlier six-hour combined Windows compile is replaced by five test suites on both OSes. [Run 37325666494](https://github.com/Azaril/poe-optimizer/actions/runs/37325666494) at `167aa1aa` passed all twelve validation/test jobs, with its final aggregate jobs failing hosted-runner acquisition. The `e613014` run exposed the V20 assertion, fixed in `53a719b`, and a later Windows Import runner lost communication without a test exit. Latest checked `f11a57c` run 37379670680 is pending behind the older live run; exact pushed-head hosted success remains open. Infrastructure annotations do not justify dropping coverage or changing tests. Retain every package/target and required aggregate check. |
| Source validation diagnostics | Accumulate bounded per-original failures and run both JIT modes before failing overall; the successful current run skipped no comparisons. Investigate the optional Windows PoB test's `LNK4098` CRT link warning as reference-runtime packaging debt. |
| Authoring/execution simplification | Retain [A1-A4](rule-execution-model-investigation.md): measure maintenance/update cost and real evaluation workloads before choosing compact DSL, declarative, injected-native or compiled execution changes. Work inside the accepted owned boundary; no revival of Lua/UI emulation. |

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
