# Proposal: generated skill source-field accounting

**Status:** Selected-source accounting implemented and validated 2026-10-06;
archived responsibility accounting added 2026-10-07. Current validation and
baseline are recorded in [implementation](implementation.md).
**Date:** 2026-10-05.

## Problem and bounded outcome

The generated-input importer already proves exact selected providers and saves
preset-owned raw quality. The occurrence-usage importer separately projects the
requested count. Neither contract accounts for every other saved Skill/Gem
field, so the broad configuration obligation remains on those source rows.
This is distinct from implementing requested participation or reporting.

Before this checkpoint, Original05 had 79 origins linked to configuration issue `01f2` and
five selected input issues. Its selected generated pairs 208/209, 226/227 and
243/244 have exact Tree/Item providers, raw-quality bindings and count projections.
The checked accounting pass discharges those six links while preserving
usage issue `0503` and shared issue `01f2`. That checkpoint left 73 configuration-
linked origins; subsequent cached-output and archived-responsibility checkpoints
are recorded in the current plan. Complete native original builds remain **0/5**.

The intended change identifies a real retained owner for unresolved semantics.
It does not prove that a global switch is inert, resolve usage, complete support
discovery, or permit native evaluation through an incomplete input inventory.
Source05's actual extra-stat/Amulet observations remain finite reference evidence;
they must not become a source-hash allowlist or a general absence rule.

## Accepted direction: one current generated-input policy

Account for fields through the existing generated-input policy, using its
`definitions`, `source`, `roles`, `catalog` and `rows`. A separate private proof
pass runs after raw inputs and occurrence usage have been materialized. No new
Core/Engine type, operation, participation rule or runtime activation behavior
is introduced.

The owner first accepted an explicit V2, then clarified that backward
compatibility is unnecessary. Prefer replacing the current implementation and
reimporting/rebuilding affected data over parallel old/new behavior. Retain
useful historical source evidence without requiring its old importer semantics.
The implementation distinguishes regenerated results from stale proof artifacts.
The existing policy digest identifies declarative inputs, not importer code;
the declaration format and release receipts can therefore remain unchanged.
Fresh normalization is the only producer of this proof: there is no sidecar
restore/cache reader whose old result can acquire new authority.

Current configuration-input policies emit sidecar schema/domain 23. A generated-
input policy without configuration accounting emits schema/domain 22, including
attempts that conservatively retire no links. Schema21 identifies the earlier selected-output-only accounting proof;
there is no historical importer mode or restored-proof reader. The CLI reports
that schema and the SHA-256 of the actual written sidecar bytes. An unchanged
draft digest does not identify unchanged accounting proof.
Reimport all five originals and compare against stored pre-cutover evidence;
do not retain the old importer merely to regenerate that evidence. Failure must
preserve successful raw/count imports and unresolved obligations. A format
number alone proves no field correspondence.

| Approach | Benefit | Cost and boundary |
| --- | --- | --- |
| **Current existing generated-input policy — accepted** | Existing rows already supply the exact provider/raw-input authority. One implementation avoids carrying an obsolete behavior branch. | Rebuild/reimport affected data and invalidate stale results. Keep accounting independent so an unsupported field does not suppress successfully imported quality or count. |
| Separate optional disposition policy | Separates accounting configuration from raw-input configuration and could later serve other source representations. | Adds another public policy, dependency commitments and migration/rebinding paths. It still needs the same exact generated/raw/count proofs; this checkpoint has no independent consumer requiring that extra surface. |
| Retain both V1 and V2 behavior | Can replay the previous importer behavior. | Adds compatibility code without a current product requirement; avoid unless a concrete reference consumer needs it. |

The owner separately accepted the [participation contract](owned-skill-participation-proposal.md).
This accounting pass does not implement that contract. Accepted preset-owned
generated inputs and usage applicability remain unchanged.

## Private proof and reference reuse

Reuse `generated_skill_sources::resolve` for the selected preset, exact source
pair, provider and grant slot. Do not match source strings a second time. Private
receipts from `generated_skill_inputs::materialize` and
`usage_inputs::occurrences::materialize` must identify their actual emitted
bindings, target, values and owner. An origin link, member count or equal display
name is not a substitute for output correspondence.

