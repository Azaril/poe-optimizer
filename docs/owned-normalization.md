# Owned import and offline identity compilation

Status: conservative normalization with explicit equipment scopes, 2026-09-15. This is an import boundary,
not a numerical evaluator or full D1 completion. The [domain architecture](domain-architecture.md)
and [migration plan](architecture-migration.md) control the end state.

## Boundaries

The production normalizer accepts immutable source evidence, a caller-owned fresh allocator
state, exact owned registry/schema/mapping/skill-role/reward-policy/item-policy artifacts, a normalization policy and
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
4. `OwnedRewardPolicy` compiles a bounded finite outcome table over those recipes. Game
   keys/defaults/options and fixed parameters are injected; no source callback is run.
   It resolves reward definitions through exact mappings and validates their direct
   parameter declarations. Missing or partial schema stays unresolved. A known input
   contradiction rejects the artifact; this never certifies effects or numerical rules.
5. `normalize_fresh` performs deterministic occurrence assembly through those injected
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
values/modifiers, quality, remaining reward semantics, scope, configuration destinations, encounters, generated
providers and saved active selections are still incomplete. Every imported selection has
explicit pending obligations. This path cannot yet finalize a complete original request.
Cached PlayerStat/MinionStat outputs remain source-only and are never scenario inputs.

The caller supplies any ordered query list; no production 22-metric or named-build list is
embedded. Known player targets can be represented; action/owned-actor correspondence that
has not been converted remains pending. All rows survive with their original query IDs.

## Explicit equipment loadouts

`NormalizationPolicy.equipment_loadouts` supplies exact source-slot relations. Each
`EquipmentLoadoutRule` names a source slot, its mapped owned equipment-slot definition and
`ImportEquipmentScope::Shared` or `Selected { loadouts }`. These loadout keys are import-local
correspondence labels; they are never cast into Core IDs. The normalizer allocates fresh
`WeaponLoadoutId` values only for keys exposed by observed, uniquely identified ordinary
slots, including explicit empty slots. The same key is reused across independent item sets.

A rule must agree with the exact equipment mapping and any known schema scope. An unknown,
duplicate or undecodable slot does not acquire a fallback scope. The all-five path keeps
unrecognized equipment/passive/skill scopes and global membership closure pending. Known
loadout members can now be selected explicitly during finalization; this does not make a
partially imported build finalizable. Saved active-weapon selection is still unconverted.

The required field is part of policy digest domain `owned-normalization-policy-v3`. Earlier
policy JSON without it rejects; an explicit empty rule list preserves unresolved scope.
Sidecar version 7 records source-to-owned loadout origins and binds that policy. Loadout
source strings, UI state and correspondence never enter the native evaluator.

## Explicit physical-gem quality

`NormalizationPolicy.gem_quality` is required. `Unconverted` retains pending quality;
`Attributes` supplies an exact schema identity, a quantity `ValueRecipe`, an attribute name
for the kind and finite exact source-kind mappings. A `Missing` kind mapping is an explicit
reviewed default for an absent attribute. A present empty, unknown or undecodable kind does
not fall back to that mapping.

The amount recipe uses explicit attributes and a Pending missing-value policy. A known
zero is a present quality selection. Missing, malformed, ambiguous, unavailable and
out-of-schema amounts remain distinct pending diagnostics; none becomes an implicit zero
or absent quality. Kind, unit, range and any known enclosing Gem quality membership must
agree. Identity-only Gem schemas can retain this independently bound authored value while
their input/rule coverage remains unresolved. Collection closure is unchanged.

All five originals exercise the converter through injected ordinary-quality data: 478
physical gem amounts, including 448 zeros. The source default-kind rule is Import data;
Core/Engine receive only the owned kind and quantity. Policy identity uses v3 and the
correspondence sidecar uses v7; stale policies must be explicitly regenerated.

## Injected item-line conversion

`OwnedItemLinePolicy` is an import artifact bound to an exact owned schema. Its bounded
literal/capture patterns use the existing lexical codecs and emit owned template, level,
quality, parameter or modifier declarations. Metadata rules retain provenance separately;
affix tier metadata is never a second applied modifier or an instruction to clamp an
explicit roll. Game names, units, IDs and supported domains are injected policy/schema data.
This layer contains no Lua patterns, source callbacks, UI state or general interpreter.

