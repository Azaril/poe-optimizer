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

`inspect-build INPUT` is the caller-driven entry point. Report schema 2 adds an independent
item projection to the existing configuration and skill projections. It labels item loading
as not run, equipment resolution as unresolved and passive allocation as unchecked.
Source-only inspection needs neither a data package nor a PoB runtime. Optional configuration
and skill identity lookup remains separate from item interpretation.

The corpus runner's schema 4 records the item source evidence alongside independent
numerical backend results. Older schema-1 build reports remain readable, with item evidence
explicitly unavailable; missing fields do not mean that a build has no items. The original
XML and raw report are retained when a projection or evaluation fails. A changed input
cannot be evaluated under the original build's identity.

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
