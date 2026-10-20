# Four item templates: intrinsic structural inventories

This offline packet closes exactly twelve empty declaration inventories:
`grants`, `actors` and `skill_grants` on Ashen Staff (`1d75`), Fine Belt
(`1e84`), Rope Cuffs (`2007`) and Tattered Robe (`238c`). It allocates no
definitions or programs and changes no numerical owner, modifier membership,
parameters, choices, outputs, sockets, quality, query or source observation.

The predecessor is the current Scenario configuration format package with input
`091556fee21e868c21ac3c2a61390c3e90c5900ab4576552c90a46a9fcc63ffd`.
`migration.json` is an ordinary current-format release migration: format 5,
schema 7 and operations V28. Each replacement restores exactly to its full
predecessor descriptor when its three closure fields are restored.
`dependencies.json` also retains the six relevant modifier/Skill definitions,
126 supplying/parameter slots and nine numerical owners. Joint publication
performs the full package inverse and all-five import checks.

Ashen Staff's Firebolt remains a modifier-owned relation:
`Modifier31c6 → Grant31c8 → SkillGrant31c9 → Skill0134`. The granting slot accepts
`item_modifier`; the supplying definition, occurrence identity and variable
level remain unchanged. Level 11 in the retained original is a source
observation, not a native constant. Neither this chain nor its incomplete output
coverage is copied onto the ItemTemplate.

## Source authority

`source-vectors.json` contains the complete four literal base rows, exhaustive
field classifications, exact source-to-template mapping entries, physical
modifier and equipment occurrence joins, and seven reviewed construction
intervals. Ten complete upstream source files are retained as gzip archives and
verified against the independently pinned source manifest. The review includes
the entire `ItemClass:BuildModList` body (2694–2863), its slot-list construction
callee, the modifier parser and item-to-generated-skill receiving boundaries.
The complete upstream `LICENSE.md`, including its MIT notice, is retained in
`evidence/` and authenticated by its byte count and SHA-256.
Unknown base fields, callable records, changed intervals or unreviewed source
routes require a new source review; this checker does not interpret Lua.

The structural distinction follows the full writer boundary. Base metadata
supplies classification, numeric properties, requirements and capacities.
Implicit, explicit, enchantment, rune and class modifier routes assemble the
modifier list. Its `ExtraSkill` records produce generated skills; their
occurrences and descendants retain the supplying modifier owner.

Three complete historical construction reports are compressed in `evidence/`.
Their JIT-off/on source commitments and exact original projections are retained.
The runtime witness stores a bounded 504-row projection of all four item
identities across both original/repeat cases and every captured fresh/rebuild
environment. Its two complete historical reports have identical hashes.
Ordinary tests authenticate the retained projection and its source metadata;
they do not reread the 100 MB acquisition or execute source code. Missing grant
fields in construction witnesses remain explicitly missing; the independent
runtime witness records the observed grants.

These observations corroborate the reviewed writer boundary. They do not close
arbitrary modifier domains, dormant presets or complete gameplay legality.
Original04's unresolved physical Ashen Staff parameters and modifier membership
remain Pending. All thirteen Original05 equipment occurrences are preserved.

## Checks and publication interface

`tests/owned_item_structural_inventories.rs` exercises the positive packet and
mutation controls for every closure, unrelated fields, Firebolt ownership,
missing dependencies, source boundaries, exact occurrence joins, unknown grants
and preserved incomplete coverage. It needs checked-in evidence only, without
`runs/`, the PoB checkout or Lua.

The helper exposes `checked_migration()` and `prior_descriptors()` for joint
publication through the existing checked release migration path. Both authenticate
the packet before returning their typed values. The root publication helper owns
whole-release inversion, dependency rebinding and five-original validation.