`NumericCapture` matches a maximal ASCII numeric token under an explicit `DecimalSyntax`
and sign policy (`Optional`, `OptionalMinus` or `Forbidden`) before its semantic codec
runs. It does not retry a shorter token or select a pattern because a decoder happens to
succeed. The combined resistance
[items.json](../data/owned/poe2/3887ae68/resistance/items.json) and
[item-source.json](../data/owned/poe2/3887ae68/resistance/item-source.json) policies cover
both fixed values and reviewed integer ranges. Fixed values allow an optional plus/minus;
inner endpoints allow an optional minus. Plus-prefixed and bare ranges are disjoint rules.
Outer-minus range inversion, decimal endpoints and unreviewed grammar remain unsupported
by that package, even though the generic lexical primitive supports other declared syntax.

Item-line policy **v4** adds explicit numeric projection and retention of known rolls under
Partial parameter membership. Versions **v2** and **v3** remain accepted with their original
wire bytes, identity domains and semantics; v2 cannot use unrounded interpolation, and
neither accepts v4 projections. Policy **v5** adds contextual template-parameter bindings:
a shared source header resolves only against the uniquely selected template's declared
slot and constraints. Standalone conversion keeps the value deferred; aggregate validation
handles duplicates, unresolved targets and default suppression. Existing v4 bytes and
meaning are preserved, and no Core source-parser field is added. Normalization sidecar
**v12** records the deferred evidence and validated assignments; it also distinguishes explicit Gem input recipes from the earlier empty-schema inference. Item-source policies use
**v4**. A
`Property { property }` line value reads an explicitly supplied Boolean fact and can emit
only into a Modifier-owned roll slot. Missing context/key remains `MissingProperty`;
direct text conversion cannot invent a false value. Capture errors still precede missing
property inputs. The supplied map is bounded and charged before matching and decoding.

The source policy requires exact `property_bindings` from labels to local property keys.
The admitted source grammar extracts runs of ASCII letters or underscores from tag values;
punctuation, digits and whitespace separate tokens. Case is preserved and aliases are
explicit. Binding labels must themselves satisfy that token grammar. This small adapter
does not embed a source pattern interpreter. For an admitted single-member grammar, it
scans every property tag, preserves every token and decoded span, and supplies true/false
only for keys requested by that rule. Unknown
labels or recognized labels without a corresponding consumed property block the whole
line. No tag list means an empty label set only in this scoped source grammar; it does
not prove absence of catalysts, quality, other headers or unconverted item effects.
These local keys remain Import data. The owned Modifier receives ordinary schema-bound
Boolean ParameterAssignments, not source labels or a source condition interpreter.
Policy decoding and encoding both charge every derived property reference against text
limits. Token scanning, aliases, variable-length comparisons and property-map expansion
consume explicit work/output budgets before they execute.

`InterpolateOffset` explicitly computes `a + f * (b - a)` for a supplied fraction in
`[0,1]`, then divides by the declared positive quantum. `SymmetricHalfOffset` applies
`floor(x + 0.5)` for nonnegative scaled values and `ceil(x - 0.5)` for negative ones,
then multiplies by the quantum. Their literal floating-point order is intentional; it
is distinct from stable `Interpolate` arithmetic and mathematical nearest rounding.
Every intermediate/result must remain finite, including `b - a` at fraction 0 or 1.
Both endpoints and quantum must share Integer kind or the exact Quantity unit, and
schema bounds still apply. These are explicit import compatibility operations, not a
change to native domain rounding semantics or a source-runtime dependency.

V3's `InterpolateUnroundedOffset` keeps exactly `a + f * (b - a)` without rounding or an
endpoint shortcut. It requires ordered Quantity endpoints in the same exact unit, a finite
fraction in `[0,1]`, and finite intermediates and result. Integer endpoints are not admitted.
Existing rounded operations retain their distinct contracts.

V4's nonrecursive `NumericProjection` composes four declared stages: a Quantity capture or
unrounded offset interpolation; optional negation; `Exact` or `SignificantDigits { digits }`
decimal transport; and `SignedQuantity`, `Magnitude` or
`NegativeDirection { invert }` output. Decimal transport formats to 1–17 significant digits
and parses a finite quantity; the reviewed range recipe declares 14 explicitly. This
boundary is observable before later rounding and must not be replaced by an implicit
numeric cast. Quantity results retain their exact unit; direction produces a Boolean.
Source decoding and every arithmetic intermediate must be finite, and work is charged
before numeric formatting. Negative zero retains its temporary sign until direction is
projected, including explicit qualifier inversion. Core receives canonical quantities and
Boolean facts, never numeric lexemes, parser state or a sign-bearing source object.

