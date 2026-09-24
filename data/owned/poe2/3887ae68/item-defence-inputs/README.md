# Ordinary defensive item inputs

This component defines 19 owned modifier families and 38 fixed-number grammars. Each
recognized physical line creates one modifier occurrence with its own numeric component.
Compound families retain their distinct meanings, including the three-effect Armour,
Evasion and Energy Shield family. Shared output statistics do not merge occurrences.

| Effect family | Flat BASE | Increased/reduced INC | Numeric formatting |
| --- | --- | --- | --- |
| Armour | Yes | Yes | Normal |
| Evasion | Yes | Yes | Normal |
| Energy Shield | Yes | Yes | Normal |
| Ward | Yes | Yes | Normal |
| Armour and Evasion | Yes | Yes | Legacy for BASE; normal for INC |
| Armour and Energy Shield | Yes | Yes | Legacy for BASE; normal for INC |
| Evasion and Energy Shield | Yes | Yes | Normal |
| Armour, Evasion and Energy Shield | Yes | Yes | Legacy for BASE; normal for INC |
| Defences | No | Yes | Legacy |
| Block Chance | Yes | Yes | Normal |

The exact spellings and source witnesses belong to the import policy and `bindings.json`.
The native evaluator receives owned definitions, ordinary typed rule programs and explicit
inputs. In particular, increased **maximum** Energy Shield has a distinct source Global tag;
it is not an alias for the ordinary increased Energy Shield family defined here.

The first eight BASE families use existing Count unit `295a` and Modifier statistic `295b`.
These are generic numeric magnitudes at an individual modifier occurrence; using them does
not assert an Actor attribute contribution, defence rating or resource-point quantity.
Later consumers must make explicit typed projections at each occurrence. Hybrid amounts
must not be implicitly added across Rating and Energy Shield units or aggregated before
ordering is established. Flat Block Chance and all ten INC families use percentage-point
unit `0002` and Modifier statistic `253e`. Keys use prefix `def.000000000000` in
`poe2/owned-mechanics-v1`.

All families retain the shared property predicates, scalability state, catalyst calculation,
ordered magnitude channel and corrupted-base factor. Properties come from explicit input
facts; English effect names do not imply tags. The source defence catalyst group includes
`defences`, `armour`, `evasion` and `energyshield`; Ward does not imply an extra category.
Missing or incomplete inputs remain unresolved. No new contribution, receiver, local/global
application rule or complete owner closure is introduced.

Fifteen families use the existing generic normal numeric-component compiler with internal
precision 1 and final decimal precision 0. Flat signed amounts retain their sign through
its rounding stages. INC families instead retain a nonnegative magnitude and an explicit
qualifier direction, applied after formatting. Existing scalar producer programs are
rebound to the exact new modifier owner and its declared slots.

The remaining four families use ordinary rule DAGs for fixed-number legacy arithmetic:
Armour-and-Evasion BASE, Armour-and-Energy-Shield BASE, triple-defence BASE and Defences INC.
Their formatting depends on two required Boolean inputs:

- `decimal_quantization` records whether the validated numeric capture contains a decimal
  point. Item-line policy v6 projects this lexical fact into an ordinary Boolean; the
  evaluator never reads source text. Equal quantities such as `1` and `1.0` can therefore
  retain different formatting behavior.
- `base_rounding_present` distinguishes an absent optional base scalar from an explicitly
  supplied scalar of 1. The currently admitted untagged import path writes false. A later
  adapter must prove presence independently when admitting such source controls.

These recipes preserve the identity-factor fast path. When scaling is required, an
explicitly present base scalar triggers base rounding before magnitude scaling. Integer
mode uses the source `floor(value + 0.001)` magnitude step; decimal mode multiplies by 10,
floors, then divides by 10 without that bias. Multiplications remain separate operations.
Textual sign or qualifier direction is projected after unsigned numeric formatting. For
example, decimal magnitude 0.15 scaled by 1.5 yields 0.2 with the base scalar absent, but
0.3 with an explicitly present base scalar of 1. The formatter metadata leaves the normal
compiler's precision fields null for these four families.

These are numeric components **before source string transport and reparsing**. The rules do
not establish that every formatted result remains an admitted source modifier. Decimal,
negative, range, tagged, variant, crafted, rune and predecessor-ambiguous source cases keep
their existing unresolved gates. Fixed-number formatting does not stand in for range
interpolation or mutable source editing history.

Source comparisons of the fallback numeric recipe cover ordinary fixed numbers with a
digit before the decimal point. The wider raw Decimal codec also recognizes `.15`, but
PoB's legacy scanner formats that spelling differently; raw recognition does not prove
formatting eligibility. Decimal INC results can also fail the later source parser. Source
item magnitude passes call the formatter again with an explicit base factor of 1 and can
skip reformatting when the cumulative factor returns to 1: reversing +25/-25 controls
retains INC values 13 versus 8 from an initial 11. A negative INC spelling can instead
install an incomplete BASE parse which assembly excludes. Nil-result retention remains
an audited source branch, not the executed witness. Those ordered lifecycle effects are
separate from a single numeric-component formula and remain unresolved.
The guarded source-member rules cover plus-prefixed BASE integers and unsigned integer
increased/reduced spellings within 0..1,000,000. They require no source tags and no generated
base members. Negative and decimal captures can still have raw numeric recognition without
receiving this source-member authority. There are no new ranged grammars. Structural
membership is authored for all 1,756 known base templates, including jewellery; it does not
prove affix legality or decide whether an occurrence is consumed locally by equipment or
contributes globally to an actor.

`extension.json` supplies definitions and programs; `membership-patch.json` binds their
structural membership to the exact raw-defence-profile predecessor. The paired `items.json`
and `item-source.json` carry raw decoding and guarded source attribution. `bindings.json`
records the owned IDs, exact family meanings and source evidence outside the runtime
calculation contract. Publication uses the existing checked `extend-owned-recipe` path with
these paired artifacts and a new output directory. The authoring specification adds 19
owners and 455 required slots, with 76 programs; it adds no units or statistic definitions.

Raw equipment profiles, canonical modifier magnitudes and source-member evidence are
separate inputs to later assembly. Active equipment selection, complete ordered local
contributor sets, quality behavior, per-level effects, overrides, final item defences and
actor mitigation still require their own recipes and validation. This component makes no
complete equipment or original-build parity claim. See
[raw defensive profiles](../defence-profiles/README.md) and the proposed
[ordered contribution boundary](../../../../../docs/owned-contribution-stages.md).
