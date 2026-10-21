# Source effect construction in the existing Import catalog

**Status:** Accepted and implemented 2026-10-20; artifact/fixture rebuilding,
scoped regressions and all-five publication preservation pass locally.
Consumer adapters are subsequent checkpoints in the implementation plan.
**Date:** 2026-10-20.
**Decider:** Project owner.

The owner approved retaining construction facts in this existing artifact and
explicitly authorized breaking unshipped interfaces, rebuilding current data and
removing related obsolete paths. Do the migration and its cleanup together;
compatibility readers, duplicate artifact models and transitional runtime branches
are not required.

## Context

The closest unchanged build remains Original05, Skeletal Sniper / Basic Attack.
Its saved skill-preset usage inventory cannot yet account for every global-effect
switch or group reference selector. The preceding checkpoint connected successful
support and generated-input adapters to that inventory without granting those
fields new authority.

The offline `SkillIdentityCatalog` already retains the source Gem's primary,
declared and constructed additional-effect references, final ordered resolved
effect list, display order, additional stat-set references and missing references.
Before this change, the Import compiler discarded most of these when producing
`OwnedGemRoleRow`; the loaded role artifact retained only the primary, role,
materialization classification and the compilation's catalog commitment. The
implemented contract now preserves the construction observations in that same
artifact; its publication status is tracked above and in the implementation plan.

Neither a catalog hash nor a primary Action correspondence establishes the
complete source effect list. Native `Gem.skills` is potential gameplay membership,
not the source's ordered list. A source may declare a missing Command even when
its currently resolved list contains only one effect. Multiple stat sets are
also distinct from multiple effects.

This matters beyond Ice Nova and Firebolt. A support can supply another active
effect, so its primary SupportAssignment role alone cannot make its saved count
and global switches irrelevant. Group reference selection also depends on
constructed source ordering. Those checks should share facts without acquiring
each other's completion authority.

## Recommendation

Extend the **existing Import-only role artifact**, with a required
source-construction field on its Gem rows. Populate it from the same validated
catalog in the existing compiler, rather than maintaining another catalog or
copying hand-authored effect lists into every disposition.

The current-format field must distinguish unavailable observations from observed
construction. Its compact representation must preserve:

- Exact source catalog row and primary identity.
- Ordered resolved effects, preserving repetitions.
- Declared and constructed indexed additional-effect references.
- Display-order information and exact unresolved references.
- The distinction between additional stat sets and additional effects.

Missing data must remain explicit; an empty resolved list cannot mean that the
source has no effects or that construction is complete. Definitions and identity
mapping can be known while mechanics remain unsupported. Do not add speculative
visibility, Vaal or lazy modifier flags as part of this change; any later
additional fact needs evidence and a concrete consumer.

The role index must validate collection/work limits, exact source and catalog
bindings, namespaces and identity/mapping joins. The offline publication step
must reproduce the facts from the authenticated catalog. A caller-authored digest
does not itself certify that a construction claim came from that catalog.

These are **source observations**, not gameplay rules or completion certificates.
A reviewed Import adapter must still establish how the source field is consumed,
check its admitted domain, and emit a private receipt for that exact field only.
The existing whole-preset gate must then recheck live source, output, provider
and preset membership, with every applicable consumer accounted for. Raw
exports remain unchanged and unresolved responsibilities remain Pending.
Generated and manual Direct occurrences additionally need their actual source
dispatch proved: a catalog Gem's list and the fallback list built directly from
an effect are different construction paths.

For example, a complete singleton effect list could support accounting for the
secondary switch only after proving every secondary consumer requires a second
effect. It would confer no authority over the primary switch, group selector,
count, reporting or numerical coverage.

The independent consumer proof now supports an exact primary `true` disposition:
it imposes no indexed restriction even when lazy metadata or effect order changes.
The existing private adapter checks the live source and its actual outputs; this
one fact does not require the proposed construction fields. Raw intent remains
preserved and no activation is inferred. Missing, false, `nil`, malformed and
secondary tokens remain outside that finite adapter. Its tests and delivery status
are tracked in the implementation plan; the role-artifact migration is approved.

