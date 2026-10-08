# Support origins belong to the composed request

**Status:** Accepted 2026-10-08; common native coverage gate implemented, validation
and game-data adoption tracked in the implementation plan. The first actual
source-construction census and slot-sharing control pass in both JIT modes.
The owner approved
discovery from each composed build, preserving authored assignments and order
on the skill preset.
**Date:** 2026-10-07.
**Decider:** Project owner.
**Scope:** The next input/composition decision needed for the first complete
Original05 evaluation. Optimizer work follows the first working build.

## First native domain

`RulePackageInput.support_discovery` carries sparse, reviewed per-owner domains.
The admitted value `AuthoredAssignmentsOnly` excludes positive extra/linked or
additional-effect origins throughout that owner's admitted inputs. Unmapped
rows retain exact GameRules gaps; omission remains unknown. Data validates
identities, uniqueness and storage budgets; the raw compiler uses the same check.
The common cold planner checks selected occurrence paths, shared Actors,
Encounter, usage and action owners. Disabled providers use the ordinary
structural resolver for coverage only; no effects or participation are enabled.
Missing/Unmapped coverage blocks both preparation and retained delivery.

Synthetic component fixtures explicitly declare their finite source domains.
Those assertions are not publication evidence for game definitions. Current
game data has not adopted these declarations, Import `01de` is still pending,
and authored-order naming/accounting and exact provider exclusions still need
their adoption checkpoint. No new positive origin family is implemented.

## Context

Original05's selected skill preset contains sixteen physical support assignments
in eight saved sequences. Their membership and local order are known, but issue
`01de` still represents incomplete support-origin discovery. The source also
considers additional effects of a Gem, item-provided supports, linked supports,
supports shared with item-generated skills, and provider-specific exclusion.

These are different responsibilities. A player chooses physical assignments on
a skill preset. The complete set of effective support sources can also depend on
the independently selected items, passive tree and generated providers. A proof
that the current item/tree combination contributes no additional support cannot
remain valid merely because the skill preset is unchanged.

Today, `SkillPreset.support_origins` is copied into the composed build by
`owned_project::compose_validated`. Core checks exact assignment membership and
targets; it does not discover additional sources. `SupportOrigin` currently has
only `Assignment`. Import's `support_inventory::Census/complete` certifies the
physical assignment list, while `support_order::OrderIndex` preserves local
order. Neither certifies complete runtime origins.

The current whole-request gates must remain. `DraftSession::finalize_with`
returns Pending before constructing a request if selected input obligations
remain; the CLI will not write a complete request in that state. Once a request
exists, native plans still require complete selected contributors. Resolving
input discovery would not complete Sand preparation, support delivery or metrics.

## Recommended decision

Keep **authored physical support assignments and their explicit order on skill
presets**. Derive and validate **complete runtime support origins from each
composed request and injected data during cold native planning**. Do not store a
provider-dependent absence claim as permanent preset intent.

Use the existing `SkillTarget`, `ProviderKey`, grant paths, supply slots and
support assignment identities. The derived inventory is an immutable checked
plan product, not a second build model, a new physical Gem, or saved PoB output.
This decision does not require implementing every positive origin family now.
The first admitted domain may contain only Assignment origins, provided every
other potential source has an explicit, valid absence or inapplicability proof.

### Cold completeness and execution

Injected owned data must declare the relevant potential support capabilities of
each selected provider and the rules relating origins to exact recipients. Cold
discovery covers every selected provider and reachable relevant capability,
including additional Gem effects; it must not depend on which Action is queried.
Reuse complete existing provider declarations/program inventories only where
their reviewed scope genuinely accounts for these capabilities. Partial or
missing coverage means unresolved discovery, never an empty set.

Data validation checks definition-level authority; cold binding resolves actual
occurrences, selected equipment/loadouts, allocations and declared descendants.
Repeated identical definitions remain separate sources. Shared-slot and linked
relationships need explicit source/recipient bindings and ordering semantics;
names, instance-ID sorting and nearest-provider guesses supply no authority.

Provider-specific `noSupports` must exclude only the proven supplied occurrence.
It must not disable a manual occurrence of the same Skill or remove independent
actor-provided level/quality properties. Discovery, activation, support selection
and numerical delivery retain their distinct checks. Inactive selected records
still need valid inputs and complete potential-source accounting.

Unknown, ambiguous, partially described or unsupported extra/linked support
families must produce a blocking coverage result. The ordinary and staged native
entry points must both enforce this check; directly loading a complete Request
must not bypass it. Only a successfully checked inventory may feed existing
support selection, receiving and source-property collection. Positive families
that need a new runtime origin representation remain unsupported until that
representation is explicitly designed.

Acquisition may inspect pinned PoB source to establish the data and exclusions.
Runtime composition, discovery and evaluation use owned Rust data and code only.
Neither Lua nor source XML interpretation belongs in the native plan or workers.

### Moving the current obligation safely