The private accounting pass then checks an existing, attached Pending usage
obligation on that same preset's `intent.usage`. Reuse the principle of
`skill_input_disposition::attach_pending_usage`, adding a no-allocation lookup
for generated intent. An absent, Complete, detached or foreign-preset obligation
cannot witness deferral. Do not attach this issue to provider Item/Tree rows
outside the SkillSet.

For the two reviewed Djinn definitions, the existing compiled Direct action
adapters carry the same exact Gem/Skill IDs, game/variant identifiers, source
pin and catalogue commitment as the generated-input rows. Their singleton Actor
and child-action selector checks are independent of raw/final level and quality;
they establish reference correspondence, not action availability or execution.
Factor a private selector-only inspection over that already-compiled topology,
called **only after** the generated root proof. Do not fabricate a Direct
request, broaden manual-source admission or materialize a generated root as
Direct. Public physical/Direct resolution keeps its existing authority checks.

The reviewed Firebolt row has no Gem reference selectors or descendants. Its
bounded path can prove actual absence; it must not invent a default Action or
stat set. Any added selector without an existing, applicable checked adapter
retains fallback. Shared mapping requires exact identity and source commitments,
not matching names or an index that happens to be in range.

## Full-field obligations

Before either row of a Skill/Gem pair loses fallback, account for every actual
attribute and descendant under the existing strict framing and work limits:

| Field family | Required disposition |
| --- | --- |
| `gemId`, `variantId`, `skillId`, `nameSpec`, group `source` and `slot` | Exact shared provider/catalogue correspondence, preserving item-use and tree-provider identity. |
| Saved `level` and `quality` | Existing provider-level proof and exact emitted preset raw-quality binding. This cannot replace the provider's level writer. |
| `count` and `groupCount` | Exact existing occurrence/group-precedence projection and emitted typed usage values. A saved `nil` token uses only the already-authored count recipe; the accounting code introduces no count-one default. |
| Group/Gem `enabled`, `enableGlobal1/2`, `includeInFullDPS` | Reviewed saved syntax and explicit unresolved usage/reporting responsibility on the existing same-preset Pending obligation. No computed participation, switch non-applicability or FullDPS result is asserted. |
| `mainActiveSkill{Calcs}`, `skillMinion{Calcs}`, `skillMinionSkill{Calcs}`, stat-set selectors and child maps | Exact reviewed source-reference semantics and singleton Actor/action mapping, including actual missing values and ignored legacy attributes. Unknown actor choices or selectors refuse accounting. |
| Empty `label`, generated `corrupted`/`corruptLevel` sentinels | Narrow source-representation proof for the admitted syntax; no generic corruption default, ignored unknown value or blanket presentation classification. |
| Any other field, child, namespace or malformed/duplicate value | Retain the whole pair's fallback until its disposition is established. |

Do not erase source `nil`, absence and false distinctions while checking these
fields. An unproved field prevents discharge, even when another field has a
known output. Accounting failure leaves successful quality/count imports and
their original obligations intact. Only the fully proven pair gains the real
usage issue link and loses its applicable configuration fallback links. Do not
retire the shared configuration issue or change draft values, allocation
watermarks, query selections or other source origins.

## Implementation and acceptance gates

1. Accepted: account for generated fields through the existing Import policy.
2. Implement one current accounting path and rebuild affected data. Choose the
   simplest explicit identity/invalidation change; do not add a compatibility
   branch solely to preserve earlier development artifacts.
3. Share the selector-only checks and add private materialization receipts plus
   the independent accounting pass. No source execution occurs during Import.
4. Test Tree and Item sources; repeated/manual/generated identity separation;
   absent, empty and literal `nil` source; malformed/duplicate/unknown fields;
   extra descendants; missing, dormant or ambiguous providers; exact source-pin
   mismatches; raw/count output mismatches; and absent/Complete/foreign/detached
   usage obligations. Unknown actor/action/stat-set selectors must retain
   fallback. A rejected accounting attempt must preserve its successful imports.
