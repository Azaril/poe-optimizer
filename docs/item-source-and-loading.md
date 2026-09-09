# Item source and ordered loading

The item input boundary preserves arbitrary caller inventory and saved selections before
resolving item definitions or evaluating modifiers. This supports the shared native build
pipeline described in the [general model proposal](general-build-input-proposal.md); its
actor/action migration remains a separate design discussion. Current implementation and
validation status live in the [implementation log](implementation.md).

## Source, loading and calculation

Four distinct stages must retain their own evidence:

1. Exact authored XML, byte ranges, occurrence order and the input hash.
2. The strings and element records consumed by the pinned PoB XML reader, including text
   fragmentation, comment removal, CDATA handling and named-entity decoding.
3. Ordered item loading: each consumed item string invokes ParseRaw, while ModRange records
   modify the current parsed line lists in between those invocations. Saved equipment sets
   and passive-spec jewel references retain separate owners.
4. Item definition resolution, full parsing and modifier assembly, equipment selection,
   item-derived grants, allocation providers and numerical effects.

Source projection prepares the first two stages and identifies the third stage's
instructions. It does not execute item parsing or certify a resulting item. Game bases,
modifier templates, ranges, caps, slot rules and effect definitions belong in injected
versioned data. A fixture name, source hash, character identity or specific supplied item
must never select special production behavior.

## Ordered text and ranges

The pinned XML reader removes comments before constructing its table. Removing a comment
can join ordinary text on either side, while CDATA supplies a separate text record.
Ordinary-text trimming and entity decoding must follow that reader's order rather than a
standard XML library's normalized text. Authored fragments and consumed strings are
separate representations, tied together by source ranges and occurrence references.

Original ItemsTab.Load iterates Item children in order. It calls ParseRaw for each string,
then applies each ModRange to the current buff/enchant/implicit/explicit line lists. A
later ParseRaw call can replace the state to which an earlier range applied. Joining all
text at the start or deferring all range instructions to the end can change the item.
The source saver and loader are not symmetrical here; normalized exports must remain
separate from exact-source inputs.

Raw unknown records, duplicates, empty or malformed values, namespace lookalikes and
inactive sets remain visible. Projection does not replace them with defaults, silently
select the last duplicate or promote an unknown tag to a known item instruction. A bound
on the item projection yields a local error; other build sections remain available
independently. The common root lexical gate still rejects globally incompatible XML,
including prefixed namespace attributes. Default-namespace lookalikes that pass that gate
remain visible as unclassified source content. Source inspection and numerical admission
have separate gates; accepting mixed item text does not admit it to native calculation.

## Inventory and ownership

An inventory Item occurrence, an authored item ID and a slot's reference to that ID are
different identities. The same inventory entry may occur in multiple saved sets. Preserve
set order and requested active selectors separately from selected equipment; literal
`useSecondWeaponSet="nil"`, false and an absent attribute remain different authored values.

Items/ItemSet/Slot records assign equipment. RuneSlot records describe character rune slots.
SocketIdURL records supply URL metadata, while actual jewel assignments belong to
Tree/Spec/Sockets/Socket records, or legacy direct-root Spec/Sockets/Socket records. A passive spec and equipment set do not share a selector
merely because their IDs coincide. Legacy direct Slot rows and independent saved alternatives
must remain available for the later source-load resolver.

Item-derived allocation providers affect graph legality. The observed From Nothing case in
the [breadth inventory](breadth-mechanism-inventory.md) shows why preserving the item's
provenance and the passive socket reference is required before applying ordinary path
checks. Separate ascendancy ownership and point budgets still apply. Loading or calculating
an item does not establish character progression, point entitlement or acquisition legality.

## Ordered loading and injected definitions

Native loading must interpret the projected instruction stream as an ordered state machine.
An Item constructor performs an empty ParseRaw before authored text is loaded. A subsequent
ParseRaw resets some fields, including base, quality, line lists and requirements, while
other state remains until explicitly overwritten. Repeated text cannot be modeled as a
fresh independent item each time. Range instructions act on the state present at that
point, with the original list order and rune-line exclusion; an out-of-range instruction
must retain its source evidence even when the original loader makes no change.

Modifier-parser results feed back into loading. Original
[Item.lua](../vendor/path-of-building-poe2/src/Classes/Item.lua) `:1219–1231` attempts a
combined two-line modifier when parsing leaves a remainder; success changes which line is
consumed next. `:1303–1353` uses parse outcomes to assign line categories. The loader therefore
needs explicit parser outcomes, consumed spans and unparsed remainders as inputs to its
state transitions. A partial metadata report must identify that boundary when the native
modifier parser or a required definition is unavailable.

