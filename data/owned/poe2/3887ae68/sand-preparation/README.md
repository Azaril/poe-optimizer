# Sand preparation authoring packet

This packet adds native data-defined preparation for the existing Sand Skill
`0322`. It does not establish a complete build, complete modifier inventory,
support inventory, damage calculation or population-override implementation.

The same rule programs read the existing typed raw inputs (`3261`, `3262`) on
manual and exact tree-generated occurrences. They do not inspect display names,
source-language objects, PoB UI selection or physical Gem records.

## Allocations and flow

The new allocations are `334e` (prepared Skill level, Integer), `334f` (prepared
Skill quality, PercentagePoints), `3350`/`3351` (projected Command level/quality
parameters), and `3352` (supported Skill quality contribution channel). The two
prepared stats and the channel have reusable Skill meanings. Their initial
producer programs belong to Sand, not to a hard-coded runtime dispatch table.

1. Ordinary preparation validates raw level as an integral value in 1–40 before
   adding the existing Player Minion level channel `30ab`. It writes the existing
   pre-support channels `30ac`/`30ad`. It does not apply the unrelated Spell
   channel, manufacture physical-Gem corruption inputs, or recover invalid raw
   values. Quality transports the existing finite signed fractional quantity
   independently of whether level is available.
2. A checked source occurrence performs one support census. Source assembly adds
   supported level `32e1` and quality `3352` to the pre-support values. It requires
   integral final-level arithmetic, then caps the result to 1–40. Intermediate
   level is not capped: the retained raw-40 control goes through ordinary 42
   before the final cap returns 40. Fractional final-level recovery is not copied.
3. The exact Command supply `32a5` receives the prepared level and quality in
   projected-only slots. Command execution readiness requires those exact
   parameters. The downstream numerical Command calculation remains unconverted;
   there are no duplicate derived stats merely to observe projected values.
4. A separate ordinary preparation program reads the shared final level, looks
   it up in the existing injected `sniper.actor-level` table, applies the source's
   final 1–100 population-level bound, and projects `001c` into the exact population
   slot `32a3`. The table name is a retained data ID;
   no Sniper quality or damage formula is reused and the runtime does not infer
   a two-times formula.

The source census draft enumerates summon and Command as effects of one source.
The three actor-child skills are not source owners in this path: source evidence
shows level 1, quality 0, no source occurrence/catalog metadata and the exact
population's actor level. They receive no duplicate property collection here.
Tree `noSupports` is a provider-specific fact; it is not assigned to the shared
Skill definition and does not eliminate actor-provided supported properties.

## Coverage boundaries

`extension.json` uses the existing `OwnedRecipeExtension` contract and preserves
the existing Partial owner closures. `source-properties.json` is an unpublished
draft: all unproved source, effect, channel, external and support inventories
remain Partial. The checked receiving loader intentionally rejects such a draft.
It must not be installed by relabelling those inventories Complete.

The retained witnesses contain no nonzero `SupportedGemProperty` result. The
supported channels are structural authoring, not a nonzero-family parity claim.
The finite component tests explicitly close an empty physical-support universe;
they inject an ordinary Minion-level amount upstream of the actual preparation
rules. A labelled counterfactual adds one synthetic external supported-property
producer per source to verify collection and cap order; it is not real-producer
parity evidence. These tests do not prove real item producer, support-composition
or all-modifier closure.

The table-only population program covers the witnessed override-free slice.
All four source override modes are explicitly absent in the retained original
contexts. Enemy-level, triggered-level, player-level and explicit population-level
overrides remain unconverted and must be accounted for before owner coverage is
completed. Source-level recovery after invalid raw values, nonintegral final
values, or corruption is not authorized by these controls. Finite quality
transport is distinct from legal item/gem quality domains and from quality-effect
mechanics; the supplied negative and above-100 controls are retained as transport
evidence, not evidence that those values are attainable in the game.

The predecessor Sand descendant grant records `32a4` and `32a6` list only
SkillUse providers, although tree-generated descendants retain an Allocation
root. `corrections.json` adds that source-proven provider role and changes no
other field. Publication uses the existing explicit release-migration path for
this correction; the monotonic membership-extension validator remains unchanged.
The finite fixture consumes the same corrected records as publication.

## Evidence and tests

`dependencies.json` preserves the exact relevant records from the current
Boolean package, with separate semantic input identity and raw file SHA-256.
`source-vectors.json` contains 140 distinct compact observations from 18 retained
source controls; each observation matched across fresh/one/two rebuilds. JIT-on
and JIT-off reports were byte-identical. The full report digest, source manifest,
source files, observer, lifecycle witness and original build files are pinned.
No PoB runtime is needed for ordinary native component or authoring tests.

The native tests distinguish the two source occurrences and their exact
descendants, exercise raw versus final domains, signed fractional quality,
independent missing dimensions and an injected table mutation, and compare fresh,
reused and Rayon scratch execution. The source tests authenticate the retained
table, tags, matched ordinary row order, source census and override-free population
records, then compare native arithmetic and exact Command input projections
against retained MAIN/CALCS source values. These tests do not
change the five-build completeness counters.

Publication through the CLI passed in 32.12 seconds at
`runs/owned-sand-preparation-publication-03/validation.json`. It reproduces all
eighteen artifact files byte-for-byte and preserves every canonical draft,
selection, disposition, selected issue and all 110 query identities across
independent all-five imports. It adds no evaluation bundle or coverage promotion.
The new input identity is
`9bbb63fab97fca1b60a67ccb048103e51677a9b0668a0e9d130355b0f930cd24`;
the authoring commitment is
`67e7326a03b13fc56f0499ee081daa31e4ddaff78f96a6432e8b51eb52efdd4c`.

The Engine regression discovered by this packet is fixed: generated preset inputs
have intrinsic Structural readiness and no authored stage. Source-cycle validation
follows their real provider ancestors without erasing dependency edges or allowing
late ordinary producers. The implementation retains one existing graph and does
not manufacture a stage merely to make this packet compile.
