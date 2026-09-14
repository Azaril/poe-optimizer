# Owned import and offline identity compilation

Status: first conservative implementation, 2026-09-14. This is an import boundary,
not a numerical evaluator or full D1 completion. The [domain architecture](domain-architecture.md)
and [migration plan](architecture-migration.md) control the end state.

## Boundaries

The production normalizer accepts immutable source evidence, a caller-owned fresh allocator
state, exact owned registry/schema/mapping/skill-role artifacts, a normalization policy and
an ordered caller-supplied query list. It returns a validated [owned draft](owned-drafts.md)
and an optional import-owned correspondence sidecar. No selected PoB UI view, source VM,
legacy game-data snapshot, numerical evaluator or skill-name profile enters this API.
Core does not import XML, external IDs, raw config keys, Lua callbacks or this sidecar.
Removing provenance does not change the owned draft or a later finalized request.

The pipeline is deliberately staged:

1. `SourceProjectEvidence::collect` preserves every source element, direct content,
   attributes and exact occurrence identity. Failed lexical fields or convenience config
   projections remain observable. Exact-key joins distinguish missing, unique, ambiguous
   and unresolved keys; an undecodable sibling cannot prove a unique match.
2. `compile_fresh_owned_skill_catalog` runs offline over injected identity facts. It stages
   fresh owned Gem/Skill IDs, exact or ambiguous mappings and import role metadata. It
   never marks an identity-only definition's entire input schema known. Source pins,
   absent-support and absent-materialization policies and compilation provenance are explicit. The final role artifact
   binds the exact final schema/mapping; its registry may extend the compiler's staged
   registry with other domains. Stage receipts are provenance, not proof of all conversion.
3. `ValueRecipe` chooses the first present tier, applies its duplicate policy, then invokes
   the bounded lexical codec. False, zero, empty and unavailable values are present. A
   selected error cannot fall through to another tier or a missing-value default. These
   recipes neither traverse source trees nor authorize writes to Core.
4. `normalize_fresh` performs deterministic occurrence assembly through those injected
   artifacts and validates the resulting DraftSession. Missing semantics become real
   pending fields or collection obligations, never empty effects or numerical zero.

Package generation is explicit offline tooling. A native application consumes prebuilt
owned packages; ordinary Cargo builds must not start PoB or fetch upstream data. Future
rule compilation will emit project-owned domain operations, not a renamed Lua AST.

## First implementation

The normalizer stores all independent source item/skill sets and passive specs. It creates
one owned record per source item and one use per nonempty receiving occurrence, including
unresolved rune/tree sockets. Equal source text does not merge records. Repeated references
to one record do not certify physical stock. Exact duplicate or undecodable item keys leave
the reference pending. Source IDs and source ordinals are never cast to owned instance IDs.

Physical gem drafts require three independent facts: a reviewed source origin, a nonempty
exact gem identity, and injected `Physical` materialization evidence. The offline compiler
classifies materialization separately from active/support role using the primary effect's
`from_tree` evidence and an explicit policy for absent evidence. `ProviderOnly` and unresolved
materialization never produce a physical Gem, SkillUse or SupportAssignment; their source
rows keep real pending obligations. Manual-looking source groups cannot override provider
ownership. Name-only unknown entries and generated representations likewise remain pending.
Physical support children on generated groups require explicit reviewed origin prefixes,
physical materialization and a known support role; their unresolved provider target remains
pending. A support can target a sole authored active only when the policy permits it and
all relevant group roles are accounted. Unknown siblings or multiple active gems prevent
that inference. Trigger/payload and support-generated actor semantics remain to convert.

Every nonempty authored passive token is retained without inferring point accounting or
connectivity from its ID. Pool, weapon overlay, attribute choice and special allocation
access remain pending. Empty/malformed token positions and unknown descendant semantics
remain in exact source evidence and the preset's pending membership obligation. An
ascendancy or jewel-granted allocation is not rejected as an ordinary disconnected node.

Character levels and gem levels/enabled flags use injected scalar recipes. Item intrinsic
values/modifiers, quality, rewards, scope, configuration destinations, encounters, generated
providers and saved active selections are still incomplete. Every imported selection has
explicit pending obligations. This path cannot yet finalize a complete original request.
Cached PlayerStat/MinionStat outputs remain source-only and are never scenario inputs.

The caller supplies any ordered query list; no production 22-metric or named-build list is
embedded. Known player targets can be represented; action/owned-actor correspondence that
has not been converted remains pending. All rows survive with their original query IDs.

## Identity, bounds and publication