## Alternatives and trade-offs

| Option | Benefit | Cost |
| --- | --- | --- |
| Extend the existing role artifact — recommended | Compiler derives one shared source fact set; future consumers reuse its bindings and unresolved-reference handling | Rebuild the current role artifact, dependent policies/packages and fixtures; add bounded validation |
| Add reviewed construction claims to each disposition | Smaller first artifact change, scoped to individual consumers | Repeats source lists and joins across physical, generated, support and group-selector adapters; claims can drift from the compiler |
| Use native Gem membership or a family allowlist | Short implementation | Incorrect boundary or overfitting; cannot distinguish missing effects, ordered source positions or unsupported mechanics |

The proposed change adds no database, second importer, source-language
interpreter, native preference or evaluator mode. Core, native Data and Engine
do not receive these source-only facts. It does not change the existing rule
that source/reference behavior must be justified before translating it into
native gameplay semantics.

## Migration and validation

Update the current format in place, regenerate existing artifacts and reimport
the five unchanged builds. Do not carry an old-reader compatibility path.
Preserve historical raw exports and source observations as evidence.

1. Completed: the owner approved the artifact ownership decision and immediate
   cleanup without compatibility support.
2. Complete independent Rust/mlua source-component tests for the constructor,
   secondary admission/export/count consumers, singleton versus multiple effects,
   missing declarations, late additions, display ordering and multiple stat sets.
   Include a singleton with the Vaal flag as a control instead of assuming that
   non-Vaal status is required.
3. Add compiler and role-index tests for known/unavailable observations, unresolved
   references, duplicates/order, stale bindings, invalid joins and resource limits.
   Regenerate current roles and all affected dependency commitments.
4. Extend existing physical/generated dispositions with narrowly checked field
   accounting, then prove exact occurrence/preset/provider binding, changed live
   outputs, unsupported tokens, cross-preset borrowing and atomic budget failure.
5. Publish through the existing release path. Reimport all five originals with
   authenticated whole-draft/sidecar inverses and unchanged native metrics,
   requirements and mechanical coverage.

Only subsequent adapter tests and the whole-preset gate can retire an input
obligation. Neither source-component tests nor the format migration alone makes
a complete native build.

### Generated reverse lookup follow-up, October 21

The full pinned catalog has one primary Gem candidate for Firebolt and for each
Djinn summon. Firebolt has one constructed effect plus an additional stat set;
each Djinn has two actual effects, summon and Command. These observations are
not a runtime dispatch certificate: a structurally valid role artifact may omit
another candidate even when its surviving row retains the catalog commitment.
The existing alias/subset controls must remain.

The required `OwnedSkillRolePackageInput.reverse_primary` census is now
implemented in this same Import-only artifact. Fresh compilation and catalog
extension carry the compiler observation forward. `Unavailable` means there is
no observation; an observed route with zero candidates proves absence for that
exact source/owned Skill. Each candidate retains its existing Gem and catalog-row
identity. Multiple candidates stay ambiguous; canonical sorting never chooses
PoB's unordered winner. A missing route or a different owned Skill remains
unavailable, not an observed zero.

Loading validates exact joins, namespaces and bounded nested work. Offline
publication additionally reproduces every route and candidate from the complete
authenticated catalog. This rejects a coherent edited subset even if its local
counts and commitments agree. The required field has no missing-field default
or old reader. Artifact rebuilding and validation are tracked in the
implementation plan; this update does not claim those checks have completed.

Requiring equality between role IDs and all mapped Gem targets is insufficient:
a mapping can also omit an alias, and a combined mapping can legitimately retain
unrelated older catalog entries. A whole-catalog commitment is not a substitute
for the facts needed by a consumer. Keep source observations out of the native
model and do not copy upstream lookup behavior into the evaluator.

