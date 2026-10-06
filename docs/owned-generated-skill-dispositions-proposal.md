# Proposal: generated skill source-field accounting

**Status:** Proposed; owner review required before implementation.
**Date:** 2026-10-05.

## Problem and bounded outcome

The generated-input importer already proves exact selected providers and saves
preset-owned raw quality. The occurrence-usage importer separately projects the
requested count. Neither contract accounts for every other saved Skill/Gem
field, so the broad configuration obligation remains on those source rows.
This is distinct from implementing requested participation or reporting.

Original05 currently has 79 origins linked to configuration issue `01f2` and
five selected input issues. Its selected generated pairs 208/209, 226/227 and
243/244 have exact Tree/Item providers, raw-quality bindings and count projections.
Accounting for all fields could discharge those six links while preserving
usage issue `0503` and shared issue `01f2`. That result has not been implemented
or established. Complete native original builds remain **0/5**.

The intended change identifies a real retained owner for unresolved semantics.
It does not prove that a global switch is inert, resolve usage, complete support
discovery, or permit native evaluation through an incomplete input inventory.
Source05's actual extra-stat/Amulet observations remain finite reference evidence;
they must not become a source-hash allowlist or a general absence rule.

## Recommendation: explicit generated-input V2

Add `GeneratedSkillInputPolicy::PobSavedGeneratedInputsV2` with the **same fields**
as V1: `definitions`, `source`, `roles`, `catalog` and `rows`. V2 explicitly opts
into a separate private field-accounting pass after raw inputs and occurrence
usage have been materialized. Reuse the existing normalization commitment and
checked migration machinery; changing the variant changes the policy/release
identity. No new Core/Engine type, operation, participation rule or runtime
activation behavior is introduced.

V1 retains its exact historical behavior and bytes. It must not silently start
retiring fallback links because a newer executable discovers additional facts.
A successful V2 accounting result uses sidecar schema **21** and its matching
digest domain. Use that version only when at least one source row actually gains
the new accounting; unsuccessful attempts retain the otherwise applicable
sidecar version. The explicit V2 policy identity distinguishes the opt-in even
when no row succeeds.

| Approach | Benefit | Cost and boundary |
| --- | --- | --- |
| **V2 on the existing generated-input policy — recommended** | Makes historical behavior explicit and mirrors physical/Direct input-disposition versioning. Existing rows already supply the exact provider/raw-input authority. | Accounting is available only alongside that raw-input policy. Keep its implementation independent so an unsupported field does not suppress successfully imported quality or count. |
| Separate optional disposition policy | Separates accounting configuration from raw-input configuration and could later serve other source representations. | Adds another public policy, dependency commitments and migration/rebinding paths. It still needs the same exact generated/raw/count proofs; this checkpoint has no independent consumer requiring that extra surface. |
| Enable accounting implicitly under V1 | No additional serialized variant. | Reinterprets historical normalization under an unchanged artifact identity and contradicts V1's retained-obligation boundary. Rejected. |

The recommendation does not approve the separate
[participation contract](owned-skill-participation-proposal.md). Accepted
preset-owned generated inputs and usage applicability remain unchanged.

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

1. Review this narrow Import policy/version decision before writing code.
2. Add the V2 compiler/rebinding branch while preserving V1 serialization and
   historical behavior. Existing V1 test helpers must explicitly remain V1.
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

Original05's 24 dormant generated origins remain outside the selected-source
proof. In particular, set 2's Complete empty usage cannot be replaced by an
invented Pending obligation. Record actual admitted rows and sidecar versions
after validation; until then the baseline remains **79 configuration origins,
five selected Original05 issues and 0/5 complete native evaluations**.