`NormalizationArtifacts` groups references; it is not a validation token. Normalization
checks the exact registry/schema/mapping/role bindings and PoB2 source family before output.
The allocator must have the source lineage and a watermark at least as high as the source
importer's final state. Every owned ID is allocated above that watermark. Work happens on
a local allocator; errors return no partial result and cannot consume the caller's state.
The host publishes draft, sidecar and new watermark together under its owner or CAS.
Repeating a fresh import is not restore, changed-source migration or concurrent allocation
authority. Those operations need separate revisioned contracts.

The version-2 sidecar records source hash/schema/revision, before/after watermarks, policy plus query
identity, exact artifact identities, draft digest and one origin entry per source element.
Its targets are a closed enum of current owned occurrences/issues. Many source rows may
refer to one real pending collection issue; candidates never allocate hypothetical uses.
Unknown semantics are not automatically called presentation metadata. The current broad
unconverted obligations are conservative and need finer ownership as each domain lands.

Inputs, policies, queries, intermediates, work, issues, origin links and output bytes are
bounded. Attribute indexes and cached group origins avoid repeated source scans. Limits
fail explicitly rather than truncating source or silently claiming complete collections.

`normalize-owned INPUT --policy POLICY --registry REGISTRY --definitions DEFINITIONS
--mapping MAPPING --roles ROLES --queries QUERIES --output NEW_DIRECTORY` is the thin CLI
consumer. It accepts XML or one PoB share code and requires all owned artifacts. It prepares
and validates draft.json, sidecar.json and report.json before creating the output directory;
existing paths are refused. A filesystem failure during publication is not an atomic
multi-file transaction. This command performs no calculation, legality check or numerical
coverage claim and is available without the PoB feature.

## Model work exposed by the five-build normalization

The following concrete gates remain before freezing D1/D2 semantics:

- **Independent passive-socket equipment contributions.** A selected Spec supplies jewel
  uses independently of the selected ItemSet. Allocation presets now carry equipment-use
  contributions, and Core composition/finalization unions the selected lists. The importer
  assigns each receiving use to its actual owning Spec while retaining pending membership
  and destination/activation facts. There are 21 such uses across the originals. Full
  source completeness and socket semantics remain to convert; the structural composition
  gap is addressed without expanding every preset combination or including all saved jewels.
- **Known-unavailable reference projections.** Some original oracle rows request a selected
  minion when there is no such source target. The parity ledger must preserve that known
  unavailable row and fixed denominator without inventing an ActorKey. Core metric requests
  keep concrete targets. A pending target is a truthful temporary import state, but cannot
  be the completion mechanism for a permanently absent target. Implement and test the
  projection distinction before calling any 22-row original comparison complete.

- **Source kinds do not determine owned definition kinds.** The identity-only compiler's
  Gem/Skill placeholders remain `Unmapped` schemas, not final physical-item declarations.
  Eighteen manual-looking gem rows across the originals have tree-granted primary effects;
  they already require a different materialization path. The full semantic compiler must
  permit zero, one or several owned declarations per source definition, including provider
  capabilities, without forcing PoB's Gem/UI categories into Core. Revisit the current
  kind-preserving identity mapping before the D2 package is frozen. Test project-authored
  definitions, physical and provider-only variants, and multiple capabilities from one
  provider. Import provenance can reference this conversion; runtime rules cannot depend
  on PoB source kinds or `from_tree` fields.

The next reference adapter should use an ordered projection ledger outside Core. Each row
routes to an owned metric request (possibly still pending) or a reference-known unavailable
projection supported by structured selected-target evidence. Six original selected-minion
rows have no selected target; other unavailable measurements do not imply that absence.
The ledger preserves all 110 requested rows while the Core request contains only concrete
semantic targets. Result joining must reject missing, duplicate or extra IDs and preserve
caller order. Exact source/selection, artifact and request bindings prevent stale reuse.
Copying reference absence into this ledger is not independent availability or numerical
parity; those claims still require owned resolution evidence. A missing mapping stays pending.
The first consumer is the breadth comparison harness, not optimizer objectives.

Choice presets now have independent reward contributions. The normalizer keeps these lists
pending until injected owned mappings and value policies identify actual outcomes. The pinned
quest-data audit establishes configuration ownership; it does not authorize inventing defaults
or relabeling automatically derived class effects as authored rewards.

Next convert domain-owned config/reward/scenario fields, loadout overlays, provider/target
correspondence and saved selections. Revisioned repair can proceed independently; it must
not delay all-five model validation. Follow with owned local-item, conditional, support and
grant rules plus shared player/minion resolution. Retire the corresponding legacy consumer
and preserve its useful numerical tests at each replacement checkpoint.
