# Architecture migration and retirement plan

Status: active delivery plan, updated 2026-09-24. The owner requested this correction before
further PoB-shaped runtime work. [Domain architecture](domain-architecture.md) is the
controlling design. [Implementation](implementation.md) records actual completion and
validation; each phase's complete exit gate remains separate from its delivered APIs.

## Starting point and immediate rule

Five originals import and evaluate in PoB; **0/5** complete natively. Existing native
calculation dispatch is Spark/Mace, including the apparent general search-build wrapper.
Injected data, generic search/objectives, numerical kernels and independent reference
fixtures remain useful. Source-shaped loader/control/program machinery is substantial.
The newly staged Common.new/BuildModList experiment is paused, isolated and not adopted.

Do not continue native PoB UI construction, loadout notification replay or general Lua
compatibility merely to advance a source-method frontier. Source audits may inform import
mappings and game semantics; they do not determine the runtime's object model. No new
Spark/Mace/third-skill profile adapters. New production work must have a named consumer
in the target semantic path and a corresponding obsolete-path retirement decision.

## Phases and exit gates

| Phase | Implementation | Exit evidence |
| --- | --- | --- |
| D0: boundary and retirement inventory | Make the owned-model ADR authoritative; supersede conflicting resume/whole-method parity gates. Map existing code to reuse, adapter-only, temporary legacy, or delete. Remove unneeded facades now. | Reviewed contracts and concrete dependency inventory. CI remains meaningful. Frozen source probes are explicitly paused; no imported state is relabelled a complete native build. |
| D1: owned semantic input contract | Implement BuildProject/BuildSpec, ScenarioSpec, semantic selectors and normalized instances without mandatory XML. Decode/encode the owned format; import PoB through a separate adapter with origins. Remove old source-document requirements from the new evaluation request. | All five originals can be represented through the adapter with unresolved semantics explicit. Directly authored input uses the same APIs with no PoB checkout. Duplicate items/skills, grants, independent views and point pools have contrasting tests. No numerical completion claim. |
| D2: offline semantic package/compiler | Add a separately versioned package for definitions and domain effects. Map existing trustworthy catalogs/numerical data first; convert representative conditional, support, grant and local-item effects to the owned IR. Keep unsupported conversion inventory. | Reproducible generated package from the pinned source, successful project-authored data injection and a data-only balance change. The new semantic package and load/evaluate path exclude PoB/VM/source programs; complete legacy/default dependency retirement is D5. Compiler rejects unknown operations and invalid scopes/dependencies. Source edits affect conversion tooling, not runtime schema by default. |
| D3: general native resolution and evaluation | Resolve actors/actions/providers, passives/items, supports, conditions and resources into a shared plan. Reuse numerical kernels after removing profile assumptions. Develop player and minion paths together, preserving all five model cases. | First complete declared Twister and Sniper evaluations with fixed metric/availability manifests, then remaining originals. Bossing and explicit mapping scenarios, invalid-but-computable cases and missing mechanics remain classified. No NativeInput::SkillName switch or hidden fixture values. |
| D4: semantic candidate/search integration | Bind generic search to BuildSpec edits and the shared evaluator; preserve all six dimensions, exact 1..N skill/item requirements, scenarios and configurable objectives. Move imports/exports outside repeated evaluation. | Controlled class, equipment, tree, support and provider changes agree with fresh reference evaluation within declared coverage. Serial/Rayon and fresh/reused A→B→A agree. Budget/cancellation/cache identity tested. Remove old experimental/profile search routes with this integration. |
| D5: complete retirement and breadth | Delete remaining old profile/parser/UI runtime paths after their useful kernel/tests are migrated or intentionally retired. Keep only reference-side compatibility tools that still have named consumers. Add independent whole-build holdouts and seeded-jewel investigation under semantic transforms. | Old modules/data/CLI schemas/fixtures/support dependencies classified and retired; no dual default pipelines. Native/browser dependency and artifact checks pass. Complete original-build and holdout coverage reported separately from component tests. |
| D6: performance and applications | Measure general evaluation/plan reuse/search; optimize compact rule execution and dependency invalidation. Deliver shared report/events, later CLI/GUI/web presentation and indexes. | Real admitted builds, changed data and interacting candidates establish throughput/memory on multicore machines. UI has no independent game logic. No speed claim from a narrow legacy profile. |

D1/D2 can proceed in parallel after agreeing IDs, effects and coverage contracts. D3 can
start with validated slices of D2; it need not wait for every game definition. D4 should
exercise the model early with small legal candidates, but a passing search smoke test is
not completion of joint optimization. D5 retirement happens throughout D1–D4, not as a
permission to keep obsolete code indefinitely. Avoid a flag day that deletes useful
numerical validation before its meaning is preserved.

## T1: Rust tooling and test consolidation

The existing Python suites test early offline exporters, corpus intake and parity expectation
utilities. They are not a native evaluator runtime dependency. Rust is the intended home for
project-maintained unit, integration, property, parity and tooling tests.

Status: planned, not started. The owner requests eventual Rust-only project tooling/tests
and explicitly defers conversion of the existing Python tests. Use Rust for new project
tooling and tests; maintenance of existing Python utilities and their tests remains permitted.
Schedule coherent replacements alongside D2/D5 consumer migration without displacing the
five-original-build integration priority.

Inventory the maintained exporters, corpus runner, expectation checker and build/boundary
checks with their CI consumers. Replace each utility and its tests together using owned
Rust types/validators, retaining independent expected results. Preserve artifact bytes and
semantic identities where they are contractual, malformed-input/resource limits, and
no-overwrite/publication behavior. A Rust harness that still requires the Python utility
is an intermediate step only. Choose the Cargo tooling entry point when implementing the
first replacement; this decision does not introduce another rule interpreter.

Exit: supported build, data-generation and validation workflows run without Python;
project-maintained test suites are Rust and run through documented Cargo commands. Remove
each Python implementation, test suite and CI setup only after its last consumer migrates
and equivalent behavior is verified. Optional upstream PoB/Lua oracle tests stay upstream.

## Mixed passive provider checkpoint

The next data-only slice supplies 251 complete unconditional defence/recovery passive lists,
including their mixed attribute effects. Existing extension and full-list compiler APIs
publish typed owned channels and 413 contributions; no source-shaped evaluator contract is
introduced. The shared all-original preservation harness covers both attribute and defence
policies. These intrinsic providers do not complete allocation access or receiving defensive
calculations. See the [data contract](../data/owned/poe2/3887ae68/passive-defence-inputs/README.md).

## Raw defensive equipment inputs

The optional source adapter acquires final constructed base-profile values once into a
finite owned catalog. The native converter consumes only that catalog, exact base identity
and injected field/type/absence policy. Weapon and defence adapters share one bounded
literal compiler while retaining separate source DTOs and policy/program identities.
This avoids another evaluator or a second copy of the numerical validation machinery.

Whole-profile absence, a present empty profile, missing individual fields and authored
zero are distinct. EquipmentUse inputs retain resource points, ratings, coefficients and
explicit optional-field presence. These facts do not select the active equipment branch,
apply quality/local/per-level modifiers, or establish complete item/actor defences. Further
composition uses ordinary owned recipes and explicit activation, never runtime source
lookup. The [data contract](../data/owned/poe2/3887ae68/defence-profiles/README.md) records
bindings and the remaining components. All five originals and their 110 queries remain
required; raw profile coverage alone cannot complete an original.

## Defensive authored-quality checkpoint

The [quality extension](../data/owned/poe2/3887ae68/item-quality-inputs/README.md) closes
32 actual input obligations across the five originals, including every one of their
21 selected physical defensive items. It adds only standard-quality allowed membership
and the existing direct EquipmentUse quality projection to 1,240 finite reviewed
base templates. There is no new native mechanism, default or source-format coupling.
Quality remains authored input; effective crafted/alternate quality and final item
assembly remain distinct work. All previous Partial closures stay Partial.

This does not remove the larger D1/D3 gates: most Gem identities still have Unmapped
provider schemas, supports are explicitly unsupported by the effect-plan compiler,
and final metric availability currently inherits whole-plan completeness. Further
scalar families alone cannot produce a complete original. Prioritize the pending
support-origin/selection, ordered contribution and metric-coverage decisions, then
integrate real providers and final calculations. A separate source-proved Gem provider
schema conversion can address breadth of inputs; it must preserve incomplete mechanics
and unresolved grants instead of declaring every catalog identity complete. Catalog
identity/effect membership is insufficient evidence for empty intrinsic parameters:
corrupted state and corruption-level delta also need explicit owned input definitions.
The five originals' 478 materialized physical gems have source false/zero values, so
there is no observed corruption regression in them; edited/nonzero builds must remain
part of the conversion's contrasting tests. Keep Unmapped-to-Known knowledge refinement
separate from the existing monotonic Known-schema membership contract.

## Staged contribution design decision

The next D3 attribute milestone needs explicit ordered contributor groups as well as finite
stage snapshots. Existing rules can lower the two passes into an acyclic graph, but ungrouped
Sum/Product and aggregated Integer-to-Count conversion do not preserve all source behavior.
Complete-function reference tests now demonstrate sequential reads, comparison refreshes,
zero-base laziness, rounding/group boundaries and inherent bonus controls.