Assembly also mutates parsed state: the original method updates requirement records and
modifier payloads, including item source labels, before later range/text operations run.
The provider result must carry those typed changes back into the loader. Replaying an
assembly's top-level scalar values alone does not preserve subsequent loading behavior.

Definitions must be injected independently of caller item instances: the complete base
catalog and aliases, affix/unique metadata, rune and bonded effects, jewel radii, header
formatting and named compatibility policies. Normal/magic base-name fallback searches the
complete catalog. Native mechanic capability does not determine identity lookup candidates.
Ambiguous or unresolved names require retained evidence, rather than a fixture-selected
fallback. Preserve hidden entries and unknown fields without assuming their effects work.
Requirements combine rune, unique, imported and base data in the source's order
(`Item.lua:1699–1726`). Catalog recognition alone does not resolve that pipeline.

Validation must include repeated ParseRaw resets and retained state, failed and successful
two-line parsing, partial remainders, range indices at category boundaries, rune exclusion,
variant/version/group selection, catalysts, advanced copied items and conflicting header
or requirement inputs. Corpus examples and independent synthetic source cases complement
one another; absence from the current corpus must not become an unsupported behavior's
default value.

## Caller-facing evidence

`inspect-build INPUT` is the caller-driven entry point. Report schema 3 retains independent
configuration, skill and item source projections. Source-only inspection needs neither a
data package nor a PoB runtime, and records item loading as `not_run`.

`--with-definitions` or `--data PACKAGE` also runs the native ordered item loader against the
selected catalog. `definition_lookup.items` contains the report or its independent error.
The report binds the exact XML, data identity and loader implementation fingerprint. It
preserves every inventory occurrence and every consumed instruction, including constructor
and final assembly. Item IDs remain authored strings; they are not occurrence identities.

This inspection path retains source text and detailed call evidence for diagnosis. It is an
import/preparation component, not the optimizer's candidate hot loop; its throughput does
not establish full evaluator performance. The eventual prepared item representation should
be immutable and reusable across candidates, with detailed tracing enabled separately from
numerical execution.

The built-in inspection provider supplies [native item formatting](item-formatting.md)
[ordinary structural modifier parsing](modifier-parser.md) and
[completed unique requirements](unique-requirements.md) using the selected data.
Selected callback execution, stateful parsing and complete assembly remain explicit
dependencies. A formatting fallback can itself require a parser call to discover precision;
that request and its outcome remain distinct from the loader's subsequent modifier parse.
The provider stops at the first required unavailable operation, retains the state established before
that operation and marks later instructions `not_executed`. The empty constructor follows
the source's no-base no-op behavior. Inspection never invokes PoB to fill a missing native
operation. Explicit providers support comparison of loading transitions against original
source callbacks in tests; supplying a parser result is not a native parser implementation.
The report's data and loader fingerprints do not authenticate a caller-supplied provider's
implementation or results. A host using a custom provider must retain that provider's
provenance independently. The built-in provider, engine formatter and structural parser are included in
the loader implementation fingerprint; injected formatting and unique-requirement definitions
are bound separately by the selected data identity.

A library host retaining a standalone `ItemLoadingReport` should also retain
`poe_optimizer_data::implementation_fingerprint()` for the catalog implementation.
`inspect-build` already emits that value as `definition_implementation_sha256` beside the
loading implementation identity, and the corpus runner preserves both. The standalone
report's data identity binds package content; its loader fingerprint alone does not bind
all code in the data crate.

