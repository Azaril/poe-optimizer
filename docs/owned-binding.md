# Owned project composition and definition binding

This contract implements part of D1/D2 in the [architecture migration](architecture-migration.md).
The [domain architecture](domain-architecture.md) remains controlling: UI/import adapters,
owned inputs, injected definitions, resolution, calculation and search have separate roles.
PoB is an offline source and optional differential oracle. These APIs do not execute Lua,
reproduce its UI lifecycle or perform a calculation.

## Host flow

1. Load or directly author an owned project, standalone build, inventory, scenario and queries.
2. Choose character, equipment, allocation, skill and choice presets independently, with an
   explicit active weapon loadout. Compose a self-contained BuildSpec.
3. Combine the build with an explicit scenario and ordered queries into OwnedEvaluationRequest.
4. Load an independently versioned owned schema package. Bind the request to that index.
5. Later resolution consumes the owned request and compiled domain rules, produces actors,
   actions and applicability, checks legality separately, then prepares a reusable numerical plan.
6. Search changes semantic inputs through this same boundary. CLI, GUI and browser clients
   display shared diagnostics/results; none supplies independent game logic.

Steps 1–4 have library implementations. Draft repair, revisioned edits, full source normalization,
resolution and general numerical plans are still separate implementation gates. Current native
numerical callers continue to use legacy profiles until their shared replacement is validated.

## Projects and composition

[BuildProject](../crates/poe-optimizer-core/src/owned_project.rs) holds one occurrence registry:
loadouts, intrinsic item/gem records, rewards, equipment uses, allocations, authored skills,
support assignments and payload links. Typed presets select these exact occurrences. A saved
variant retains explicit IDs for each preset and its active loadout. There is no positional
pairing of source sets, default selected variant, hidden clone or Cartesian product materialization.

Project construction validates the whole registry and preset references, including unselected
records. Composition checks the entire optional inventory union before selecting records; an
unselected conflict cannot hide a duplicate identity or inconsistent rolled item. It then emits
only the selected referenced item/gem records and checks cross-preset dependencies through
BuildSpec. Missing providers, containers or payload endpoints are errors, not inferred matches.
Inventory-only item adoption requires an explicit edit; composition does not silently import it.

Allocation presets also contribute equipment-use IDs. Composition unions those IDs with the
selected equipment preset, so passive-socket equipment follows the passive setup while ordinary
equipment remains independently selectable. Duplicate IDs within either list reject; the same
ID in both lists selects one use. Distinct uses remain distinct even if they share one backing
item or destination. Input memberships are bounded before deduplication, and the combined build
still requires every supplying allocation/container. Composition neither allocates IDs nor
adopts inventory, selects providers automatically, or establishes socket capacity/legality.

Choice presets similarly contribute reward-selection IDs alongside character rewards.
These contributions are additive, without precedence or replacement. Alternative outcomes
must have one authored selection owner; automatically derived class/passive effects remain
rules and grants. For example, configuration-owned quest outcomes belong to choice presets,
while their effects can still use the selected character context. Composition does not replay
configuration callbacks or choose unspecified defaults.

Project/preset IDs share the monotonic occurrence domain and cannot collide with item copies
or other record kinds. The version-3 owned document codec accepts `project` alongside `build`,
`inventory`, `scenario`, `query` and `request`. Explicit null options and saved variant selections
roundtrip. The codec preserves authored choice-owner spelling while normalizing unordered tables.
Versions 1 and 2 reject explicitly. Allocation equipment membership remains required,
with no implicit empty-list default. Item level is a required optional field: explicit null
means unspecified, while omission rejects. Binding checks the template's item-level range
only when a value is supplied; it does not establish legality or provide a fallback for
calculation. Equal-ID records with null versus a number conflict, and removing a supplied
level invalidates content-bound snapshots. Semantic content digest domains remain unchanged
so unchanged numeric payloads keep their identity. The wire envelope and draft protocol
are versioned separately from calculation.

Four direct choice owners alias the corresponding empty provider path: Character, EquipmentUse,
Allocation and authored Skill. The exact declared choice slot is part of that identity. Either
spelling satisfies the same required choice; supplying both rejects even if the values agree.
Allocation-local choices share that same identity. Nonempty paths, generated skills, actions,
modifiers, supports and rewards do not acquire extra aliases.

## Binding results

[bind_owned_request](../crates/poe-optimizer-core/src/owned_binding/mod.rs) uses only the immutable
DefinitionSchemaIndex. Its diagnostic report binds the full canonical request digest and
DataIdentity, and retains every query row in authored order. The report has no deserialization
or prepared-plan authority. Consumers must inspect schema and selector status together.

