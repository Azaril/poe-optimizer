# Owned mechanics components: PoE 2, source 3887ae68

This is persisted production input for the owned recipe assembler. It supplies complete finite level tables and reviewed component rules for Twister and Skeletal Sniper. It is not a complete game catalog or a claim that either original build is fully evaluated.

The package contains 38 registered owned allocations, seven scalar tables with 40 rows each, and explicit Gem → generated Skill / Skill → actor grants. Nine rule owners and four action-routing entries remain Partial. Those gaps are deliberate: other modifiers, conditional skill behavior, minion damage scaling, supports, actor ability instances, and full metric coverage are not supplied by these components. Protected-original complete native evaluation remains 0/5.

## Files

| File | Purpose |
|---|---|
| recipe.json | Source-independent RegistryInput, SchemaPackageInput, RulePackageInput with operations v5, and ActionRoutingInput. Accepted directly by the production assembler. |
| ids.json | Human-readable inventory of the exact persisted typed IDs. Labels are authoring conveniences, not runtime source-name dispatch. |
| source-manifest.json | Import-only source file/span pins, exact literal row shapes, selected identities, table/literal bindings, reviewed algorithm spans, and unresolved source references. |
| mechanics-facts.json | Reproducible source evidence, exact numeric tokens, full row census, winning identity declarations, and association with the exact recipe bytes. |

Runtime compilation/evaluation needs the assembler's schema/rules/routing outputs and an owned request. It does not need the source manifest, source checkout, legacy game-data package, or PoB runtime.

## Assemble and consume

Run the built project CLI with an output directory that does not exist:

    poe-optimizer assemble-owned-recipe data/owned/poe2/3887ae68/recipe.json --output runs/my-owned-components

The production assembler validates the registry, canonical schema identity, exact schema/rule/routing bindings, rule semantics, and finite table structure before publishing its artifacts. This is a component validation step, not metric or original-build parity.

A caller with a separately constructed owned request can consume those published artifacts with resolve-owned-effects and explicit --input, --schema, --rules, and --routing paths. No source-selected skill, actor, or UI index is inferred. The integration target owned_recipe_real exercises the public assembler/evaluator with these persisted files rather than private fixture assembly.

## Reviewed semantics

- Twister levels 1–40 retain base damage factor, attack-speed percentage, character-level requirement, and mana cost. The recipe computes attack factor as 1 + percentage / 100, and extra-Twister chance from the independently source-bound standard-quality coefficient.
- Sniper levels 1–40 retain character-level requirement and spirit reservation. The ordinary minion-level table projects levels 2–80 into the exact child actor; skill level 20 maps to actor level 40. The standard-quality coefficient projects a separate damage factor. Base spirit reservation belongs to each generated summoning action, so separate Sniper uses retain distinct per-skill costs; player-level cost aggregation and multiplicity modifiers remain unconverted.
- Each standard-quality stat multiplies its coefficient by raw quality, then truncates toward zero in percentage points before accumulation, matching the pinned CalcTools algorithm. Raw quality 20.5 projects unchanged but contributes 20 percentage points: Twister chance 20 and Sniper factor 1.2. Quality 0.9 contributes zero. The source's includeAltQualityStats adds alternate stats alongside standard stats; that additive mode remains a Partial game-rule/input obligation, with no invented alternate quality kind or switch.
- Physical Gem level and quality are projected through the declared primary SkillGrant. An explicitly absent owned quality selection contributes zero quality bonus through a lazy presence check. A missing demanded value stays unresolved. The generated Skill level's computation domain is 1–40; physical input transport bounds are broader and do not claim game legality.
- Character-level requirements produce separate requirement results. A failed requirement does not rewrite numerical coefficients or turn an unsupported input into a default.
- Skeletal Sniper's Basic Attack and Gas Arrow retain separate actor output declarations and source skill potential membership. These output ports do not invent activated ability Skill instances.
- The two source stat-set actorLevel tables are retained as scaling-coordinate evidence only. In particular, 97.699996948242 at level 20 is not the summoned actor's level.
- The declared and constructed CommandSkeletalSniperPlayer references are missing from the actual identity catalog. Both ledger rows survive, and the corresponding grant, skill-grant, and owner-program membership remain Partial; no replacement Skill or actor is fabricated.

