# Resistance contribution component

This append-only recipe preserves the 2,515 existing registry entries, schema
declarations, and program bodies from `../import/compiled/recipe.json`. It adds
nine initial identities, two fixed item contribution programs and four reviewed reward
contribution programs. The property extension appends fourteen further identities for
nominal cold/all-elemental amounts and five Boolean properties per modifier. A further
sixteen identities represent catalyst selection and its enabled amount. It does not produce a final resistance
stat or metric. Original-build completion remains **0/5**.

| Owned key suffix | Definition |
| --- | --- |
| `09d4` | Cold resistance BASE contributions, Actor-targeted percentage points |
| `09d5` | All-elemental resistance BASE contributions, Actor-targeted percentage points |
| `09d6`, `09d7` | Cold modifier and its required roll parameter |
| `09d8`, `09d9` | All-elemental modifier and its required roll parameter |
| `09da`, `09db` | Life modifier input and roll; numerical effect remains Partial |
| `09dc` | Sapphire Ring template; modifier, socket, quality and game-rule coverage remain Partial |
| `09dd`, `09de`, `09df` through `09e3` | Nominal cold modifier, amount, and five Boolean properties |
| `09e4`, `09e5`, `09e6` through `09ea` | Nominal all-elemental modifier, amount, and five Boolean properties |
| `09eb` through `09f8` | None plus13 canonical catalyst Options |
| `09f9`, `09fa` | Exact Sapphire template catalyst selection and enabled amount parameters |

All keys have the `def.000000000000` prefix and the namespace recorded in
`ids.json`. The percentage-points unit reuses existing key `0002`. The roll
envelope is a bounded computation domain, not a claim about possible game rolls.
The property extension preserves the first 2,524 registry entries, all existing slots
and program bodies. Its only change to an existing descriptor appends the two nominal
families to the Sapphire Ring's partial modifier membership. The catalyst extension preserves
those2,538 entries and all old slots/rule bodies, adding two required Sapphire inputs. The
current watermark is2,554; nominal rule owners remain Partial with no effective contribution
program.

`items.json` contains general lexical integer captures for plain cold,
all-elemental and Life lines, plus integer endpoint ranges for cold and
all-elemental resistance. `item-source.json` binds those declarations to the
exact schema and source layout. Metadata headers do not become ItemLevel or
Quality facts. Item lines use wire/digest version2 and source policy version3. Five exact property labels map
to typed Boolean modifier inputs; unknown or unconsumed labels, ambiguous preceding
lines, missing template membership and unproved range lifecycle information remain
unresolved. Matching a line does not prove whole-item conversion or activation.

Canonical `Catalyst: NAME` headers convert all13 exact source names to owned Options;
`CatalystQuality: VALUE` converts finite decimal input in percentage-point units. Full
currency names and descriptor aliases are separate source syntax and remain unresolved.
Selection None and enabled amount20 are explicit per-template missing-input data. Amount
is used only when a catalyst is enabled; it does not assert an authored header or active
scaling. Explicit0 remains0.

The source policy proves missing fields only after complete canonical layout admission.
Each parameter fallback names the source headers whose presence suppresses it. Raw metadata
headers also block absence, and authored/pending assignments suppress fallback for their
exact slot. Unknown, malformed and duplicate relevant headers do not activate defaults.
The Sapphire base alone has reviewed item-level and ordinary-quality absence policies;
Grand Spear has different quality behavior. Defaults retain separate sidecar provenance.

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
rows across four item sets. Its two raw members are the cold implicit and
`+10 to maximum Life`. The original text now converts without removing its
`{tags:cold_resistance,elemental_resistance,elemental,cold,resistance}` annotation:
the nominal cold amount is25 and all five explicit property parameters are true.
The source label tokens/spans remain in attribution evidence. Each rule receives
only its declared properties. Recognized labels cannot disappear into an older
fixed-value rule that has no property outputs.

Range perturbation tests preserve that annotation and vary XML fractions0,0.5,1,
yielding nominal amounts20,25,30. Unknown added labels or members remain unresolved;
all eight saved receiving rows remain distinct. The older tag-stripped diagnostic
helper has been replaced by these tag-preserving cases. Original02 item26 is a
separate Grand Spear; rune/Bonded and ordinary weapon inputs remain separate work.
Protected original files are unchanged.

The new amounts are deliberately nominal. Existing fixed-value inputs are not
reinterpreted as nominal, and the new families do not emit unscaled effective
contributions. Canonical catalyst/header absence proofs are implemented for Sapphire. Value
encoding, applicability/scalability and ordered magnitude rules are the next obligations in the
[item-scaling design](../../../../../docs/owned-item-scaling.md). Whole-item closure,
final resistance metrics and complete build parity remain incomplete.

The Life input declaration preserves that second source member; it does not
supply a Life effect. Source layout completeness is separate from the Partial
item, modifier, quality and rule memberships in the owned package.

## Remaining receiver

`source-facts.json` records the exact cold/all-elemental BASE combination,
increase/more multiplier, override, truncation and clamp source. There is no
game receiver root in the shipped package yet. The native stat-owned receiver
mechanism is implemented, so common calculations need no fabricated usage, encounter
or class owner. Contributor completeness, real receiver data, limits, overrides and
selected actor identity must be resolved before final Stat/Metric coverage is claimed.

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