Update the current format in place: make the persisted sequence's authored-order
meaning explicit, and remove its implication of complete runtime discovery.
Choose the smallest DTO/name change during implementation review. Reimport and
rebuild affected artifacts; retain no old/new compatibility branch.

Do not simply delete `01de` or mark its list Complete. First prove that every
source-authored assignment, target, ordering relationship and other required
support input has been captured using the existing whole-source census and exact
provider resolver. An unconverted source relationship remains an Import input
obligation. Separately install the mandatory cold discovery/coverage gate before
transferring runtime completeness responsibility out of the preset.

Only then may the old mixed-scope obligation resolve for an admitted import.
Its provenance must identify the checked authored inventory and the new enforced
runtime responsibility. A structurally Ready request still does not claim
numerical readiness; unresolved native discovery must block evaluated metrics.
This preserves complete-input finalization and whole-plan Engine coverage while
placing each claim at the boundary where its dependencies are available.

### Identities and reuse

Bind derived inventories and plans to the complete composed request and all
relevant definition/rule/discovery-policy identities. Equipment, tree, loadout,
support assignment/order, provider inputs and scenario changes must invalidate
or revalidate affected discovery. A cached proof for one selection cannot certify
another selection using the same preset. Source updates must invalidate the
owned capability evidence and its dependent artifacts.

Reuse immutable compiled data and per-worker scratch. Bound discovery work and
provider traversal before allocation; deterministic ordering comes from declared
semantics. Failure, exhaustion and A/B/A reuse must not leave usable stale proof.
Reassess plan identity domains for semantic changes that do not alter serialized
input bytes; do not rely solely on a format rename for cache invalidation.

## Alternatives and consequences

| Approach | Benefits | Costs / limits |
| --- | --- | --- |
| **Request-composed discovery — recommended** | Preserves independent presets and exact provider ownership; one cold validity boundary for direct requests and later candidate changes. | Requires explicit capability completeness and a checked discovery stage; unknown families continue blocking evaluation. |
| Store complete origins on the skill preset with dependency certificates | Can retain much of the current persisted shape. | Every independent provider change needs certificate invalidation and reconstruction; derived state becomes coupled to preset intent and still needs a cold check. |
| Bind a skill preset permanently to one equipment/tree combination | Makes a fixed imported snapshot easier to certify. | Conflicts with accepted independent presets and duplicates intent across combinations; a later optimizer would require a redesign. |

The recommendation adds no optimizer search or broad support catalog milestone.
Its first delivery is a sound path for Original05's actual request, with generic
criteria rather than build names, hard-coded instance IDs or an example checksum.
Usage/configuration completion can proceed independently through their accepted
ownership models. More numerical components alone cannot clear these input gates.

## Implementation and acceptance gates

1. Specify the exact authored-order DTO change and owned capability/discovery
   declarations. Reuse Import's physical census, `source_shape` framing and
   generated-provider resolver. Keep physical membership, runtime discovery and
   numerical completeness separate in diagnostics and evidence.
2. Establish a complete pinned source census for primary/additional Gem effects,
   extra supports, slot sharing, linked supports and provider exclusions. Prove
   each admitted absence/inapplicability criterion; observed empty lists alone
   are insufficient. Preserve unimplemented positive families as blocking cases.
3. Implement mandatory cold discovery in the one native path, then migrate
   Import obligation accounting. Require the same enforcement for ordinary,
   staged, direct-request and checked-release entry points.
4. Test hostile mutations: same preset with a different item/tree/loadout;
   insertion/removal of an extra or linked support provider; additional Gem
   effects; manual/generated uses of the same Skill; duplicate providers;
   foreign or missing recipients; reordered assignments; disabled sources;
   Partial capability inventories; stale data/plan identities; failed A/B/A
   scratch reuse and bounded-work exhaustion. Unsupported changes must remain
   blocked rather than silently retaining the prior Assignment-only result.
5. Reimport Original05 unchanged and finalize it through
   `finalize_selection_checked` after all five selected input obligations have
   genuinely resolved. Retain its independent preset selections and all 22
   query identities; no fixture-only request or edited completion flags.
6. Publish an actual evaluation bundle and run the resulting request through
   `evaluate-owned`. Require complete discovery, contributor and preparation
   coverage, then correct requested numerical/availability results and stable
   reference comparisons outside accepted source defects. Verify native
   serial/Rayon determinism. Structural readiness or a report full of unavailable
   metrics is not the first working-build milestone.

## Existing integration points

### Source census and implementation implications (2026-10-08)

The optional [Rust witness](../crates/poe-optimizer-pob/tests/support/support_origin_source.rs)
uses the existing complete source bootstrap and supervised JIT-mode runner.
Its [observer](../crates/poe-optimizer-pob/tests/support/support_origin_source.lua)
captures calls to original `processGrantedEffect`, `createActiveSkill`, `List`
and `Tabulate` from pinned `CalcSetup.initEnv`. It preserves raw modifier buckets,
returned query values, exact source Gem/group object joins and constructor
candidate positions. It does not call a replacement discovery algorithm or
infer query absence from final damage.