Review [the concrete proposal](owned-contribution-stages.md) before dependent shared-contract
changes. The preferred option lowers finite stages into the current graph and adds bounded
ordered membership/group semantics; a stage-aware runtime is the larger alternative. Neither
is implemented or accepted here. Independently convert fully reviewed ordinary passive
providers through the existing data compiler. Keep all original coverage gates intact.

The [verified setup audit](owned-attribute-setup-evidence.md) now demonstrates a real
fresh/cached grouping difference using explicit source-only inputs (1.02 versus 1.0201).
The owner is also choosing the authoritative reference path or explicit compatibility
modes. Fresh evaluation is recommended but not approved. Output-reset, condition fallback
and candidate invalidation evidence informs the pending contract; it does not authorize
native cache-dependent results or replace the staged-architecture decision.

## Item contributions to shared actor attributes

Four injected programs connect the existing formatted item attribute outputs to the
three existing Player Integer contribution channels. This closes the item-to-actor
numerical dependency without a Core, routing, schema or opcode change. All Attributes
retains one provider occurrence and emits three effects. The precision-zero formatter
contract makes the Count-to-Integer conversion exact; arbitrary fractional attribute
producers require a separately reviewed representation. See the
[owned attribute design](owned-attributes.md) and
[data contract](../data/owned/poe2/3887ae68/actor-attribute-inputs/README.md).

This component leaves the four owner closures Partial and all five originals incomplete.
Next implement the explicitly staged attribute calculation, condition snapshots and
inherent resource/accuracy bonuses through the existing owned rule model, retaining
contributor membership and activation gates. Final resource metrics require their own
conversion, override and modifier stages; do not publish baseline-only values as totals.

The [support proposal](owned-support-activation.md) now separates ordered selection from
bounded type preparation and final applicability. Original selection can retain two
positions referencing one support, so a prepared application needs position identity in
addition to its physical origin and receiver. The compatibility-versus-correction policy
remains an owner decision before dependent shared-contract implementation.

## Authored skill scope conversion

The optional import `SkillScopePolicy` converts exact reviewed parent-slot syntax into
an owned loadout scope after existing manual/physical/SkillUse admission. The injected
policy admits only a missing slot. Global-effect switches, enabled state, support
applicability and selected source UI groups are independent. Omitted policies retain old
canonical bytes and Pending behavior; named/empty/generated cases remain unresolved under
this policy. The existing Core/evaluator contract is unchanged. See
[the scope contract](owned-skill-scopes.md) and the current [checkpoint](implementation.md).

The five originals have 140 eligible authored scopes (9/63/9/13/46). This component conversion
preserves gem inputs, item obligations and all 110 queries; it does not establish whole-build
parity. Publish an explicitly authored normalization input using the existing compact
successor API, validating the old tree first and installing unchanged content under the
new binding. No new runtime source loader or publication subsystem is needed. Support,
socket configuration and metric coverage decisions remain separate pending owner input.

## Schema-proved gem inputs and support reference checkpoint

The historical implicit empty-schema inference is superseded by explicit schema-bound
Gem input recipes. Fresh v12 normalization requires source guards/conversion even for an
empty Complete parameter declaration. The reviewed neutral policy admits 12 original
collections; policies without recipes leave all 478 Pending. Prior serialized policy bytes
remain compatible. Typed nonempty recipes and a separate V4 physical-Gem knowledge migration
prepare broader conversion without changing Core/routing/evaluator contracts. See the
[input contract](owned-gem-inputs.md). Complete native originals remain **0/5**.

The finite physical-Gem compiler now prepares V4 migrations and typed input recipes from
an injected identity catalog and reviewed policy. The first family covers 514 support
identities, retains fractional corruption deltas, and leaves all new membership closures
Partial. Publication uses the existing checked schema and normalization commands; no
source catalog, lexical recipe or PoB runtime enters evaluation. See the
[support Gem data workflow](../data/owned/poe2/3887ae68/support-gem-inputs/README.md).