The new source-component controls distinguish the actual dispatchers:
`LoadSkill` and generated-skill construction use a Skill-table-keyed reverse
lookup, while bare `ProcessSocketGroup` processing uses a string-keyed lookup
and can take the direct-effect fallback. Complete source construction, explicit
Gem selection and both alias-construction orders are separate observations.
These tests supply source evidence, not runtime Import or native authority.

Existing generated-field adapters still need their exact live provider, source,
slot and level checks before consuming the census. Count, secondary-switch and
reference-selection proofs remain independent. Both Djinn effects and unresolved
cases must survive; Firebolt's singleton cannot stand in for them. This change
adds no native behavior or schema revision and retires no selected input issue.
The bounded audit is `runs/generated-source-reverse-routes-audit-20261021/README.md`;
the implementation plan records its priority and execution status.

## Implementation and rebuild plan

The migration stays in Import. Preserve the native recipe, definition identities,
mechanical coverage and numerical programs; source construction is not a new
native Skill inventory. Implement the following bounded partitions, then rebuild
and publish together.

| Partition | Existing implementation seam | Required result |
| --- | --- | --- |
| Role contract and compiler | `owned_skill_catalog.rs`: `OwnedGemRoleRow`, `compile_catalog`, `OwnedSkillRoleIndex::new` | Required construction observations, compiled from the injected catalog and validated before use |
| Offline reproduction and rebinding | `owned_catalog_recipe.rs`, `owned_release_revision.rs` dependency rebinding, existing release assembly | Reproduce catalog-derived rows, preserve the native recipe and rebuild every dependent commitment in order |
| Physical and generated field accounting | `owned_normalize/skill_input_disposition.rs`, `gem_inventory`, `direct_skill_inputs`, `generated_skill_inputs`, `usage_inputs/inventory.rs` | Existing private receipts gain only the specifically proved source-field responsibility |
| Related policy cleanup | Current physical-Gem and Direct input policies plus their producers and fixtures | One current wire contract per policy, with no old-reader branches or inferred defaults |
| Artifact and regression rebuild | Import compiler fixtures, executable release seeds, dependent authoring fixtures, all-five publication harness | Regenerated current artifacts, explicit provenance changes and unchanged raw builds/native behavior |

Contract/compiler work can proceed independently of the policy-cleanup work once
field names and ownership are agreed. Fixture edits need separate ownership from
production edits. Rebuild/publication follows a frozen contract and compiler;
consumer tests can use compiler-produced synthetic catalogs while portable
artifacts are regenerated. Keep Cargo validation serialized with other work.

### Construction observations and validation

`SkillIdentityCatalog` already retains the necessary facts in `GemIdentity` and
its exact `missing_references` census. Copy those observations through a single
compiler routine; do not reimplement the source constructor in native code. An
observed row can still have unresolved references. Preserve source identifiers
alongside checked owned identity joins, including repeated positions, and keep
stat-set references in their distinct field. Unavailable observations must be an
explicit value, never a missing-field default or a fabricated singleton.

The role index currently validates roles, schema metadata, source pins and final
mapping commitments. Extend that same validation to the new nested collections,
source-row binding, primary/effect mappings, unresolved reference kind/index and
namespace. Charge reference counts, text bytes and aggregate work before cloning
or building indexes. Do not sort or deduplicate effect positions. A raw effect
identity may map to an Unmapped native Skill without granting execution coverage.

Runtime role loading validates the serialized contract without retaining the
source catalog. Offline authoring must additionally reproduce construction rows
from the authenticated catalog and compare them in full. A changed observation
must fail that comparison even when the caller recomputes the artifact digest.
Fresh compilation and same-pin extension should share this reproduction logic.
Directly authored test roles with no catalog observation must explicitly say
unavailable; this cannot authorize a singleton consumer.

### Rebuild without a native schema change

`compile_owned_release_revision` requires a real changed schema descriptor and
new schema release. Do not invent either for this migration. Use the existing
bounded dependency-rebinding sequence from the offline authoring boundary,
followed by ordinary release assembly, with the exact same native recipe. Rebind
role-dependent support, payload, Direct/generated input, skill and usage policies,
reference-action correspondences and physical-Gem inventories in dependency order;
then rebuild configuration-disposition and tree commitments where their inputs
changed. Ordinary loading must continue to reject stale commitments.