Defence display headers now populate first-class optional armour data before the source's
later `hidden_specs` branch. Their keys and base rewrites come from the selected data
package. The [state contract below](#defence-display-state) distinguishes these copied
values from calculated defensive ratings.

An item status of `pending` identifies the required dependency; `source_error` records an
attempted source operation that failed. `no_base` means loading ended without a recognized
base. `complete` describes completion of the supplied loading operations only. None of these
statuses certifies equipment selection, mechanic coverage, gameplay legality or complete
build evaluation. Top-level verification says `reported` when a loading report exists,
including a partial one; numerical and legality gates remain separate.

The corpus runner's schema 5 records this loading evidence alongside independent numerical
backend results. It validates complete occurrence ownership, source instruction order and
text hashes, the executed prefix and stopped suffix, selected data identity and loader
fingerprint. It retains raw metadata without treating it as numerical capability. Earlier
schema-1 and schema-2 build reports remain readable with loading evidence explicitly
unavailable. Missing fields do not mean that a build has no items. Original XML and raw
reports are retained when projection, loading or evaluation fails; changed input cannot be
evaluated under the original build's identity.

## Socketed-augment state

Socketed augments use the selected `item_loading.rune_loading` policy and a borrowed
view over the complete rune family. The native loader selects a constructed base's broad
and specific slot contexts, then any extra SoulCore contexts. Classification precedence,
subtype rewrites, header grammar, empty-slot identity, numeric patterns, display markers
and vector rules come from data. Record membership does not establish socket legality;
loading must preserve original behavior even for an unusable authored combination.

Authored rune names retain order, including empty slots, unknown names and entries beyond
the active socket count. All names participate in the source's rebuild eligibility check;
only active positions contribute rebuilt modifiers. Requirement calculation subsequently
examines every slot record of every authored name, including inactive positions. An
unknown name prevents that rebuild but does not skip preceding raw-line loading or the
subsequent annotation operations.

Enabled Bonded display lines skip the raw modifier parser and contribute to implicit-line
routing counts. Disabled Bonded rows follow the ordinary formatting/parsing path. Context
hints are captured per row, applied only for the selected variant and reset on each reparse.
An item override has last-write semantics; extra SoulCore types form a distinct set.

Rebuilding restores the natural level, clears rune rows and visits socket contributions
in source order. Broad, specific and extra slot contributions are deliberately not
deduplicated. Stacking keys contain augment type, Bonded status and Lua-rendered stat order;
they do not contain the line template. Numeric callbacks reproduce Lua text round trips,
including the source pattern's unusual treatment of multi-digit decimals. A new row is
parsed before insertion. A combined row assigns its rewritten text before parsing and
only then replaces its modifier payload. Failed dependencies retain that exact prefix of
state. Disabled flags are restored by normalized-template counts after a successful rebuild.

`ParseRequest.line_index` and `LoadedModLine.source_line` are nullable: generated rune
contributions have no authored line. Each generated request carries a `RuneContribution`
with socket index, selected slot, definition line index, Bonded status and whether the text
was combined. Rows retain all contributions to their current text. The request's separate
`combined` flag preserves the parser argument; it stays false for rune reconstruction.
Optional order, augment type, rune count and effect/display annotations distinguish absence
from false and zero. Ordinary authored reports keep their existing numeric line indices.

Contextual grouping and minimum-count searches also run for explicitly named runes. A
unique count vector can annotate normal and Bonded rows without choosing an arbitrary Lua
traversal order. Ties that affect inferred identities or final types remain explicit
`RuneReconstruction` dependencies, including equal vectors with implicit trailing zeroes.
Multi-slot floating sums and multiple extra-slot contribution orders require proof before
execution. Resource exhaustion is an error, never a no-solution result. Pattern work,
callback text, temporary grouping rows, sort comparisons and search expansion have bounds.

Unscaled inference can identify omitted names and rebuild their lines. GAME inference that
requires ranged augment-effect modifiers remains a separate dependency, as do nonzero
configured effect defaults, unrepresented metadata operations, advanced-copy unique stat
ordering and complete item assembly. These boundaries must be implemented with their own
source parity before admitting dependent builds. They must not be replaced with fixture
identities or an implicit reference-backend fallback.

The numeric primitives are portable Rust. Catalogs are immutable and shareable; per-item
loading scratch is local. Diagnostic grouping is preparation work, not the numerical
candidate hot path. A future prepared-item layer should cache proven contextual programs
and keep item parsing, data loading and hashing outside parallel evaluation loops. Original
PoB methods remain optional development oracles for the same state and call boundaries.

## Authored affix state

`ItemState.prefixes` and `suffixes` each contain ordered entries and an optional limit.
An entry retains its `mod_id`, optional scalar or independent range, and optional fractured
marker. A present empty range array, numeric zero, signed zero, NaN and infinity remain
separate from absence. Invalid numeric elements are omitted according to Lua insertion
semantics; an invalid scalar uses the injected default except for the injected empty-slot
identity. Both lists, including limits, reset on every `ParseRaw` call.

Header names, Lua patterns, default quality, modifier-limit patterns, rarity roles, caps,
empty-slot identity and legacy-label field come from the selected `item_loading` policy.
The native pattern engine compiles the grammar with bounded storage and execution work.
Source-derived reserved headers prevent an injected affix name from taking over another
loading operation. Postparse effects retain source branch order: preceding exact effects,
first matching prefix/suffix limit branch, then later exact effects. Disabled modifier lines
supply the empty string to those branches.

Reconciliation uses the retained selected modifier family, even after a later parse has
lost its base. A missing family clears the crafted flag. Supported rarities determine the
active side counts using the injected rules; entries beyond those counts remain intact.
Empty active positions are filled, exact truthy IDs survive, unique legacy labels resolve,
and definite misses become the empty-slot ID. Rewriting an ID preserves its range and
fractured marker. Ambiguous legacy labels and unrepresented traversal behavior stop with
an explicit dependency before changing the unresolved entry. Lookup does not choose a
sorted winner for Lua `pairs` collisions.

This is imported loading state. Original ordinary `ItemsTab.Load` does not call `Craft`,
and native loading does not generate crafted modifier lines from these records. Complete
item assembly and equipment legality remain separate dependencies. Lookup indices are
immutable data-catalog preparation; native lookup requires neither Lua nor subprocesses.
Per-item diagnostic pattern scratch and evidence are bounded independently of numerical
candidate evaluation.

## Defence display state

`ItemState.armour_data` distinguishes an absent table, an empty table and retained numeric
entries. `ParseRaw` does not reset this table. A recognized header creates it even if numeric
conversion returns nil; nil removes that key, while other keys and exact numeric values,
including signed zero and explicit non-finite values, survive. Repeated headers and later
text/range instructions observe the resulting state. No absent value receives a default.

The injected rewrite operation compares the retained base name even when the current base
reference is absent. It changes only that name and reference, before number conversion.
It does not rerun base setup or reset the previous type, requirements or modifier state.
A missing target leaves an absent reference with the rewritten name. The original pinned
catalog's missing references remain visible; synthetic source tests inject bases separately
to exercise successful rebinding without relabeling those cases as authentic catalog data.

Recognizing a header does not bypass the remaining line-processing state machine. A header
inside an explicit or implicit section can still cause a modifier-parser request. Assembly
can subsequently replace or remove armour data before another text record is loaded.
`AssemblyOutcome.armour_data` therefore uses `ArmourDataUpdate::{Preserve, Clear, Replace}`;
`Replace` can supply an empty table. This provider boundary transfers observed state, not
an implementation of the original item assembly calculations.

State tables are bounded to 256 entries. Header keys and rewrite names are validated with
the injected catalog, and provider replacement keys and numeric representations are checked
before applying assembly updates. These diagnostics extend the existing loading state
report without changing its numerical admission contract. Native complete item assembly,
including how display state interacts with computed ratings, remains a required later phase.

## Base buff generation

Selected item bases can supply independent `flask.buff` and `charm.buff` definitions in the
injected catalog. Loading handles flask first, then charm, once per family within each
`ParseRaw` call. An initialized empty table still prevents regeneration when a later base
is selected. Buff rows and both suppression sets reset on a subsequent parse. Base variant
selection and ordinary base setup happen before these parser calls.

Every consecutive definition entry invokes the parser directly with its exact text,
including duplicate or empty strings. There is no formatting, annotation stripping,
catalyst scaling or additional combined-line retry. A generated row keeps the parser's
exact remainder; nil modifiers become an empty modifier list. Range, scalar and selection
metadata start absent. Requests and rows retain the triggering authored base line as their
source context, while the selected data identity binds the generated text.

Each family also records a set of texts to suppress once from the later authored input.
Suppression runs before separators, literal flags and headers. Repeated definition entries
produce repeated rows and requests but one suppression key. When both families contain the
same text, the first authored match consumes the flask entry and the next consumes the
charm entry. Sparse indexed tables follow source `ipairs`: stop at the first missing integer
key, without compacting holes or treating string keys as numeric keys.

Parser unavailability, errors and resource bounds retain the completed prefix; the failing
row is not appended. The string-only provider does not invent a request for a malformed
non-string definition entry. Source tests distinguish attempted calls from completed string
requests and separately prove the original type-error boundary. Generated rows, suppression
keys, retained text and parser results share the bounded loading-evidence budget; generated
row count has an explicit limit independent of authored line count.

XML ModRange visits generated buff rows before enchant, implicit and explicit rows. A later
parse rebuilds buff rows and their absent ranges; assembly-provided modifier payload updates
remain visible until then. Loading these records does not implement flask/charm charges,
duration, activation, effect calculations or complete item assembly. No package migration
is needed: these definitions were already included in item-loading schema 2.

## Validation requirements

Independent tests must execute the original XML reader and source load methods, retain
both authored fragments and consumed record order, and distinguish any instrumented
ParseRaw/GUI boundaries from real modifier calculations. Compare all saved corpus sets
and passive specs, plus synthetic mixed-content and duplicate cases. Tests for source
loading do not substitute for the later complete item/numerical parity suite.

Preserve the five complete caller builds, original numerical goldens, injected package
and pinned source. Native item admission expands only with a complete supported pipeline
and fresh full-build differential evidence. A recognized base name or matching modifier
string is not enough.