Known schema ranges and empty direct ports describe this reviewed input model; they do not certify all game rules. The quality envelope is a bounded computation envelope, not a gameplay quality cap.

## Reproduce the optional offline export

No Lua execution is involved. From the repository root:

    python scripts/export-owned-mechanics.py --manifest data/owned/poe2/3887ae68/source-manifest.json --recipe data/owned/poe2/3887ae68/recipe.json --identity-catalog crates/poe-optimizer-data/data/game-data.json --source-root vendor/path-of-building-poe2 --check-facts data/owned/poe2/3887ae68/mechanics-facts.json

For a new reviewed source/recipe revision, replace --check-facts with --output-dir pointing to a new directory. The exporter only replaces declared table cells and exact numeric Literal nodes in a copy of the supplied recipe. Identities, schemas, operation choices, and topology remain authored data. It does not allocate new IDs, choose between ambiguous declarations, infer missing numeric values, or interpret source expressions.

File and span hashes normalize source CRLF to LF; source numeric token spellings are retained. The catalog pin uses LF-normalized bytes. The generated JSON artifacts use exact UTF-8 LF bytes, including the final newline, and are protected by the repository's exact-byte attributes.

The exporter reserves its output directory exclusively and writes files with exclusive creation. It never overwrites even an existing empty directory. If writing fails after reservation, it reports and leaves the incomplete new directory for inspection. This export directory is not an atomically published runtime bundle; atomic validated bundle publication belongs to the production assembler.

Run the Python regression suite with:

    python scripts/tests/test_export_owned_mechanics.py -v

Source drift, expressions, duplicate/missing rows, nonintegral integer cells, missing-reference changes, byte/row/cell bounds, unsafe paths, ambiguous numeric captures or literal bindings, implicit-array key relabeling, and output collisions fail closed. Aggregate table expansion is charged before row builders run. Algorithm evidence is exact pinned text, never executed. Source facts are evidence, not permission to weaken the Rust constructors. Coefficient-only changes retain IDs and change the recipe digest; additions require a reviewed registry successor rather than rerunning fresh allocation.

## Broad raw equipment inputs

The [weapon-profile catalog](weapon-profiles/README.md) extends the item-base successor
with all 337 finite attack profiles, all 14 raw numeric fields, explicit absence policies,
and typed equipment-local outputs. Native compilation uses no source checkout or Lua.
Compact v2 publication preserves the same checked constituent/policy/query contracts while
omitting the duplicate recipe. Local item effects and final action assembly remain open.

## Explicit headers and finite actor/augment acquisition

[Item header inputs](item-header-inputs/README.md) replace two opaque metadata rules
with explicit quality and item-level values using the existing injected codecs.
The checked publisher accepts exact paired item/source policy replacements; all
calculation/schema/query artifacts are preserved.

[Actor baselines](actor-baselines/README.md) and [augments](augments/README.md) are
finite optional acquisitions for the next native semantic converters. They retain
source absence/order and unconverted coverage, and introduce no PoB dependency into
the native evaluator. The original five builds are still incomplete natively.

## Player attribute contributions

[Actor attribute inputs](actor-attribute-inputs/README.md) connect the existing formatted
item attribute amounts to shared Player Integer contribution channels. This data-only
extension preserves partial coverage; final attribute and resource calculations remain open.

## Ordinary passive attribute contributions

[Full-list passive attribute inputs](passive-attribute-inputs/README.md) add 58 reviewed
plain flat/increased attribute nodes through the same converter as class-dependent views.
Both ordinary and ascendancy pools retain their identities. Mixed-effect nodes and final
attribute/resource stages remain explicit integration work.