5. Use synthetic identities in maintained tests and all five unchanged real
   originals in the historical checkpoint. Check every changed and unchanged
   origin against the stored predecessor, preserving allocator state, all 110
   queries, draft values and issue IDs. Census other originals rather than
   hard-coding the implementation to Original05 or assuming only six links can
   qualify across the whole corpus.

## Archived source responsibility (2026-10-07)

The selected-output proof above remains unchanged. Archived generated rows can
also have correctly scoped unresolved owners even though their exact provider
cannot yet be resolved. Requiring a successful selected-provider binding before
acknowledging those owners accidentally leaves their known saved fields on the
global configuration obligation, coupling independent presets.

The existing generated-source resolver now separately recognizes source syntax
on an explicitly nonselected preset. It shares the exact configured definition/
source identity and the pre-admission source/slot duplicate census. This private
receipt has no `GeneratedSkillKey`, provider, native input or usage value. Missing
or invalid active selection is not evidence that a row is archived.

A deferred accounting proof requires all three existing same-preset Pending
inventories, with their expected codes and actual source attachment:

- Generated inputs retain unresolved provider, saved level and quality ownership.
- Usage retains counts, global effects, participation and reporting intent.
- Support origins retain runtime source discovery and ordering responsibility;
  one saved Gem child does not prove a complete effective support inventory.

It never creates an issue or turns a Complete inventory back into Pending. Each
row still needs the full generated field grammar, typed saved level and quality,
the existing deferred usage codecs, and exact MAIN/CALCS selector correspondence.
Generated missing/`nil` count/global fields are recognized only as unresolved
source syntax; they produce no native default or Boolean. Ordinary values still
use the same typed codecs. Unknown attributes, malformed values, unsupported
selectors, ambiguous identities and live physical/generated-output links refuse
this proof. The selector inspector proves topology, never provider authority.

Only the admitted source rows gain the exact preset and its three retained issue
links, replacing their configuration fallback. They remain `Contributes`; raw
source, draft values, IDs, allocator state, active axes, queries and every issue
completion remain unchanged. A source-kind or skill name alone cannot authorize
this change, and no production branch recognizes an example build.

The audited Original05 candidates are seven archived Djinn pairs (14 origins),
on skill sets 3, 5, 6 and 1. The remaining four Warrior pairs lack the required
configured correspondence, and set 2's Firebolt lacks a Pending usage obligation;
those ten origins must retain fallback. These observations are test expectations,
not runtime allowlists. All-five publication compares against pinned earlier
sidecars; see the current plan for executed results and remaining blockers.

## Verified selected-source checkpoint (2026-10-06)

All 287 normalization tests, 184 Import library tests, 16 Direct-disposition
and eight Direct-source-action tests pass. The five-build CLI reimport test
passes in 5.89 seconds against stored pre-cutover imports, using the existing
attribute-step release. Original01 changes exactly four origins (210/211 and
214/215), Original05 exactly six (208/209, 226/227, 243/244), and Originals02–04
change none. Every changed pair replaces only its configuration fallback link
with the exact already-existing same-preset Pending usage issue. The complete
drafts, allocation watermarks, saved selections and all other origin records
are unchanged. All five regenerated sidecars use schema21 and match their CLI
byte counts and SHA-256 reports. The package retains 110 queries.

Evidence is `runs/owned-generated-field-accounting-01/validation.json` and
`runs/owned-generated-field-accounting-tests-01.log`. That checkpoint used
the now-retired `tests/owned_generated_field_accounting.rs` migration harness.
Its stored output remains historical evidence; current all-five preservation is
covered by `tests/owned_boolean_publication.rs`, while maintained Import tests
retain the positive/negative ownership proofs. The old harness compared stored earlier output;
there is no retained old importer mode. Two failed synthetic controls were
corrected at their actual boundaries: absent/empty source can already have
physical provenance, and duplicate XML attributes fail decoding before
normalization. No production validation was relaxed.