Every line retains its text, index and known/pending outcome. Multiple matching rules or
capture boundaries stay ambiguous. Interpolation needs an explicitly supplied range
fraction; rounded variants also require their declared quantum. `convert_text` supplies no
implicit range/default. Aggregate fields and member lists include only declarations
individually admitted by the unique item template. Conflicting headers/parameter
assignments stay pending; listed members of partial sets may survive without implying
that the set is complete.

V4 may retain a uniquely admitted modifier's known rolls under Partial parameter membership.
Every known required slot must still be supplied with the exact owner, type, unit and
allowed value; unknown slots or invalid required values cannot be skipped. Converted
modifiers carry that membership closure through aggregation. Normalization preserves a
Pending roll collection and diagnostic instead of promoting it to Complete. V2/v3 retain
their previous Pending outcome for Partial parameter membership. None of these rules closes
modifier membership, source attribution or whole-build coverage.

Admitting an unrounded source component establishes a raw input, not a final effective
value. Required corrupted-base and magnitude inputs need separate producers and eligibility
proofs. The reviewed ordinary grammar can emit an explicit per-line corruption factor of
1 only because an absent per-line factor tag means unity; the item's `Corrupted` header
does not supply that factor. Unsupported or malformed tags remain blocked. Ranged, fixed
and baked encodings have separate admission obligations, and valid raw capture does not
prove source formatter/cache authority. Keep that distinction in Import diagnostics and
optional oracle comparisons; the native evaluator executes owned numeric semantics.

The source-layout adapter is an additional injected Import artifact bound to the exact
item-line policy. It supports a reviewed canonical preamble and one consumed text chunk
followed by ordered ModRange children. It proves source category/member positions before assigning
fractions; XML IDs are never raw line numbers or counts of emitted owned modifiers. A line
can emit several owned values while occupying one source slot. Unknown membership or
unimplemented list-changing lifecycle leaves attribution pending. Multiple text resets,
unsupported tags and child operations cannot be stripped into apparently known semantics.

An immutable attribution plan retains original/semantic text, source spans, layout evidence,
flat range-write records and the winning write. Inline fractions precede later XML writes;
repeated valid XML IDs apply in source order. The existing injected converter still owns
interpolation quantum, rounding and owned slot destinations. The normalizer preserves raw
line outcomes and fresh modifier IDs, with a version-10 attribution sidecar. Item parameter
and modifier collections retain pending closure; attribution is not whole-item evaluation.

Source policy v3 requires `template_defaults`, including an explicit empty list when none
are reviewed. Each exact template may declare literal parameter fallbacks and separate
item-level/quality absence policies. A parameter fallback lists every admitted source
header spelling whose presence blocks that fallback. The adapter requires a complete
source layout, checks raw headers even when their rules emit only Metadata, and rejects
conflicting relevant headers as a default scope. The aggregator separately suppresses
fallback for every authored or pending occurrence of that exact slot. Constructors check
owner, membership, type, unit, value range, options and default uniqueness through shared
input-schema validation. Template scans, schema reads, text, output and comparisons remain
bounded; encoding preserves tightened constructor limits.

Defaulted parameters are stored separately from located authored parameters. The normalizer
combines their owned values while retaining default provenance in each item sidecar and
the proven template in source attribution. Scoped absence becomes Known(None) only when
no corresponding field is present. Direct text conversion cannot activate these defaults.
No synthetic source lines, runtime defaults or conditional-default interpreter are used.
The integrated evidence for this phase is recorded separately in the implementation log.

Source presentation positions are consumed before matching semantic rules: an item titled
`49% increased Attack Speed` does not acquire that modifier. Unsupported tags preserve raw
text and a bounded union of raw/body candidates; cleaned text is diagnostic input only until
its meaning is admitted. A blocked competing parameter/header invalidates an earlier known
assignment. An unrecognized source line may consume the following line as a combined
modifier; that following row stays blocked until its independence is known. Arbitrary
reminder-block control flow is declined as an unsupported item lifecycle, not replayed. Source membership proof and individually admitted literal facts remain distinct.
Malformed overlays invalidate earlier fractions conservatively; a later valid write can
recover its exact proven target. No source checkout or exporter-version authentication is
implied by the policy's offline provenance pins.