Original05's selected preset produces the following cold-load census in both
MAIN and CALCS:

| Boundary | Observed result |
| --- | --- |
| Selected source groups / root constructors | 12 groups / 16 constructed effects |
| Primary effect processing | 28 calls, including 16 support effects |
| Additional effect processing | Four calls; Sand/Water Commands, none classified as supports |
| ExtraSupport | Nine original queries; every raw source bucket and returned list empty |
| LinkedSupport | One original query; raw source bucket and returned list empty |
| Tree-provider exclusions | Four Djinn summon/Command constructors have `noSupports` and empty candidate lists |
| Manual Sand | Three support candidates; disabling its Magnified Gem reduces this to two |
| Item-generated Firebolt | Empty candidate list in the unchanged original |

One source-format control changes only the manual Sand group's slot to
`Weapon 1`. Firebolt then receives all three supports from that **different
source group**, without changing physical support definitions or assignment
counts. This confirms a real source dependency outside the preset's simple
assignment sequence. It is a diagnostic of PoB's behavior, **not** evidence that
this slot arrangement is legal PoE2 gameplay, nor authority to add PoB socket
groups or automatic slot sharing to native data. Positive extra/linked sources
and additional support effects still need their own admission evidence.

The source also attaches `gemData` to manual Djinn rows and shares `fromTree`
metadata with their definitions. Neither field identifies a native physical
Gem or exact grant source. Preserve physical versus manual input identity from
the existing import proof and bind generated exclusions to their exact supply
occurrence. Do not put a global cannot-be-supported flag on the shared Skill
definition; that would incorrectly remove the manual occurrence's supports.

Six fresh loads per JIT mode cover baseline, independent repeat, uninstrumented
baseline, disabled support, slot sharing and uninstrumented slot sharing. The
two complete reports agree byte-for-byte: 607,968 bytes each, SHA-256
`e34ebf95d61c2ebbddd1300b59582149700468d67e24d6cd059008070407eba8`.
Reports are `runs/owned-support-origin-source-03/source-jit-{off,on}.json`;
the successful driver log is `runs/owned-support-origin-acquisition-03.log`.
This is source behavior at a fixed cold lifecycle, not native discovery proof.

The next native implementation must therefore:

1. Preserve the persisted sequence as authored assignment order only. Reuse
   exact existing target/provider relations, not the observer's group indices.
2. Check a complete capability inventory for every selected provider and
   relevant descendant before support selection or source-property collection.
   A complete skill-grant list or empty original source query alone does not
   certify all support-source capabilities. Unreviewed positive families block.
3. Keep provider-specific exclusion separate from shared Skill identity and
   from external supported level/quality contributions. Both manual and
   generated occurrences of the same Skill must be covered by tests.
4. Store discovery evidence on the immutable composed plan, enforce it for
   direct and staged entry points, and invalidate it with provider/data changes.
   Move Import `01de` only after this gate and authored-input accounting exist.

### Code boundaries

- [Core preset and composition](../crates/poe-optimizer-core/src/owned_project.rs):
  `SkillPreset`, `compose_validated`.
- [Core support records](../crates/poe-optimizer-core/src/owned_build/records.rs)
  and [structural validation](../crates/poe-optimizer-core/src/owned_build/structure.rs):
  `SupportOrigin`, `SupportOriginSequence`, `support_origins`.
- [Checked draft finalization](../crates/poe-optimizer-core/src/owned_draft/intent.rs)
  and [selected-input gate](../crates/poe-optimizer-core/src/owned_draft/finalize.rs):
  `finalize_selection_checked`, `finalize_with`.
- [Import physical census](../crates/poe-optimizer-import/src/owned_normalize/support_inventory.rs)
  and [local order](../crates/poe-optimizer-import/src/owned_normalize/support_order.rs):
  `Census`, `compile`, `complete`, `SupportOriginOrderPolicy`, `OrderIndex`.
  [Normalization lifecycle](../crates/poe-optimizer-import/src/owned_normalize.rs)
  currently completes physical membership before resolving generated inputs.
- [Engine support indexing](../crates/poe-optimizer-engine/src/owned_supports/build.rs),
  [provider coverage](../crates/poe-optimizer-engine/src/owned_plan/compile.rs),
  and [staged effects](../crates/poe-optimizer-engine/src/owned_plan/support_effects.rs).
- [Request output gate](../src/owned_draft.rs) and
  [native metric CLI](../src/owned_metrics.rs): Pending drafts cannot be written
  as requests; checked release evaluation requires an evaluation bundle.

## Owner decision

**Adopt request-composed support discovery, with authored physical assignments
and order remaining on independent skill presets, and mandatory cold completeness
checks for every selected provider?** This is the recommendation. It replaces
the current preset-level runtime-completeness claim without accepting unknown
origins or weakening the first full-build gates.
