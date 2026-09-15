# Resistance contribution component

This append-only recipe preserves the 2,515 existing registry entries, schema
declarations, and program bodies from `../import/compiled/recipe.json`. It adds
nine explicitly allocated identities, two item contribution programs and four
reviewed reward contribution programs. It does not produce a final resistance
stat or metric. Original-build completion remains **0/5**.

| Owned key suffix | Definition |
| --- | --- |
| `09d4` | Cold resistance BASE contributions, Actor-targeted percentage points |
| `09d5` | All-elemental resistance BASE contributions, Actor-targeted percentage points |
| `09d6`, `09d7` | Cold modifier and its required roll parameter |
| `09d8`, `09d9` | All-elemental modifier and its required roll parameter |
| `09da`, `09db` | Life modifier input and roll; numerical effect remains Partial |
| `09dc` | Sapphire Ring template; modifier, socket, quality and game-rule coverage remain Partial |

All keys have the `def.000000000000` prefix and the namespace recorded in
`ids.json`. The percentage-points unit reuses existing key `0002`. The roll
envelope is a bounded computation domain, not a claim about possible game rolls.
The range-policy extension preserves all 2,524 identities and the existing
recipe/schema bytes.

`items.json` contains general lexical integer captures for plain cold,
all-elemental and Life lines, plus integer endpoint ranges for cold and
all-elemental resistance. `item-source.json` binds those declarations to the
exact schema and source layout. Metadata headers do not become ItemLevel or
Quality facts. Unsupported lines, tag semantics, ambiguous preceding lines,
missing template membership and unproved range lifecycle information remain
unresolved. Matching a line does not prove whole-item conversion or activation.

The reviewed fixed outcomes contribute +10 cold, +5 cold, +5 all-elemental, and
−5 all-elemental from existing Reward identities. The mixed negative outcome
retains Partial program coverage for its other six effects. Programs target the
selected player's contribution set, without copying a cached build total.

## Lexical and arithmetic boundary

`NumericCapture` consumes a maximal ASCII integer token before semantic numeric
conversion. Fixed numbers allow an optional plus or minus. Inner range endpoints
allow an optional minus only, matching the pinned source grammar. This separates
fixed and ranged shapes without choosing whichever semantic decoder succeeds.
The package declares plus-prefixed and bare ranges as separate, disjoint rules.
Outer-minus range inversion, decimal endpoints and other source grammars remain
unsupported; no per-item rule or observed-number pattern is substituted.

The range policy explicitly uses `InterpolateOffset` for `a + f * (b - a)` and
`SymmetricHalfOffset` for the literal source operations: nonnegative values use
`floor(x + 0.5)` and negative values use `ceil(x - 0.5)`. This preserves source
floating-point ordering rather than silently substituting stable convex
interpolation or mathematical nearest rounding. Both operations reject
nonfinite intermediates/results. For example, −3..2 at fraction 0.7 follows the
source result 0.5→1; stable convex interpolation produces a value below 0.5.

Original05 item26 is one Sapphire Ring record referenced by eight receiving
rows across four item sets. Its two raw source members are the cold implicit and
`+10 to maximum Life`. The exact original remains **Pending**: its
`{tags:cold_resistance,elemental_resistance,elemental,cold,resistance}` metadata
feeds catalyst and modifier-magnitude semantics that are not yet modeled here.
`UnsupportedTag` prevents promotion of its cold modifier. Production does not
strip or ignore that metadata.

The integration target separately uses clearly labelled in-memory diagnostic
copies that remove exactly this one reviewed literal tag and vary the two XML
range writes to 0, 0.5 and 1, yielding cold rolls 20, 25 and 30. These copies prove the generic
range/source-attribution component, not native success for the original. An
inserted unknown member must still make the affected range pending. All eight
receiving rows remain distinct; none is selected or multiplied into a build
total here. Original02 item26 is a Grand Spear and is kept distinct. Protected
original files are never rewritten. Source modTags and their scaling effects
are the next blocking semantic-input obligation for the original ring. The proposed
[item-scaling slice](../../../../../docs/owned-item-scaling.md) defines the next input,
owning-item and ordered-transform seams; it is not implemented tag/catalyst or rune support.

The Life input declaration preserves that second source member; it does not
supply a Life effect. Source layout completeness is separate from the Partial
item, modifier, quality and rule memberships in the owned package.

## Remaining receiver

`source-facts.json` records the exact cold/all-elemental BASE combination,
increase/more multiplier, override, truncation and clamp source. There is no
appropriate common receiver owner in the current package. This slice does not
invent one through a UsagePolicy, Encounter or per-class duplicate program.
Contributor completeness, receiver ownership, limits, overrides and selected
actor identity must be resolved before a final Stat and Metric can be declared.

## Reproduction

Run from the repository root with Python; no Lua or source VM is executed:

```text
python scripts/export-owned-resistance.py --base-dir data/owned/poe2/3887ae68/import/compiled --import-inputs data/owned/poe2/3887ae68/import --authoring data/owned/poe2/3887ae68/resistance/authoring.json --source-root vendor/path-of-building-poe2 --check-dir data/owned/poe2/3887ae68/resistance
python -m unittest scripts/tests/test_export_owned_resistance.py -v
cargo test -p poe-optimizer-import --test owned_resistance_recipe
```

To regenerate, replace `--check-dir` with `--output-dir NEW_DIRECTORY`. Output
creation refuses an existing directory or file; an I/O failure can leave an
incomplete new directory. This exporter is not an atomic runtime publisher.
The production `assemble-owned-recipe` command independently validates and
publishes the recipe. The Rust integration target loads the persisted recipe
and both policies through production constructors before exercising them.
Existing import mapping/role/policy artifacts are not silently rebound.

Source file and span SHA-256 hashes use LF-normalized source bytes. The recipe
digest covers exact artifact bytes. Source pins and source text are offline
provenance, and are not read by the rule executor.