Required fields deliberately make old role JSON invalid for the current reader.
Rebuild executable seeds and fixtures from their authenticated source inputs;
retain old bytes only as historical evidence, not as a second runtime format.
The retained `import/skill-identities.json` catalog and upstream source observations
need not change. Regenerate `import/compiled/roles.json` and its transition, the
older executable `current/` seed, and the latest published endpoint plus all
current policies, manifests and test pins that depend on those outputs. Audit
that dependency set explicitly: a bulk digest replacement is not a provenance
proof. The directory named `current/` is a reconstruction seed, not the latest
validated endpoint recorded in the implementation document.

### Remove redundant branches while preserving live responsibilities

Replace the `GemInventoryPolicy` V1/V2 wire variants with its current V3 shape and
replace `DirectSkillInputPolicy` V1 with its current V2 shape. Rebuild their
producers and fixtures rather than retaining compatibility readers. The Direct
`version2` runtime flag and version-dependent `sets()` dispatch can then disappear;
keep the actual source-frame checks and make incomplete disposition coverage
explicit in the current model.

Do not mistake older implementation names for dead behavior. The validated
`60827bda…` predecessor still has two `primary_skills` inventory entries (Pain
Offering and Frost Bomb), six `primary_dispositions` and 514 support entries.
`AwaitUsage`, `BoundPrimaryUsage` and `PendingPrimaryInventory` therefore remain
live unless those two entries are migrated to the general checked-disposition
path with equivalent usage destinations and refusal tests. Likewise, potential
native Skill membership, ordered source effect construction and primary Action
correspondence are different facts; do not merge their proof obligations.

Remove the explicitly historical support-schema acceptance in
`gem_inventory.rs`: the ordinary support inventory currently accepts either an
empty gameplay Skill membership or the old singleton primary solely for replay.
After rebuilding those fixtures, retain only the reviewed current membership
condition. In Direct source handling, absent `source` and explicit empty `source`
are different loader dispatches, not old/new wire formats; preserve that
distinction. Keep MAIN/CALCS, stat-set/minion selection and descendant accounting
in `account_reference`, because construction observations do not replace them.

### Narrow consumer cutover and completion checks

First use complete construction facts for a reviewed secondary-switch receipt on
an exact singleton physical source. Missing declared effects, extra constructed
effects, unavailable observations or mismatched source/owned joins must refuse
that receipt. Multiple stat sets do not themselves make a second effect, and Vaal
status is not a prerequisite for the proven singleton non-consumption. Keep the
independent exact-primary-true adapter and its raw-token checks unchanged.

Generated and manual Direct receipts must additionally establish their actual
catalog-versus-fallback construction dispatch. Do not borrow a catalog singleton
merely because a primary correspondence resolves. A support's primary role alone
also cannot account for count/global fields when it can supply another effect.
Keep those responsibilities Pending until their exact consumers are proved.
Retain current live output, provider, origin, preset and whole-census checks;
no field receipt alone closes a preset.

Required regression coverage includes missing required fields, explicit
unavailable observations, missing primary/Command references, ordered duplicates,
late additions, display order, separate stat sets, forged catalog claims,
foreign/stale mappings and bounded nested work. Reuse the independent
`owned_secondary_effect_switch_source` and `owned_preset_reference_selection_source`
component tests; they remain source evidence rather than full-build parity.
Adapter tests must cover changed outputs/providers, cross-preset borrowing,
unsupported tokens and atomic refusal on budget exhaustion.

Finally reimport all five unchanged originals. Authenticate complete draft and
sidecar inverses, permitting only the explicitly reviewed field-accounting delta
and dependent provenance identities. Preserve all native query/requirement/rule
content and unrelated Pending obligations. Record any retired obligation only
after the existing whole-preset gate proves every applicable consumer; the role
format migration itself retires none.