The [item modifier properties and scaling](owned-item-scaling.md) slice now converts the
untouched original05 ring's nominal range and five property predicates through the
production policy. Its eight source uses remain distinct. New nominal resistance
families retain Partial numerical rules, so that value is not an effective contribution.
Existing fixed-value definitions/programs retain their prior meanings. Canonical catalyst
selection/amount and Sapphire-specific missing-input defaults are now converted using
reviewed source data. Amount20 is the input to use only when a catalyst is enabled, and
its default provenance remains distinct from an authored header. Ordinary quality for
other bases, complete ordered scaling and rune/Bonded semantics remain unconverted;
whole-item and whole-build closure remain open.

The original Grand Spear has explicit attack speed 49% while its affix metadata names a
26–28% tier. The import fixture preserves 49 once and retains the discrepancy. The original
Ashen Staff has a ranged Firebolt grant and no Item Level header. Its generated source
skill is not a physical gem and is separate from the manual Skeletal Sniper. The optional
source oracle confirms grant levels 1/11/20 at fractions 0/.5/1; this does not establish
native provider resolution or full build parity.

Finalized item records permit an explicitly unspecified item level; owned wire version 3
requires a numeric or null field and rejects omission. Drafts distinguish `Known(None)`
from `Pending`. A converted numeric header becomes `Known(Some(level))`; the current
normalizer keeps every other case pending. No matching rule is not an absence proof:
a partial policy can miss a real header. A later source-format-scoped proof or explicit
owned authoring must establish unspecified. Never substitute zero, equipment requirement
or granted skill level. Native rules leave a demanded missing fact unresolved while unused
facts do not block component effects. Provider binding and complete selected-request
finalization still need their own evidence. The provenance sidecar uses version 7 for the
source-layout identity/attribution; owned input and draft protocols remain version 3.

## Identity, bounds and publication

`NormalizationArtifacts` groups references; it is not a validation token. Normalization
checks the exact registry/schema/mapping/role/reward-policy/item-policy/source-layout bindings and PoB2 source family before output.
The allocator must have the source lineage and a watermark at least as high as the source
importer's final state. Every owned ID is allocated above that watermark. Work happens on
a local allocator; errors return no partial result and cannot consume the caller's state.
The host publishes draft, sidecar and new watermark together under its owner or CAS.
Repeating a fresh import is not restore, changed-source migration or concurrent allocation
authority. Those operations need separate revisioned contracts.

The version-7 sidecar records source hash/schema/revision, before/after watermarks, policy plus query
identity, exact artifact identities (including reward, item-line and item-source policies), draft digest and one origin entry per source element.
Its targets are a closed enum of current owned occurrences/issues. Many source rows may
refer to one real pending collection issue; candidates never allocate hypothetical uses.
Unknown semantics are not automatically called presentation metadata. The current broad
unconverted obligations are conservative and need finer ownership as each domain lands.

Inputs, policies, queries, intermediates, work, issues, origin links and output bytes are
bounded. Attribute indexes and cached group origins avoid repeated source scans. Limits
fail explicitly rather than truncating source or silently claiming complete collections.

`normalize-owned INPUT --policy POLICY --registry REGISTRY --definitions DEFINITIONS
--mapping MAPPING --roles ROLES --rewards REWARDS --items ITEMS --item-source ITEM_SOURCE --queries QUERIES --output NEW_DIRECTORY` is the thin CLI
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
  be the completion mechanism for a permanently absent target. The import-side projection
  ledger now implements this distinction; independent availability resolution is still
  required before calling any original comparison complete.

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

## Recorded reference routing

`RecordedReference` reads a bounded recorded CLI evaluation and validates its result,
source hash, backend and structured snapshot bindings. A confirmed absent selected minion
routes to `KnownUnavailable`; no snapshot or an unavailable metric for a present actor
cannot prove absence. No reason-string parsing, invented actor or zero substitute is used.

The host filters requested templates before fresh normalization, then constructs
`ProjectionPlan` from the exact produced draft queries and explicit caller correspondences.
`validate_normalized` binds source, artifacts, draft and query preset while build selection
is still pending. Full selection binding and finalized-request joining remain stricter
operations: unresolved loadouts cannot be fabricated merely to reach them. Row joining
rejects missing/extra/duplicate IDs and preserves caller order independently of result order.
A raw ID association only locates result rows; it certifies neither values nor finalization.

The all-five breadth consumer retains 110 ledger rows: 104 Evaluate routes and six
reference-confirmed absent-target routes. Evaluate routes can still contain pending owned
queries. The ledger does not feed optimizer objectives or claim independent availability
or numerical parity. Exact compressed reports are offline tests, not runtime game data.
Their source/report bytes are hashed exactly; the companion expectation manifest hash
explicitly normalizes line endings so Windows and Linux checkouts agree.