Recipe-level numeric token aliases now permit explicit finite source spellings without
changing direct codecs or the evaluator. Each replacement is decoded once through the
same typed numeric codec and validated against the owning Gem slot. The reviewed support
successor maps only selected literal `nil` deltas to zero. Empty tables preserve historical
policy bytes; populated tables bind a new normalization identity. Missing inputs and
unreviewed text remain unresolved, and no parameter collection closes because of an alias.
See [the input contract](owned-gem-inputs.md#explicit-numeric-token-aliases).

The explicit multi-effect compiler mode retains resolved potential Skills on one physical
Gem and keeps primary role separate from display ordering. It accepts source-generated
extras only with complete set agreement and exact owned joins. This is an import-only
refinement over the existing Gem schema; it does not introduce active SkillUses, providers
or a runtime source catalog. Physical Gem levels do not constrain all derived active-effect
levels: reference preparation can inherit a supported action's level. See the
[multi-effect input boundary](owned-gem-inputs.md#multiple-potential-effects-on-one-physical-gem).

### Full data-release assembly before further active-Gem integration

Delivered: the reusable Rust full-release assembler and thin `assemble-owned-release` CLI
validate explicit persisted inputs or checked old/new packages, compile every constituent
and publish immutable artifacts. Normal assembly requires exact supplied bindings. The
separate revision operation verifies a prior full-input commitment, applies reviewed
existing-address descriptors, explicitly rebinds the endpoint and records provenance.
See the [release API and publication contract](owned-releases.md).

The tracked Twister/Sniper correction replaces prematurely Complete-empty direct parameter
and choice declarations with accurate Partial knowledge in a new release. It preserves all
IDs and the registry watermark, known members, source evidence, programs and 110 queries.
Old releases retain their meaning. Complete-preserving V3 and Known-preserving V4 are
unchanged; neither API accepts a 'skip preservation' option. Runtime still consumes owned
registry/schema/rules/routing artifacts, with no new Core type or evaluator operation.

The shared checked loader accepts releases for existing offline compilers and verifies all
artifact bytes, exact bindings and receipt reconstruction. Query-set and row order remain
semantic. Bootstrap from the checked current package avoids shifting historical IDs;
explicit tracked correction inputs prevent rebuilding the earlier coverage mistake.
Standalone provenance is declared authoring history, not authenticated ancestry. Retain
immutable parents and revision policies when old successor formats are used downstream.

Final gate evidence is recorded in the [implementation checkpoint](implementation.md):
reproducible publication; unchanged older bytes and identities; stale/mixed inputs rejected;
all-five normalization with only the declared coverage corrections; native dependency and
WASM checks. The corrected release now precedes further active-Gem conversion. Migrate
consumers before retiring historical build orchestration, and retain independent V3/V4
contract tests. This delivers the release mechanism, not complete D2 or build parity.

Active-Gem follow-through covers the remaining 53 definitions / 128 materialized occurrences.
Of these, 36 definitions / 77 occurrences already fit the compiler's singleton or resolved
multi-effect modes. Review physical level-key domains, natural maximum and quality input
semantics using complete source loading/validation and independently authored accepted
ranges. Additional stat sets (12 definitions / 34 occurrences), unresolved command effects
(five / 17), and minion selections need explicit representation; missing references cannot
be fabricated or suppressed. Prepare Twister's selected action and Sniper's actor/action
together through existing owned seams. This phase is incomplete until the new data is used
by the general path; producing another isolated scalar test is not its exit gate.

D3 still needs selected support applications, provider ownership and contribution stages.
Input conversion is not evidence that any support effect or original build evaluates.
D3 support reference work now executes actual upstream preparation and modifier merging in
four optional Rust tests. Real supports can add types that enable or exclude other supports,
so the proposed design needs a bounded type-preparation stage before final applicability.
This is distinct from cyclic numerical rules. A synthetic source-input regression also
proves an order-sensitive hole in the pinned source retry algorithm. The
[support proposal](owned-support-activation.md#support-interaction-evidence-and-pending-parity-policy)
records the evidence, coverage limits and pending decision between isolated versioned
reference-compatible behavior and corrected behavior with an explicit parity difference.
No native support relation is admitted before that contract is agreed.

A parallel CI audit identified two old tests that still rejected operations v10 after it
became the current supported version. Both now test current-version acceptance and reject
a stable unknown sentinel; production semantics and historical compatibility are unchanged.
This repair is independent of support design. New tests/tooling remain Rust under T1.

## Rarity/Chaos inputs and the next integration milestone

D2 adds two distinct ordinary item families through existing owned contracts. Rarity keeps
magnitude and direction separate; Chaos Resistance uses a direct signed quantity. Both
retain required inputs, explicit properties, occurrence identity and Partial coverage. No
native runtime opcode, source interpreter or Actor contribution is introduced. See the
[data contract](../data/owned/poe2/3887ae68/item-rarity-chaos-inputs/README.md).
The five originals retain 64 admitted raw modifiers, 53 resolved display observations and
all 110 queries. Complete native evaluation remains **0/5**; this is component progress.

The next D3 milestone is support receiving/applicability/activation, following the
[proposed contract](owned-support-activation.md). The engine currently rejects every support
assignment. Removing that rejection alone cannot establish correct origin/receiver binding.
Owner review is pending before changing the shared rule/routing contracts. Once agreed,
exercise player and minion targets together, preserve support-owned inputs, explicitly
select generated receivers, and contrast false/unknown applicability, duplicate assignments,
missing inputs and sibling non-propagation. Follow with a source-verified injected support
from the originals. Gem parameter collections, skill scopes, selected preset membership,
allocation access and whole-build contributions remain independent integration gates.

### Finite membership representation investigation

Before dozens more broadly assigned families, measure and propose shared immutable finite
membership sets or equivalent factoring. Compact offline authoring still expands current
per-template descriptors to 9,516,263 of 16,777,216 allowed bytes. A broad family repeats
1,756 IDs (approximately 189,648 bytes); 38 more is only an identifier-only upper bound.
One-family batches do not avoid accumulated descriptor growth.

Exit evidence: equivalent memberships and Partial/Complete closure, deterministic bounded
lookup and canonical identities, reproducible artifacts, explicit migration/versioning,
load-time/memory/lookup measurements, and rejection of stale or invalid references. Structural
membership remains separate from roll legality. Discuss the format proposal before adopting
it; neither wildcard membership nor blindly raising caps fulfills this milestone. Existing
T1 Python suites remain unchanged; all new implementation and tests are Rust.

## Ordinary item attributes and compact authoring checkpoint

D2 adds four flat attribute owners using the existing owned schema and numeric compiler.
A Count unit separates attribute components from catalyst percentages and resource values.
All Attributes stays one physical occurrence with one amount; four PoB stat records are
source witnesses, not four independent rolled modifiers or a fourth Actor attribute.
Item/gem requirements and derived resources remain separate. No native rule opcode or
source interpreter changes. Partial eligibility, activation, collections and numerical
coverage remain explicit; Actor Integer contributions require later reviewed projection.
See the [data contract](../data/owned/poe2/3887ae68/item-attribute-inputs/README.md).

A finite [offline membership patch](owned-recipe-membership.md) applies the four families
to 1,756 explicit templates without copying whole descriptors in authoring inputs. It
retains V1 extensions, V3 membership checks, exact prior bindings and materialized runtime
schemas. Existing Python utilities/tests remain unchanged under T1; all new compiler and
source/native tests are Rust. This is component progress, not complete original-build parity.

## Elemental-resistance breadth checkpoint

D2 adds distinct Fire/Lightning raw modifier owners through existing owned contracts: four
fixed grammars, 46 required slots, eight shared numeric programs and finite membership in
all 1,756 base templates. Only reviewed plus-integer, no-tag/no-generated-prefix source
members are admitted. No runtime interfaces or source lifecycle machinery are added; prior
programs and all Partial eligibility/collection coverage remain. See the
[data contract](../data/owned/poe2/3887ae68/elemental-resistance-inputs/README.md).

Original admissions increase from 10 to 34 (19 Fire/Lightning and five newly independent
Cold followers), preserving 53 display observations and all 110 query rows. All five builds
remain incomplete. Next, ordinary attributes offer breadth across all five originals and
precede eight unresolved resistance rows. Compound all-attributes meaning must preserve one
source occurrence with several semantic effects. This is data/import progress toward D3;
whole-item, source order, allocations, actor resolution and numerical parity remain gates.
Pending socket/coverage design choices still constrain their dependent model changes.

## Derived-display observation checkpoint

D2 now compiles six reviewed numeric display grammars against a separately authenticated,
finite base catalog. Owned source policy v7 binds them to explicit template sets and admits
only proved fresh preamble positions. Actual item inputs remain distinct; no displayed
total becomes a numerical base roll. The optional exporter retains source-format knowledge,
while the native compiler/runtime consume owned data without PoB UI or source execution.
See [the contract](owned-display-observations.md).

All 1,756 constructed bases are covered by acquisition checks. The supplied originals now
admit 53/54 display rows; the unresolved decorated magic base remains pending. All 10 prior
modifier facts and all 110 query rows are preserved. Four Corrupted markers still have
possible preceding-line consumption. This is import progress: complete native originals
remain **0/5**, with collection/order/allocation/numerical gates unchanged.

The next work follows the fresh inventory: 43 pending Fire/Lightning resistance rows span
all five originals; 14 plain rows have no immediate combination blocker. These are candidates
for independently reviewed membership and numeric data, not a parity claim. Keep true corruption state
and general base-name grammar separate from metadata. Socket-configuration and per-metric
coverage proposals remain pending owner decisions. All new work is Rust; existing Python
utilities/tests remain under T1.

## Source-role migration checkpoint

D2 replaces the final 29 unconditional member-role declarations in the current successor
with Unresolved fallback roles and 27 reviewed conditional alternatives. The earlier cold
consumer remains, giving 28 guarded rules over the constructed catalog. Raw sign, bounded
captures, source flags and initial catalyst context now constrain these assertions.
Two bare direct-resistance ranges remain unresolved. The injected rule, schema, registry,
numerical program and receiver artifacts do not change.

The import-only vocabulary adds exact-sign DecimalCapture, NoSourceScalingTags and
InitialScalingIsOne. The last can prove a fresh source prefix contains no catalyst setter;
it never consults owned defaults. V6 globally declines GGG markup after reference evidence
showed escaped advanced headers can alter tags on later lines. Constructor validation now
uses bounded sorted lookup indexes under the unchanged resource cap.

**Follow-up at this checkpoint (display portion delivered above):** prove template-specific meaning for 54 display-header candidates separately from
four actual Corrupted inputs. Acquire finite compatible-base evidence and bind preamble
observations to reviewed owned templates; class names alone do not prove recomputation.
Displayed totals must not become owned base rolls. Corrupted needs true Boolean input and
trailing-marker attribution, preserving all four actual predecessor ambiguities.
Retain all-five/110-query, collection, allocation and reduction-order gates. Broader signed
and scaled modifier domains need their own evidence; the two scoped-coverage/socket model
proposals still require owner input before dependent changes.

## Conditional source-membership checkpoint

D2 now implements the [bounded import contract](owned-source-conditions.md). Source-policy
v6 carries typed all-of prerequisites; successful conditional membership does not change
fallback roles or numerical coverage. Exact raw capture spelling, physical tags and a
structurally selected no-generated-prefix template are independently checked. Constructor,
encoder and runtime budgets apply before new work; old wire domains stay intact. A failed
condition cannot be bypassed through raw-text lookup, and v6 never publishes an unproved
non-header modifier merely because its recipe does not request property inputs.

The first reviewed cold consumer recovers three original raw modifiers while all five
builds remain Pending. Source-generated Sapphire Charm +25% demonstrates why fixed lines need
a prefix proof. The reference harness executes complete ParseRaw, including nested assembly,
but separates initial membership observations from final-value authority.

**Delivered in the successor above:** context-conditioned proofs replace the remaining
existing SingleModifier rules in the current package.
Pinned source evidence confirms partial parses for bare Life/resistance/critical text,
plus-ranges resolving to zero, huge finite range endpoints and negative catalyst contexts.
Use data-authored lexical bounds/context prerequisites and source range evidence; extend
the vocabulary only when a named consumer needs a bounded fact that it cannot express.
Migrate through successors, retain original query rows and compare exact original admissions.
Do not mistake source-family recognition, a partial cached source row or a green component
test for final-value or full-build parity. Continue derived-header/Corrupted work afterward.

## Canonical cold-family checkpoint

D2 now carries canonical raw cold owner2542 across all 1,756 constructed base templates,
with 1,755 membership-only refinements and every Partial closure retained. Fixed signed
recipes emit the canonical 23 inputs; the old09d6 direct-program consumer is retired from
fresh fixed import without reinterpreting its persisted IDs. Structural permission does not
imply affix legality, source attribution or final numerical applicability.

Pinned source tests disproved unconditional fixed-line membership under signed-zero,
scientific formatting and catalyst contexts. Both fixed source roles remain Unresolved.
The generic adapter also withholds membership proof for ValueOutsideSchema. Seven original
modifiers remain admitted; zero additional full items/builds are complete. All 22 fixed-cold
lines and 110 query rows remain. See the current implementation checkpoint and
[cold-family data contract](../data/owned/poe2/3887ae68/cold-family-inputs/README.md).

## Context-conditioned source membership

First consumer delivered above; retain the following contract for further D2 coverage.
Separate a raw semantic family conversion from a reviewed
proof of physical source members. A source rule's lexical match is insufficient when source
formatting depends on item state and may combine following physical lines. Keep this
compatibility contract in the importer/offline tooling; do not reconstruct PoB formatting,
UI notifications or parser execution in the native evaluator.

Design an injected, bounded prerequisite model for a rule's source role. Bind the exact
line-policy/schema identity, lexical restrictions and source-context facts used by the
proof; missing, duplicate, unsupported or invalid facts withhold that proof. Source facts
and constraints must be explicit, not inferred from English text or item-category switches.
Keep raw conversion, source member indexing, final value eligibility and crafting legality
as distinct results. Use typed facts/guards with a small reviewed interpretation; do not
introduce a general source-program interpreter or a skill-specific exception list.

First consumer: plain untagged positive integer cold lines with bounded digit-led spelling.
Verify the source's explicit empty-modTags scalar-one path independently of catalyst header
values, and reject flags/contexts that bypass it. This can recover the three real records
without admitting every tagged or decimal case. Extend to other spellings/contexts only
with separate proof. Audit existing SingleModifier roles for the same contextual assumption.
Do not weaken the current unresolved roles simply because individual fixtures look safe.

Exit: data-authored positive and contrasting cases prove source member counts and boundaries;
negative zero, large formatting, leading-dot decimals, matched/unmatched catalyst contexts,
negative quality, unknown context, overlays and following-line combinations retain correct
classification. Existing source blockers persist. All five original builds and all 110
query rows remain, and newly imported raw records are not counted as full-build parity.
After this contract, return to derived header classification and actual Corrupted inputs.

## Exact coverage diagnostics checkpoint

D3 retains the already bounded DefinitionBindingReport in the immutable effect plan and
exposes it by reference. The effects/metrics CLI envelopes advance to version 2 and include
exact binding sites, issue classes/codes/subjects and ordered query statuses. The aggregate
SchemaUnresolved gap, numerical completeness boolean, identities and worker evaluation path
are unchanged. No per-evaluation diagnostic rebinding or full-report cloning is introduced.

The version-2 draft CLI report groups exact issues by owned top-level row and code, separately
for the entire draft and an explicit selection. Unselected/global uncertainty remains saved;
only Core finalization determines selected blockers. These are presentation indexes, not new
Core input fields or a partial-request evaluation path. Ten all-five diagnostic selection
probes preserve 22 queries each and remain Pending. The probes select explicitly authored
first-known preset combinations under each known loadout; they do not recover PoB saved
selection or establish legal variants. Selected counts are 321/136/317/384/114, compared with
all-draft counts 326/920/322/389/920. This proves scoping, not native build completion.

The [coverage proposal](owned-coverage.md) requires reviewed finite possible-effect bounds
before excluding incomplete mechanics from a metric's dependency scope. Its numerical
contract awaits owner input. Unknown reach retains current broad blocking. Diagnostics
retention does not authorize that change or evaluating Pending drafts.

**Historical next priority (superseded by the cold-family checkpoint above):** repair stale fixed-cold conversion and generic semantic-family
membership before another header-only admission slice. The current fixed grammar still uses
early owner09d6; canonical raw cold owner2542 and the old cold owners are permitted only on
Sapphire Ring09dc. Three real lines (Frayed Shoes +10, Iron Ring +8, Gold Ring +34) have no
source blockers but fail that membership gate; 19 other fixed-cold candidates retain source
blockers. Convert the grammar to canonical raw inputs and generate reviewed family membership
from finite base capabilities, preserving Partial collection/eligibility and missing numeric
inputs. Do not broadly enable the old direct contribution owner or add fixture whitelists.
Structural modifier membership does not imply affix generation or crafting legality.

The separate header audit found 54 derived display lines and four Corrupted markers across
44 of 116 items. Defence/Spirit/Charm displays require template-conditioned observation
classification; their numbers must not become added rolls or authoritative assembled totals.
Corrupted is a real semantic input, and all four occurrences retain predecessor-combination
ambiguity. Neither header recognition nor family membership proves item completion. Keep
source-order, modifier membership, socket configuration and all-five/110-query gates intact.

## Metadata preamble checkpoint

D2's source policy v5 explicitly classifies reviewed zero-member preamble rules. The
native adapter validates referenced Header recipes and preserves raw occurrences; owned
builds, numerical packages and inventory identity gain no PoB metadata field. Existing
v3/v4 policy bytes/domains and sidecar v11 remain unchanged. The first supplied recipe
covers Unique ID; no arbitrary Header rule gains zero-member authority.

The source audit separates fresh XML import from later quality normalization and identity
reconciliation. Selection/group/version tags can act before header interpretation, and
GGG markup can conceal them. Explicit unsupported-control gates preserve these boundaries
without introducing source text rewriting or a UI lifecycle into the native model.

All 97 original Unique ID lines are preserved and match the new recipe; 52 normalize as
known metadata while other source lifecycles keep the remaining records Pending. Unknown
header blockers clear on four items, but no additional original modifier is admitted.
Gameplay headers, source ordering and unresolved numerical contributions still need work.
All five drafts and 110 query rows remain; zero-member evidence is not build completion.

The coverage audit confirms DraftSession already excludes unrelated stash and unselected
preset uncertainty. Selected authored rows, including disabled/off-loadout members, must
still resolve under the current complete-input contract. The Engine is more conservative:
one plan-wide completeness flag withholds all final values and metrics when any applicable
provider/receiver/program discovery is incomplete. This is deliberate protection against
unknown effects, not evidence that every catalog row or inventory item must be implemented.

**Next checkpoint: coverage diagnostics and contract review.** Preserve exact binding issue
sites and report selected-draft blockers by owned entity before changing numerical gates.
Design occurrence- and channel-scoped coverage obligations for complete requests, with
reviewed bounds on what missing effects may affect. A walk over implemented programs alone
cannot prove omitted contributors irrelevant. Unknown effect reach must retain broad
blocking; reductions need closed membership and transforms need proven semantic order.
Coverage identities must bind request, definitions, rules, routing and requested metrics.
Test unrelated gaps, unknown reach, activation, empty versus unknown membership, ordering,
and edits that invalidate prior proofs. Keep every requested metric row and separate
numerical coverage, legality and oracle agreement.

This review does not authorize evaluating Pending drafts or dropping selected disabled
rows. Either change requires an explicit design discussion. Scoped numerical coverage alone
will not make the current originals evaluable: they still lack complete selected inputs.
Convert meaningful headers and item eligibility using finite semantic facts, retaining the
pending socket-configuration decision before its dependent Core change. The contribution
reduction-order parity question also remains open.

## Generated-prefix evidence and elemental endpoint checkpoint

D2 now has a finite, independently exported constructed-base layout catalog. A native
compiler binds it to the prior package and exact template/header mappings before refining
source prefix absence. The optional adapter authenticates source construction; the native
compiler validates supplied data and identities without requiring PoB. Source formatting
knowledge remains in Import and never becomes a native evaluator field. Existing item-base
v1 artifact bytes and source-policy v4 semantics remain unchanged.

The source package proves no generated prefix for 1,743 of 1,756 bases, while 13 bases
remain known to have prefixes. Applying the catalog removes this blocker from 90 of 116
original item records; other source-layout gates still prevent additional modifier
admission. The five originals remain Pending and all 110 queries remain present.

D3's component package adds eight assembled elemental/chaos endpoints and one shared
local-elemental percentage channel. Eight effective-value contributions and eight receiver
programs use existing owned operations for all 337 weapon templates. Complete actual
occurrence evaluation, action routing and whole-build parity remain open. The real rule
artifact crossed the old 8 MiB Engine wire ceiling; its bounded ceiling is now 16 MiB with
all structural/work guards unchanged and explicit smaller-limit rejection tested.

**Next acceptance sequence:**

1. Add a narrowly versioned, injected Import classification for proven zero-member metadata
   preamble rules, beginning with the 97 `Unique ID` occurrences in the originals. Preserve
   source identity/provenance, placement checks and existing policy dialects. Do not treat
   every Header recipe as harmless metadata. Quantify real range/default admission changes.
2. Classify meaningful defence-display, Spirit, Charm Slots and Corrupted headers separately.
   Preserve rune/socket and ordered modifier obligations until their own semantic conversion.
   The pending socket-configuration proposal still requires the owner's answer before its
   dependent Core change.
3. Connect actual imported eligibility and effective-value producers to assembled consumers.
   The original corpus contains no magnitude donors, so implement donor coverage with a
   genuine additional fixture rather than speculative source lifecycle replay. Proving
   source donor absence does not close an incomplete native modifier collection.
4. Validate contribution reduction order near rounding boundaries before claiming bitwise
   occurrence-level parity. Current receiver tests start with reduced facts; deterministic
   native sum order alone is not a source-order proof. Resolve this semantic contract before
   final local weapon field/action integration.

This checkpoint retires no still-used legacy numerical consumer. Its adapter-only source
acquisition has a named finite-data consumer; no new parser VM, profile dispatch or UI
construction path enters evaluation. Keep all-five/110-query gates and the T1 Rust tooling
policy unchanged.

## Shared item input and local scaling checkpoint

D2 now uses item-line policy v5 to map shared catalyst headers to the uniquely selected
template's declared parameters. This is an Import adapter seam: owned builds retain
ordinary typed assignments, and native evaluation consumes equipment properties. No
source field, template-name switch or new runtime interpreter enters Core. Existing
v2-v4 policy domains and numeric semantics are retained.

The injected package extends 337 existing weapon templates with 674 parameter slots and
337 adapters, while retaining the reviewed ring. Nine local modifier families gain
catalyst and ordered-magnitude scalar programs; seven effective-value contributions feed
existing physical/rate/critical receivers. Elemental/chaos assembled weapon channels and
action routing still require separate work. No raw profile baseline becomes a contribution
receiver, and no Partial imported roll set becomes complete.

Source default validation indexes requested templates and charges actual bounded lookup
work; the 338-template package fits existing limits. Fresh-import catalyst defaults remain
conditional on an admitted source layout. Unknown weapon range prefixes still prevent
absence proof, and explicit headers cannot borrow another template's slot. Header parsing
and scalar computation do not prove game crafting legality.

Next acceptance work must connect reviewed magnitude/eligibility producers and effective
values through actual owning item occurrences, then assembled weapon/action consumers.
Retain the five originals, 110 query rows and complete-request gates. Caller-fact component
chains are useful validation but do not advance 0/5 complete original-build evaluations.
The socket-configuration proposal still needs the owner's answer before dependent changes.

## Canonical modifier value checkpoint

D2 compiles explicit raw component policies into existing native rules: internal precision,
signed rounding, corruption, ordered magnitude, display precision and qualifier sign.
The compiler adds no runtime parser/interpreter or Core opcode. Import policy v4 now
performs explicit numeric projections and preserves known rolls under Partial input
membership. Existing v2/v3 meanings and identities are retained. Temporary lexical zero
signs and the source's decimal transport remain Import concerns, not native evaluator state.

The reviewed source migration covers 27 grammars across eleven canonical owners. All five
originals exercise it: six previously admitted lines yield eleven raw numeric components;
22 other matched lines retain their blockers. Each admitted ordinary line supplies an
explicit required corruption factor of one only after source controls are checked. Partial
roll schemas become Pending draft membership; missing facts never gain runtime defaults.

The authored revision is rebuilt from the 9533-entry transform ancestor, ending at 9854.
The earlier 9843-entry artifact remains immutable and is not a valid predecessor: changing
its Complete canonical input lists to Partial would violate the extension contract. The
new revision adds eleven required factor slots and leaves future eligibility inputs open.
See the [numeric contract](owned-modifier-values.md#canonical-numeric-components) and
[data package](../data/owned/poe2/3887ae68/modifier-value-inputs/README.md).

Next establish real magnitude/eligibility and source encoding authority, then route final
weapon/character contributions. PoB's fixed-value cache can retain an earlier nonidentity
factor result; its ranged recomputation has a different boundary. Raw admission and final
factor parity do not certify every cache path. Preserve those gaps explicitly rather than
reintroducing UI replay. Whole-build parity remains 0/5; all 110 queries and global gates
remain. New tooling/tests are Rust; existing Python suites stay under the T1 migration plan.

## Ordered modifier and augment membership checkpoint

D2/D3 now bind ordered Add/Multiply projections to exact sibling modifier occurrences in
the existing native dependency plan. Operation set v10 and the injected consumer retain
older contracts and all Partial coverage. Producer semantic order comes from the build;
recipient predicates, initial factors and activation are ordinary typed dependencies.
This component does not establish effective source values or complete original-build parity.
See [modifier values](owned-modifier-values.md#ordered-transformation-contract).

Rune evidence now prevents the importer from closing semantic item/equipment membership
while socketed child records are absent. Known records and provenance are preserved. The
[next augment binding contract](owned-augment-binding.md) now records a persistent
configuration gap: per-use edges cannot preserve unused inventory setups. The separate
[socket configuration proposal](owned-socket-configurations.md) awaits a design decision;
dependent Core/materialization work must not assume it is accepted. Caller-authored preview
requests remain preparation inputs, not authority that their occurrences exist in a build.
Complete native originals remain 0/5; socket materialization, reconciliation, effective
numeric stages and final weapon/action routing are still required.

## Attribute conversion checkpoint

The ordinary attribute family now has a Rust offline compiler using injected target
stats, lane values and exact source facts. It reuses the existing choice/effect language
and preserves all 293 physical owners. A checked, endpoint-bound passive declaration
refinement closes only reviewed port-list membership; unchanged slots, topology, registry
history, import policies and query meaning remain enforced by the shared finalizer.
This is a concrete D2/D3 input/effect component, not full tree legality or native-build parity.
See [passive topology](owned-passive-topology.md#ordinary-attribute-effects).

Class-dependent views now use typed selected-character predicates in operation set v7,
with unchanged v6 artifacts still accepted. The finite catalog contains 78 nodes with views
across Witch, Abyssal Lich, Druid and Huntress selectors. The Rust offline converter lowers
reviewed whole lists into ordinary guarded contributions: matching class first, matching
primary ascendancy second, default otherwise. Physical nodes and topology stay unchanged.
The first seven-node family appends four injected stat definitions to the attribute
successor and includes a player-to-Sniper granted-minion-modifier receiver path. This is
component coverage, not final spell/minion damage or whole-build parity. The other 71
view-bearing nodes and final attribute receivers remain open.
This 7476-entry successor precedes the class and intrinsic conversions below; see the
[production data command](../data/owned/poe2/3887ae68/passive-views/README.md).

## Class and attribute integration checkpoint

All eight class-base attribute rows now have an offline Rust conversion into ordinary
owned contributions. The generalized endpoint-bound port-closure assertion supports only
Class and PassiveNode; earlier passive manifests retain their exact representation.
Class numerical coverage stays partial because source unarmed defaults and other intrinsic
effects are separate from the three base attributes. No extra schema base-value fields or
class-specific Engine branch were added.

The generic v8 QuantizeInteger operation preserves explicit unit/count boundaries and
provides arithmetic for future attribute receivers. It does not establish final attributes.
The [attribute design](owned-attributes.md) makes finite staged snapshots and contribution
membership the next D3 structural work: two passes, ordered live per-stat reads, between-pass
comparison updates, separate MORE grouping and inherent-stat receivers. This prevents
accidental final-stat cycles or source UI/database objects from becoming the native model.
Do not substitute the existing single-store numerical fixtures for complete original-build
validation. The class-base successor precedes the intrinsic conversion below; all 110
queries remain fixed.

## Intrinsic attack and source-selection checkpoint

The optional Rust/`mlua` adapter now exports a finite, pinned intrinsic attack catalog;
the default native compiler consumes only that artifact and injected field/stat/unit policy.
All eight class baselines append to the class-base successor using six new owned IDs
(four Actor statistics and two exact units). Source class 0 is an explicit exclusion, and
every source field is accounted for. Class numerical membership remains Partial. The
conversion preserves the original 110 queries and does not add a profile-specific evaluator.

Routing v2 chooses a named source once per action/selector. It separates authored active
hand occupancy, a typed equipment eligibility capability, intrinsic actor values and
skill-local replacements. Every channel has exhaustive typed branch bindings; missing
selected values never trigger another source. Existing v1 artifacts preserve their wire
and digest. Native selection and evaluation run in process with per-worker scratch.
This is D2/D3 component coverage: concrete original equipment/action policies, staged
attribute contributors and complete original evaluations remain open. Continue from the
7482-entry intrinsic successor; keep the historical checked-in `current` bundle unchanged.
No existing Python utility or test was converted; the new exporter, converter and tests
are Rust, consistent with T1.

## Item-base identity and source-presence checkpoint

The optional Rust adapter projects the authenticated final constructed base table into a
finite catalog of 1,756 identities; the native compiler accepts only that artifact and
injected policy/definitions. It emits exact base headers and ordinary EquipmentUse
capabilities for explicit base-profile presence (337 true, 1,419 false). Unsupported shapes
remain unknown. The single registry appends capability 7483 and 1,755 templates through
9238, preserving Sapphire Ring and all prior schemas. New schema facets and numerical
membership remain Partial. Native evaluation never receives legacy metadata tables.

A full-catalog test exposed needless quadratic accounting in the source-default validator.
It now retains only templates with configured defaults, still scanning and charging every
emission; resource caps, default guards and serialized identities are unchanged. New
exporter/converter/validation code and tests are Rust, with existing Python code untouched.

This is D2/D3 identity/component progress. Continue with numeric equipment channels and
independent compatibility/hand/actor policy, not another skill-specific profile. Keep all
five original query sets and 0/5 full native results. Conditional variant selection,
decorated magic base names, local modifiers, replacements and generated-actor baselines
remain explicit obligations. The new successor follows the 7482-entry intrinsic package;
the checked-in `current` bundle remains historical. See
[item-base commands](../data/owned/poe2/3887ae68/item-bases/README.md).

## Native actor and item input checkpoint

The finite actor catalog now has a native offline compiler. Explicit policies bind
scalar fields and level curves to exact Actor slots and output stats; optional creating
parameters project through a finite level table. The generated runtime package contains
ordinary typed rules/tables with preserved Partial ownership. The first real policy
adds six Sniper scalar facts and the explicitly allied level-damage curve. Separate
creating occurrences are tested with distinct levels, activation and worker scratch.

Ordinary weapon modifiers now have explicit nominal roll recipes and exact base-template
membership. Reduced ranges apply sign after interpolation; nominal values cannot become
effective contributions until source encoding and ordered magnitude stages are proved.
Augment preparation is an import-only stage: explicit socket occurrences/categories
produce grouped descriptions with provenance. Their activation/scaling remains Unapplied,
and their existence in a validated owned build is not established by caller DTOs.

Next, bind those prepared groups to actual socket membership and reconcile saved rune
lines before owned modifier admission. Complete the ordered modifier/effective-value
contract, weapon assembly and action compatibility, while generated-actor actions move
through the same native plan. These are D2/D3 components; no legacy numerical consumer
has been retired by intermediate values alone, and D5/full-original exits remain open.
All new compiler/CLI/test work is Rust; T1 leaves existing Python suites intact.

## Current structural and breadth priorities

The owned-model boundary is the controlling design; the latest user review reinforces
build-time conversion, independent UI/model/evaluation/search and physical removal of
obsolete paths. Every new operation must serve owned game semantics. No further work is
scheduled on native PoB UI lifecycle or the generic source-language frontier.

Modifier precedence is now explicit owned input, separate from record storage order.
Input/project/inventory/request and draft envelopes move to version 4; unknown imported
precedence remains Pending. This supplies structural correctness for future noncommuting
transforms without inventing PoB parser order or adding another interpreter. It does not
implement transform folding or make partial item membership complete.

The current-source census finds no direct item-magnitude transform producers among the
595 serialized member lines in the five originals. This is prioritization evidence, not
permission to treat unknown modifiers as harmless. The checked offline transition now joins the 2,589-entry item successor with the
2,515-entry main import family, preserving old declarations and publishing the consolidated
current bundle. All five originals consume it through the existing normalizer CLI; private
test-only rebinding no longer substitutes for that integration.

Class/ascendancy implicit roots now belong to the Character provider, deduplicated by
physical passive identity. Attached options belong to parent choice slots. Schema wire 2
makes root membership explicit; point pools admit Either scope where appropriate. The
[passive topology contract](owned-passive-topology.md) specifies separate injected costs,
shared-plus-maximum and each-scope budgets, acquired capacities and access constraints.
The injected allocation cost/budget DTO, validated package and offline source conversion
are now delivered. The native usage/capacity/access executors remain next; structural
conversion and validated storage do not establish a legal build.

The finite tree exporter and Import compiler now extend the single current ledger from
2,589 to 7,469 identities. The same finalizer emits a typed tree normalization artifact,
bound to the final schema/mapping/registry and the rebound base-policy digest. All 16
saved specifications map to 1,333 allocations, 335 choices and 32 root origin links.
Catalog reruns reuse exact IDs; missing links and unconverted effects stay partial.
The importer consumes source syntax once. Core/Engine only receive owned records,
schemas and typed effects, never source node lists or Lua programs.

All five selected trees expose 613 source tokens, including two implicit roots each;
do not treat them all as paid allocations. Every selected attribute node has an explicit
override. Original02/04 each have disjoint 24+24 weapon overlays whose usage shares the
ordinary pool. The whole saved corpus has 1,369 node tokens, including roots, attached
options and inactive alternatives. Count selected closure and whole-build parity separately
from records. Common item effects and support/provider semantics follow through the same
model, with Twister and Sniper developed together.

The existing catalyst scalar remains an intermediate modifier-local value. Final effective
scaling needs source encoding, complete transform membership and distinct rounding stages;
receivers then need all applicable contributions. Current owned operation support includes v9 equipment receivers; supported historical
contracts retain their identities. Global contributor closure and complete-request gates
are unchanged.
Original native completion remains 0/5. Paired consumer retirement is still required; an
owned codec, successful package build or absent mlua dependency alone does not close D5.

## Native metric and retirement checkpoint

The [owned metric contract](owned-metrics.md) now gives D3 a data-bound native query
consumer. It reuses the effect executor, binds exact final stat indices and separately
checks actor/action activation. It requires complete owned requests and the existing
global contributor closure; it does not adopt partial-build evaluation or complete D3.

The ordinary resistance arithmetic in `engine::resistance` and `actor_receiving` now
shares one pure Rust kernel with explicit inputs and limits. Both legacy callers retain
their admission and contribution order. This removes duplicate arithmetic, not their
profile preparation. Real owned item/reward contribution recipes remain partial and lack
real final receiver data. The [stat-owned actor receiver boundary](owned-stat-receivers.md)
is implemented: explicit applicability, one instance per concrete actor, existing activation/
dependency checks and global closure. Direct-authored player and owned-actor contrasts
exercise metric queries, collisions, cycles and worker reuse. Rule wire v2 is explicitly
migrated; shipped game registries remain empty until actual receiver data is reviewed.
Do not use a fabricated class/usage owner or change partial-input gates to bridge the gap.

D5's distribution exit must inspect both normal dependency edges and shipped/compiled
contents. Data/Engine now have an isolated owned-only feature closure, checked against
actual compiler dependency files in CI. Legacy source/parser/profile/snapshot modules
and source-embedding fingerprints are excluded when these libraries disable defaults.
The root CLI now makes PoB opt-in, while full CI explicitly enables reference tests.
Root Import/Native consumers still activate the legacy library features: moving those
callers and testing the shipped application with only owned artifacts remains open.
A separate library build is evidence for the seam, not completion of D5. Keep existing
legacy callers behind that temporary feature until each named replacement is admitted;
remove their entry points and feature dependencies together. The independent oracle stays
optional. The [retirement inventory](legacy-retirement.md) names the remaining pairs.

## Current offline production data gate

The [offline assembler and production data](owned-offline-data.md) now connect persisted
registry/schema/rule/routing recipes to the same validators and native compiler used at
runtime. Real Twister/Sniper coefficients use complete finite level tables and explicit
source-bound quality coefficients. Checked native lookup preserves unsupported domains
through required inputs and provider/actor/action evaluation. The optional source exporter
recognizes reviewed literal shapes only; it does not load UI state or interpret Lua.

The production catalog extension now binds all-five normalization to this persisted
registry. It preserves existing IDs/descriptors, rejects stale inputs and produces explicit
successor bindings; runtime loading never repairs artifacts. Reviewed reward/equipment/
quality policies retain the existing breadth coverage, and breadth tests load shipped data
instead of allocating a separate private identity namespace.

This advances D2 beyond test-authored numerical fixtures. It does not complete D2/D3:
selected input semantics and effective values still need all applicable contributions,
and the real recipes retain partial mechanics/routes. The assembler and normalizer report
those limits instead of declaring a full build. D5 retirement of
active Spark/Mace preparation still requires replacing both fresh calculation and its
independent realization validation with the owned semantic path.

## Concrete first vertical slice

1. Define the owned inputs, game/rules identity and metric selectors in portable contracts.
   Make the input constructible directly; origins are optional adapter metadata.
2. Map the five imported selected views into those contracts. Record ambiguity and unsupported
   mappings explicitly. Preserve full original source separately for inspection/export.
3. Compile a small set of real domain effects with distinct semantics: local item arithmetic,
   an attribute-conditioned effect, support applicability and an item-granted action. Use
   injected definitions and more than one build/actor context, not per-skill handlers.
4. Resolve/evaluate the resulting semantic plan using shared numerical kernels. Keep full
   build coverage false until all required active dependencies and metrics pass.
5. Compare semantic intermediate values and final observable results with the optional
   oracle. A source callback completing or a table graph matching does not pass this gate.
6. Remove the replaced profile/compatibility consumer and migrate its meaningful numerical
   cases to the shared API in the same checkpoint. Record remaining consumers by name.

See the [concrete retirement inventory](legacy-retirement.md) for file/type dependencies,
completed finite-catalog deletion and remaining shared-type catches. The
[owned build contract](owned-build-contract.md) proposes concrete D1 records and validation
seams. Its record/codec/request API, standalone inventory, coherent record unions and exact
availability bindings are implemented. Independent project/preset composition and schema binding are implemented; see the
[composition/binding contract](owned-binding.md). The [owned draft boundary](owned-drafts.md)
adds partial session storage, structural validation and explicit selected finalization. Draft
repair and revisioned edits/adoption remain open. A conservative all-five source normalizer
now assembles owned drafts with explicit gaps; complete selected normalization/finalization
remains open. See [owned normalization](owned-normalization.md), including independent
passive-socket equipment contributions (typed composition now implemented; imported semantics
still pending), implemented reference projection routing and source-kind independence.
Finite injected configuration reward contributions now exercise the offline data seam
across all five builds. Their direct input schemas and reference row bindings do not
complete effect compilation or numerical evaluation. D2 must replace identity-only
placeholders with semantic declarations and
allow source definitions to produce zero/one/many owned declarations; PoB Gem rows do not
prove physical gem ownership. The
[owned package contract](owned-definition-package.md) now has typed descriptors and a
validated injectable schema package/index. Durable owned-ID allocation and exact offline mapping artifacts are implemented. Catalog conversion is now implemented. Complete D2 semantic coverage remains open;
schema validity is not numerical coverage.

The [owned rule component boundary](owned-rules.md) is now implemented: Core authoring DTOs,
Data's strict schema-bound rule storage, Engine's typed DAG compiler and explicit-fact
executor, and the thin `check-owned-rules` CLI. Stat/capability definitions bring the schema
to 24 standalone descriptor families. Item/gem quality has explicit presence/amount reads;
missing facts never become default zero. Synthetic component fixtures exercise local-item,
attribute-condition, support and grant operations with changed data and scope checks.
Persisted real-effect recipes and reviewed numerical tables now extend these fixtures,
without completing original-build parity; **0/5** remains.

The remaining D2 gate is sufficient real-effect coverage for complete original builds,
with independent numerical evidence. D3's owned effect-plan component implementation is validated. It accepts a
directly authored `OwnedEvaluationRequest`, injected schema, compiled rule package and a
schema-bound action-routing artifact. It binds exact equipment/modifier/provider occurrences,
projects stats into declared generated actors, and compiles effect-level dependencies with
explicit closure and cycle checks. It returns concrete effects, semantic stats and gaps;
the subsequent [metric layer](owned-metrics.md) consumes its final values and activation
gates. Neither completes a selected original build.

Routing names an exact action output, an All or exact part/mode/stat-set selection, and an
explicit player-equipment slot or action-actor stat source. Storage preserves overlapping
routes; the Engine resolves the concrete relation and rejects competing final producers
after Current/Actor/Player normalization. There is no implicit weapon selection, numeric
fallback or source-format lookup. Generated actor keys retain the supplying provider prefix.
A parent projection can remain known while its child grant is false: child consumers and
actions are inactive, and that diagnostic value is not proof of an active actor. Missing
grant production remains unresolved.

The initial plan conservatively requires complete discovered contributor coverage before
using reductions or final-stat dependencies. Unsupported active support, payload, usage and
granted-allocation relations remain explicit gaps. An empty complete contribution set can
use its declared identity; partial or missing membership cannot. Plans bind request/schema/
rule/routing content and separate immutable dependencies from per-worker scratch. Direct
occurrence, lazy-input, required-input, activation, cycle and scratch/worker tests pass in the
selected checkpoint suite documented in [implementation](implementation.md). The plan still
prepares an exact snapshot for every changed candidate; reusable candidate bindings and
throughput measurement are open. Hosted success is recorded separately.

No active legacy numerical consumer is retired by introducing this plan. Local weapon
preparation and its active action/timing consumers still need a shared owned replacement,
with their useful reference laws preserved and the replaced profile/request dependency
closure deleted. Complete metric coverage, all-five finalization and that paired numerical
retirement remain open.

## Shared timing and real quality inputs

The owned operation set introduced in v4, now v6, includes a bounded native ordinary-timing algorithm with
explicit reciprocal units and independent output classification. The legacy timing entry
point delegates to the same pure primitive, preserving numerical/reference laws. Its
profile-specific input preparation and independent realization-validation path still need
migration before removal. A checked timing component does not establish complete branch,
support or metric coverage.

Explicit gem quality now imports through injected kind/amount policy for all five originals,
including known zero amounts. This advances input normalization while preserving pending
schema/collection/selection obligations. Production persisted recipes and reviewed
finite tables are now implemented. Expand their coverage through the existing package
validators and keep source-fact acquisition offline; do not encode source callbacks, promote test recipes to a complete catalog, or
use reference output values as execution inputs.

## Explicit skill supply and equipment-scope checkpoint

Use existing domain grant operations for physical Gem -> supplied Skill -> owned Actor
relationships. Gem potential membership remains useful to the import/binding contract;
activation and multiplicity come from exact declared slots and instantiated rule producers.
The plan now accounts for such explicitly supplied gem skills, rejects multiple traversal
paths to one skill slot, and gates routed values on the same required generated inputs as
rule outputs. Root action selectors and authored support targets are not implicit child
aliases. No new source interpreter or activation artifact is introduced.

The importer now admits equipment scopes from injected exact source-slot mappings and
allocates loadout members only from observed unambiguous slots. All five originals exercise
this data seam. Complete original finalization, saved active-selection conversion and
passive/skill scope semantics remain open. Keep global pending closure and source diagnostics;
do not manufacture an otherwise empty complete build to run the numerical engine.

The immediate next conversion work is explicit quality kinds/amounts, projected effective
skill levels and real local weapon/actor inputs, followed by support application to exact
supplied occurrences. Sniper's actor abilities must retain their own output/skill context;
an unresolved source command reference must not produce an invented physical gem or skill.
The numerical timing replacement also needs independent output availability: a finite
capped rate can coexist with an unavailable nonfinite uncapped rate. Preserve the existing
kernel/reference laws while replacing both calculation and realization-validation callers.
The active Spark/Mace consumers remain scheduled for retirement with that replacement.

## Item/provider checkpoint and next integration gate

The import seam now has injected item-line declarations, while rule operations v2 can
project an item-owned computed parameter into a declared generated skill. CLI normalization
requires the item artifact and records its identity; optional source tests isolate local
weapon arithmetic and granted-skill range behavior. These are partial D1/D2 capabilities.
No native source-method replay or PoB UI object is introduced into the owned engine.

Before the next complete semantic plan, address the model issues exposed by real items:

1. Explicit unspecified item level is represented by the version-3 owned input/draft model.
   The importer still needs a scoped source-absence proof; unmatched rules remain pending.
   Validate selected finalization separately from field representation and legality.
2. Convert range/variant/socket source meaning with explicit provenance. XML range IDs are
   classified-list positions, not raw line indexes; source save/load rune membership differs.
3. Bind generated-skill parameters to an actual receiving provider and separate activation.
   Transport local weapon outputs into the action's chosen attack source explicitly.
4. Prove producer and contribution closure across programs before publishing a metric.
   Compile compact indexed inputs once; keep import, hashing and serialization outside
   repeated Rayon candidate evaluation.
5. Migrate the first active numerical consumer and its useful parity laws together, then
   delete its profile/source-shaped request path. Component tests alone do not retire it.

Original02 Twister and original05 Sniper remain joint design cases, with all five source
inputs and all 110 reference rows retained. Item-line knowledge and an oracle-confirmed
grant level are not complete native builds. The denominator remains **0/5**.

## Source range-attribution checkpoint

The bounded Import-only source-layout adapter now surrounds the existing injected item-line
converter. Source syntax and classified-list positions belong to that adapter; owned
modifier/parameter definitions, units, interpolation and rounding remain injected. Reuse
bounded lexical matching instead of building another semantic parser or source interpreter.
One physical line may emit several owned values without occupying several source list slots.
Unknown source membership must never disappear and shift subsequent range targets.

First prove one canonical text chunk with reviewed single-line members and no unimplemented
list-changing lifecycle. Original05's staff is the positive case: prove implicit/explicit
positions, apply inline then ordered XML overlays, and obtain native grant levels 1/11/20
from changed fractions. Original02 retains literal speed 49 while rune/Bonded rebuilding and
source-list uncertainty stay explicit. Source save/load lists differ; do not index raw lines,
reject ordered duplicate writes as inherently ambiguous, or regenerate affix metadata over
explicit numbers. Missing/malformed/out-of-scope inputs remain visible and dependent values
stay pending. Literal values can survive without complete range attribution.

Keep one flat provenance table for line/category positions, range writes, winning writes
and unresolved targets; bound aggregate work and diagnostic output. Compatibility source
pins document offline review, not a requirement to load PoB during ordinary import or proof
of the exporter's exact version. Runtime binds supported source dialect and artifact identity.
Native attribution/conversion fixtures now cover the original staff and conservative spear
path. Two optional tests execute the pinned original ParseRaw, XML parser, exact ItemsTab
overlay loop and BuildModList; they confirm staff writes and the spear's rune-excluded
load indices. Production does not construct PoB UI objects.
Whole-item closure, provider activation and complete builds remain separate gates. The
source audit and proposed API are in `runs/owned-item-level-01/range-design.md`.

The normalizer requires an explicit `--item-source` policy bound to its item-line artifact,
and the current sidecar v8 binds reference routing to that same policy. Rare titles are consumed as
presentation independently of injected semantic patterns. Unsupported tags remain blocked
while retaining candidates from raw and cleaned text, so they cannot hide competing fields.
Preamble position failures block facts as well as range proof. Source/output/work bounds
include repeated invalidation and candidate collection; uncertainty never becomes a default.

The owned effect-plan work above validates exact provider bindings and now integrates
real imported item inputs with persisted scalar recipes. Complete local weapon outputs,
final resistance and whole-build evaluation remain open.
Do not broaden the source adapter into general PoB object reconstruction. Rune/Bonded,
variants, collection closure and complete selected finalization remain explicit conversion
work. Scoped item-level/quality absence is implemented for the reviewed Sapphire template;
broader absence/default policies require separate evidence. D3 must activate the item-granted Firebolt separately from
the authored Sniper and exercise player/minion paths through the same owned contracts.
The next numerical checkpoint still requires a named active legacy consumer migration and
retirement; the source-attribution component alone does not meet that gate.

## Retirement ownership

| Existing area | Destination / retirement condition |
| --- | --- |
| Generic metrics/objectives, CandidateEvaluator, search budgets, cancellation and locks | Reuse; adapt public request/candidate types to semantic inputs. |
| Injected catalogs, stat aggregation, local arithmetic, actor/defence/action kernels | Reuse behavior after extracting profile-specific inputs/outputs; preserve numerical tests. |
| ImportedBuildInstance/source spans/selected-view adapters | Import/export sidecars and normalization tools. They cannot be required by the new native evaluation ABI. |
| NativeInput/NativeCalculation Spark/Mace dispatch, profile::parse and remaining profile search adapter | Temporary legacy only. Delete entry points and dependency closure as semantic consumers land; do not add profile variants. |
| Data records and CompiledGameData caches specific to legacy profiles | Remove with last legacy consumer; owned package must not require Spark/Mace sections. |
| Config controls, loader stages, class protocols, Lua callbacks/upvalues and source VM | Confine needed acquisition/oracle tooling to optional tooling dependencies; delete unused native consumers. Port actual game effects to domain rules. Do not simply rename source bytecode as domain IR. |
| Controlled mutation/XML export helpers | Keep only as format adapters/oracle tests where needed; search operates on BuildSpec. Delete redundant re-export facades immediately. |
| Spark/Mace reference builds and independent numeric goldens | Preserve as tests through the shared API where they cover real mechanics. Remove redundant harnesses/examples together with obsolete commands, not by substring filename deletion. |
| Prototype hook/constructor probes under runs | Frozen diagnostic evidence. Not production or an adoption queue; their unresolved harness issues do not block D1. |

An active legacy module is not dead merely because it has the wrong architecture. The
inventory must identify callers, shared types and unique tests before deletion. Conversely,
continued active use is not an excuse to leave it in the target model. Experimental CLI
removals can be breaking changes; document removed commands and migrate examples/tests.
Do not invent silent compatibility defaults or a second permanent path to preserve them.

## Validation and checkpoint rules

- Keep fixed original-build metric manifests and semantic selections. No changed tolerance,
  dropped effects, smaller denominator or fallback-to-PoB disguised as native progress.
- Owned-codec roundtrips preserve semantic identity. Supported PoB import/export roundtrips
  preserve build meaning and instance/provider correspondence, with explicit unresolved or
  non-exportable diagnostics; byte-for-byte XML/UI identity is not the target.
- Model both validity and computability: an infeasible build can have useful finite or
  classified nonfinite measurements. Preserve negative resource values when the model
  produces them; search uses feasibility independently.
- Compare fresh native to fresh matched PoB measurements and domain transitions. Reference
  UI/table/closure history is diagnostic evidence only. Relevant imported history becomes
  explicit domain state or a documented compatibility policy.
- At each deletion run affected tests, static dependency checks, native-only compile/lint
  and applicable portability checks. Keep hosted Windows/Linux coverage and useful oracle
  suites; move tests only when their responsibility moves, never to hide a failure.
- Measure total complexity: shipping code/data, import/compiler, oracle tooling, tests,
  update effort and runtime cost. Moving all VM machinery to an always-required generator
  without reducing maintenance cost is not the end of the investigation.

## Decisions still requiring experiments

The owned-model/offline-acquisition/UI separation is decided. Benchmark textual DSL versus
structured authoring, compact interpreter versus generated/native kernels, and package
encoding on the same domain contracts. Do not reopen the requirement that PoB formats and
UI state stay outside native evaluation. Bring genuinely new semantic trade-offs, unresolved
import meaning or gameplay-versus-PoB discrepancy policies to the owner with concrete cases.

The first compact-authoring experiment now has a Rust implementation and real consumer.
The Fire/Lightning extension restated 1,756 templates in 8,376,788 bytes. The four-family
attribute successor uses a 178,797-byte V1 rule/allocation extension plus a 199,551-byte
finite membership patch instead of a 9,225,003-byte explicit extension for the same change.
The bounded offline compiler binds exact prior registry/schema/rules/routing and supplied
extension identities, then expands into the existing V1/V3 validation path. Runtime sets,
closure proofs and package formats remain unchanged. See [the contract](owned-recipe-membership.md).

This resolves repeated descriptor authoring for explicit item-template modifier additions;
it does not choose a general DSL, alter legality/coverage or shrink the materialized runtime
schema. Continue measuring whole-package load/evaluation cost, review/update effort and
compiler maintenance. Other membership facets need their own explicit reviewed contract if
real consumers require them; avoid a generic patch language without that evidence.
## Raw weapon profiles and action-specific baseline checkpoint

The optional authenticated adapter projects all 337 constructed weapon tables and all
14 numeric field names into finite owned input. Source absence survives acquisition;
reviewed conversion policy supplies damage-zero defaults and separate reload presence.
Numeric channels remain equipment-local raw facts, distinct from local item assembly,
hand selection, skill compatibility and final damage. The compiler shares append-only
exclusive-writer checks with intrinsic class baselines and preserves Partial coverage.

Compact successor publication v2 removes the duplicated recipe from the artifact bundle.
Its checked loader reconstructs the canonical recipe from typed constituents and validates
its manifest, policy/query/tree membership and endpoint bindings. V1 publication and its
noncanonical-input identities remain supported. V2 binds the exact supplied prior recipe
by a separately bounded digest, avoiding the prior-plus-successor duplication in the
transition hash without increasing Core or output caps. This changes authoring packaging,
not the semantic runtime or coverage model.

The next shared baseline slice must develop selected player and minion actions together.
Use finite actor-profile constants/level tables, exact creating-skill parameters and
`ProjectActorStat` to bind receivers to the declared actor occurrence. Action spell damage,
cast time, crit and selected stat-set coefficients stay action-owned. Do not add a
monster/skill-name runtime switch or make every minion consume an attack-rate channel.
SandDjinn has authored attack time zero, while its selected spell uses a finite cast time.
Its finite level 1–100 spell interpolation can be materialized offline with source-order
and rounding checks; a new exponent instruction is not currently required.

Original01 selects authored SandDjinn group1 at level20/action2 with no source attribute;
its separate group16 is Tree:13289 at level1/action1. Preserve those occurrences and their
creating providers. A definition's `fromTree` flag is not evidence to replace the selected
occurrence's level. Test different levels for duplicate summoning skills, unknown levels,
incomplete grants, minion isolation from player equipment, and exact active weapon/stat
sets. Local weapon assembly must preserve source filtering, quality/round order, separate
reload/damage-pair presence, and ordered overrides. Full native originals remain 0/5.

## Owned recipe extension and local equipment checkpoint

Directly authored domain expressions now have one bounded append/publish host:
`extend-owned-recipe`. It allocates explicit typed schema entries, preserves prior
programs/tables/receiver rows, and permits only reviewed membership additions to existing
Known Partial schema facets. Endpoint-bound refinement v3 retains scalar values, prior
members and coverage gaps across definitions and slots. Complete sets cannot grow and
Unmapped descriptors cannot be promoted through this policy. Existing v1/v2 refinements
and successor publication formats retain their meanings and identities.

This seam is shared by quality/modifier declarations and future generated actor/skill
ports; it is not an item-specific deserialization patch. New Complete rule owners are
allowed only for newly allocated subjects. A Complete receiver registry can gain rows
only for newly allocated statistics; it cannot silently expand prior-stat applicability.
Existing Partial owner programs can gain immutable, uniquely named rules. Publication
still rebinds the checked prior tree/import/query artifacts and preserves all 110 rows.

Operations v9 extends stat receiver applicability to exact EquipmentUse templates. Actor
wire aliases and v6/v7/v8 operation sets remain supported. Exact template adapters retain
item-quality read authority; stat-owned equipment receivers consume typed inputs and
contributions. The initial injected local recipes cover pre-override attack rate/crit and
rounded physical values before positive-pair field emission across all 337 raw profiles.
They do not close original item, allocation or whole-build coverage.

Source flag policy v4 is opt-in: fractured/desecrated token spans and explicit Boolean
properties survive conversion. V3 bytes/digests/behavior remain unchanged. Unknown,
malformed and rune tags withhold facts; preserved token provenance is not admission.
Rune headers require a later finite owned catalog, exact socket/category selection and
ordinary/Bonded reconstruction/reconciliation. Do not import saved rune lines as a
shortcut or count them twice. Existing production source policy remains v3.

Next finish local modifier/quality source admission and rune lifecycle with contrasting
real-input evidence, then elemental/chaos/range/reload, positive-pair emission, ordered
overrides and exact action compatibility/hand selection. Develop finite minion actor and
requested-action baselines independently, retaining exact creating skills and level
projections. Known schema ports can grow through membership v3; existing Unmapped
SandDjinn/Kelari identities require an explicit reviewed semantic promotion contract,
not an implicit membership edit. Keep all five originals and separate full-build parity
from component numerical checks. Complete native originals remain 0/5.

All new utility and test code in this checkpoint is Rust. T1 remains planned; existing
Python implementations and tests are deliberately unchanged.

## Explicit header inputs and finite actor/augment catalogs

The recipe extension host now accepts an optional exact pair of item-line and item-source
policies. Both files must be supplied together; stale definition/line bindings fail instead
of being silently rewritten. Omitting the pair retains checked prior-policy rebinding.
The first data consumer replaces opaque quality/item-level metadata with explicit values
for the existing template contracts. A quality amount of zero is distinct from absence;
missing, malformed, duplicate or inapplicable selections cannot become a known zero.
This preserves the definition, rule, tree and 110-query artifacts and changes only import
policy meaning. No new general parser or source interpreter was introduced.

Finite offline Rust acquisitions now preserve all 649 actor profiles and 594 socketed-
augment selector rows. They run only authenticated bounded data constructors behind the
optional PoB feature. Their catalogs contain typed scalar/ordered facts and explicit
unconverted metadata; source scripts and UI state do not enter native evaluation. Next
consumers are the exact generated-actor baseline compiler and native rune reconstruction.
These components do not retire a still-used legacy numerical consumer; D5 retirement
remains attached to corresponding end-to-end integration.

Actor conversion must use an explicit owned actor slot and exact creating occurrence.
Summon-level and allied/hostile damage tables remain separate: the creating-skill policy,
not the acquisition module name, selects the curve. Preserve raw zero attack time and
route spell actions through their cast-time inputs. Extra actor life/defence scalars and
modifier/flag descriptions remain unconverted until explicit policies lower them.

Rune reconstruction must retain socketed-item identity and its EquipmentUse::ItemSocket
ancestry. Select bounded active sockets/categories before combining effects by augment
family, normal/Bonded lane and numeric stat order. Equal order does not establish equal
modifier meaning. Known headers rebuild/reconcile saved rune lines once; unknown names,
including surplus headers, cannot be ignored. Bonded activation and additional effect
scaling are separate semantics. The extra scaled copy uses its own truncation/rounding,
not a blanket multiplication of the combined total. Missing-header inference is deferred.

Next integrate ordinary local modifier rolls with explicit nominal/effective magnitude
stages, then reconstruct/reconcile rune effects and finish local damage/range/reload and
ordered overrides. Integrate generated actors and caller-requested action baselines in
parallel. Keep whole-plan, allocation and original-build completeness gates unchanged.
All new tooling and tests are Rust; existing Python suites remain unchanged under T1.
