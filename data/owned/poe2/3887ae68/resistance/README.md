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

`items-fixed.json` contains general signed integer captures for plain cold,
all-elemental and Life lines. `item-source-fixed.json` binds those declarations
to the exact schema and source layout. Metadata headers do not become ItemLevel
or Quality facts. Unsupported lines, tag semantics, ambiguous preceding lines,
missing template membership and range lifecycle information remain unresolved.
The source adapter still must prove line positions and whole-member indexing;
matching a plain line is not proof that a whole item is converted.

The reviewed fixed outcomes contribute +10 cold, +5 cold, +5 all-elemental, and
−5 all-elemental from existing Reward identities. The mixed negative outcome
retains Partial program coverage for its other six effects. Programs target the
selected player's contribution set, without copying a cached build total.

## Range boundary

No executable ranged policy is published. Two generic patterns for fixed and
ranged numbers overlap structurally before numeric codecs run. Choosing the
decoder that succeeds would hide that ambiguity. In addition, pinned
`ItemTools.formatValue` uses symmetric half-away rounding, whereas the current
item interpolation operation only offers nearest-ties-positive. Negative half
ties differ. A general lexical capture-shape constraint and signed half-away
operation are needed before admitting the ranged syntax.

Original 05 item 26 is one Sapphire Ring record referenced by eight receiving
rows across four item sets. Its source range is 20–30; fractions 0, 0.5 and 1 give
20, 25 and 30 in the pinned formula. These are source-example facts, not runtime
conversion or full-build results. Original 02 item 26 is a Grand Spear and is
kept distinct. No fixture name, item number, source path or cached measurement
selects a production handler.

The Life input declaration keeps the ring's following `+10 to maximum Life`
line semantically visible. It does not supply a Life effect or permit a range
proof to ignore that source member.

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
```

To regenerate, replace `--check-dir` with `--output-dir NEW_DIRECTORY`. Output
creation refuses an existing directory or file; an I/O failure can leave an
incomplete new directory. This exporter is not an atomic runtime publisher.
The production `assemble-owned-recipe` command independently validates and
publishes the recipe. Item policies must be loaded against that exact schema;
existing import mapping/role/policy artifacts are not silently rebound.

Source file and span SHA-256 hashes use LF-normalized source bytes. The recipe
digest covers exact artifact bytes. Source pins and source text are offline
provenance, and are not read by the rule executor.