## Constructed base prefix evidence

`compile-owned-item-layouts` consumes a finite catalog of generated-prefix presence
and an exact predecessor-bound policy. The optional reference exporter obtains this
catalog from actual constructed flask/charm buff fields. Empty string members still
occupy source line positions. Item kind, weapon presence and copied source pins are
not evidence of absence or authenticated construction.

The native converter requires a bijection of source names to existing owned templates
and their exact admitted literal header rules. It refines only unresolved prefix
absence; present and unsupported shapes remain unresolved, and contradictory prior
knowledge is rejected. Source policy identity changes, while the item line policy,
owned schema/rules, query manifests and unrelated source declarations remain intact.
See the [versioned finite artifacts](../data/owned/poe2/3887ae68/item-layouts/README.md).

Prefix absence is only one source-layout prerequisite. Other headers, implicit/rune
members and unknown source lines can still block range-index attribution or omitted
input defaults. This converter introduces no source-layout field into Core and grants
no whole-item, modifier-order or build completeness. A future metadata-preamble seam
must classify explicitly reviewed zero-member rules; admitting every header as a no-op
would discard meaningful values such as Spirit, Charm Slots and corruption state.

## Explicit zero-member preamble rules

Source policy v5 adds `PobExportedSingleTextPreambleV1` with the v4 flag bindings and
an injected list of metadata rule IDs. A reference must identify an existing Header
recipe with nonempty, exclusively Metadata emissions. Missing, duplicate, mixed semantic
or non-header recipes reject. Rules remain in the item-line policy; the adapter adds no
runtime source field or spelling-based special case for a particular build.

A configured metadata header consumes no modifier index after the selected base and
before modifier insertion. Duplicate source occurrences and raw text remain in evidence;
no inventory identity is inferred. Normalization uses sidecar v12 (v11 before explicit Gem input recipes) and binds the new
exact source-policy digest. V3/v4 serialization, domains and behavior remain unchanged.
V5 preserves v4 flag behavior even with an empty flag-binding list. Metadata validation
shares the schema-work budget with default validation; encoding also enforces that budget.

Metadata is not universally inert source state. In the supported fresh single-text path,
Unique ID does not change the reviewed quality/catalyst absence rules, but it can affect
later source quality normalization and identity reconciliation. Those uses are not native
fields and must be converted explicitly if a future import boundary requires them.
Selection tags are pre-scanned before source header dispatch; the v5 raw metadata dialect
rejects closed variant/version/group tags, the Foil Unique marker and square/angle markup
that could conceal controls. Ordinary range/tag/rune braces inside metadata are preserved
without creating modifier writes. See the [finite successor and source references](../data/owned/poe2/3887ae68/item-metadata-inputs/README.md).

## Finite configuration reward conversion

Each actual ConfigSet contributes its own reward selections to a ChoicePreset. The importer
collects exact direct Input values once per scope, then matches injected Boolean/String
lanes and applies declared precedence/defaults. Valid same-name typed alternatives remain
separate candidates. Unknown input names/scopes, matching placeholders, malformed or mixed
value shapes prevent absence/default inference. An error cannot fall through to a lower
tier. A unique legacy direct Config is supported; source active-set UI state is not replayed.

A chosen fixed outcome creates a fresh owned RewardSelection with injected parameters;
explicit None creates none. Both the global reward collection and each contribution remain
pending because a finite rule list, even empty, does not prove catalog completeness. Source
origins link real occurrences or pending obligations, never placeholder reward records.
The CLI requires a bound reward policy, including when the caller intentionally supplies
an empty partial policy.

The first offline metadata fixture covers all 17 config-controlled quest families (nine
checkboxes/eight lists). All are already present in schema40 configuration metadata; only
the old narrow quest projection omitted the lists. The exporter checks injected metadata
and pinned source hashes without Lua execution. Fixed outcome input declarations do not
parse stat text, compile reward effects, establish completion of the separate 12 progression
point rows, or make a complete native build. Game-specific names/defaults live in the
reviewed fixture, never production normalization code.

Next convert domain-owned config/reward/scenario fields, loadout overlays, provider/target
correspondence and saved selections. Revisioned repair can proceed independently; it must
not delay all-five model validation. Follow with owned local-item, conditional, support and
grant rules plus shared player/minion resolution. Retire the corresponding legacy consumer
and preserve its useful numerical tests at each replacement checkpoint.