| Status axis | Meaning |
| --- | --- |
| Schema Valid | No encountered schema violation or unresolved fact; no calculation or legality claim. |
| Schema Invalid | A known kind, unit, range, role, owner or membership constraint is violated. |
| Schema Unresolved | Missing/Unmapped schema or partial membership prevents a complete conclusion. |
| Selector SchemaBound | A schema-level address is established, such as Player. |
| Selector PendingResolution | A declared owned actor/action still requires activation and effect resolution. |
| Selector Unavailable | A saved root is absent, disabled, in an inactive explicit loadout, or not offered. |
| Selector Unresolved | Schema coverage prevents determining its address. |

An invalid metric role can accompany a SchemaBound player or PendingResolution action. Neither
selector status overrides schema Invalid. An unavailable query remains saved and is never
retargeted to another action. Work/issue exhaustion or an inconsistent index returns an error,
not a truncated report that could be mistaken for a successful binding.

Binding checks all five parameter sites, quality presence/amount, class/ascendancy membership,
point-pool scope, item destinations and exact socket ownership, support/payload roles, scenario
inputs, choices and requested outputs. Units must match exactly; equal dimensions do not imply
conversion. Zero and false are present values. RequiredOnce applies to concrete sites; partial
required-slot lists remain unresolved. Gem companion effects and unselected outputs remain
potential. An explicit selected action makes its exact skill/provider context and output choices
concrete, without instantiating every sibling definition.

A payload link with a known allowed skill intersection is potentially compatible. It is not
proof that a trigger fires, a support applies or the link contributes damage. Graph connectivity,
ascendancy budgets, stock conflicts, resource costs and numerical coverage remain later checks.

## Provider reachability

Global slot registration is not reachability. A provider starts with the selected class/ascendancy,
exact item template, equipment-use/modifier pair, gem/direct skill, allocation, reward or support.
Each grant step must be offered by the current context. Entering a Skill restricts outputs to its
explicit grant. Entering an Actor exposes only that actor's outputs, not its owner's siblings.

For provider P and actor slot A, traversing grant G enters the actor identified by `(P, A)`.
The action provider path becomes `P.G`; the actor does not become `(P.G, A)`. Generated skill
keys likewise retain their supplying parent. Distinct equal-definition instances stay distinct.
Nested equipment and support target dependencies propagate explicit loadout/enable availability.
Conditional activity and support-target actor inheritance are not inferred; later resolution
requires explicit semantic rules for those relationships.

## Offline identities and mappings

[owned_mapping](../crates/poe-optimizer-import/src/owned_mapping.rs) is import tooling, outside
the core/data runtime contract. OwnedIdRegistry allocates counter-based typed definition/slot
IDs without a source key or display name. Persisted contiguous history and tombstones prevent
reuse; retiring an owner requires retiring its declared slots first. Allocation/retirement
preflights resource limits atomically. validate_successor rejects history deletion, retargeting,
retirement reversal and rewritten reasons. Hosts must persist with compare-and-swap against
the previous digest; this library does not own a filesystem or transaction manager.

OwnedMappingIndex binds namespace, registry digest, schema DataIdentity, exact source revision
and file hashes, and a conversion-policy version. Selectors retain Missing versus empty text,
variants, owners, roles and source values. Exact lookup returns Mapped, Ambiguous or Unmapped;
there is no display-name guess, runtime allocation or fallback evaluation. One target has at
most one Exact mapping; additional aliases require an explicit ReviewedAlias reason. Mapping
an identity to an Unmapped schema preserves that uncertainty rather than granting coverage.

Registry/mapping codecs validate bounded input and canonicalize immutable artifacts. They are
infrastructure for conversion. The conservative [owned normalizer](owned-normalization.md)
now consumes identity/role artifacts across all five originals; full semantic conversion remains open.
The source revision/paths never become native definition IDs or required runtime dependencies.

## Verification and remaining work

Focused project, codec, record/value, selector, mapping and CLI tests exercise these contracts
with directly authored data. The CLI command is `bind-owned-input request.json --schema schema.json`;
it can write a new report with `--output`, and accepts a configurable `--max-work`. A standalone
build cannot silently supply default scenario/query inputs. `check-owned-input` can canonicalize
a project without importing PoB. These commands explicitly report that calculation has not run.

See the [living implementation record](implementation.md) for producing test/CI receipts and
remaining gates: import drafts and all-five normalization; owned effect compilation; shared
player/minion resolution; legacy consumer retirement; joint optimization and wider holdouts.
Component contract tests do not increase complete native-build coverage.
