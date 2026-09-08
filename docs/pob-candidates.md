# Supplied PoB catalogs and legacy calibration bridge

`search-calibration` compares complete caller-supplied build documents through the optional
PoB backend. The command requires a catalog and has no embedded character or fixture fallback.
It verifies finalist numerical consistency, while generic requested-versus-realized fidelity
and game legality remain explicitly unverified. The older four-fixture canonical projection
is retained only in test support, as described below.

## Supplied catalog CLI

From the repository root, with the optional PoB backend available:

```powershell
New-Item -ItemType Directory -Force runs | Out-Null
cargo run --release --locked -- search-calibration --catalog examples/calibration-catalog.json --objective examples/calibration-objective.json --jobs 4 --max-evaluations 5 --timeout-seconds 60 --output runs/calibration-search.json --export runs/calibration-winner.xml
```

The [example manifest](../examples/calibration-catalog.json) explicitly selects the four
original calibration fixtures. Another manifest can select unrelated complete builds. The
required input shape is:

```json
{
  "schema_version": 1,
  "builds": [
    { "id": "mapping-build", "path": "builds/mapping.xml" },
    { "id": "boss-build", "path": "builds/boss.import.txt" }
  ]
}
```

Relative paths resolve from the manifest's directory; absolute paths also work. Each input
is decoded through the portable importer, accepting exact PoB XML or a compressed share code.
The manifest admits 1..64 entries with distinct IDs of 1..128 ASCII letters, digits, `.`, `_`
or `-`. Paths contain 1..4096 bytes and no control characters/newlines. Unknown JSON fields,
duplicate decoded XML, malformed imports and missing files fail before any comparison.
The manifest is limited to 64 KiB; each source is at most 8 MiB, share-code text at most 1 MiB,
each decoded XML at most 8 MiB, and total decoded XML at most 32 MiB. Portable XML structural
limits also apply. No rejected input is replaced with a fixture.

An opaque candidate selects an exact supplied ID/XML-hash pair. This proves membership in
the requested finite document list, not a legal six-dimensional game state. Every evaluation
uses the document's imported action selection and configuration with no scenario override.
Different documents may therefore describe different encounters. Objective JSON uses the
same configurable typed metrics and constraints as `evaluate --objective` and `assess`.

The total evaluation budget includes failed attempts and one reserved fresh finalist attempt.
Proposal, beam and archive limits derive from the entry count. Five attempts cover the four
example alternatives and verification of the best feasible result; a smaller budget produces
a partial report. The shared deadline starts after input preparation and includes search and
verification. Each worker also has a 30-second cap. Isolated PoB processes are supervised by
bounded OS threads. `--pob` selects the pinned source directory when it is elsewhere.

### Report and export evidence

Report schema 2 records the manifest/path/hash, per-source path/format/input hash and exact
decoded XML hash, objective, search settings/counters, ranked assessments, backend identity,
warnings and verification results. `realized_observations` retains each candidate's latest
successful backend summary, context, selected-action coverage and normalized-export hashes,
plus its successful evaluation count. These observations are reporting evidence; they never
serve a cached calculation or certify the requested document's full semantic realization.
Backend identity changes during the run reject the affected measurements.

`best_verified` exists only when the best feasible candidate passes fresh numeric verification.
Both it and the domain evidence mark `generic_realization` and `game_legality` as `unverified`.
The report distinguishes requested XML from backend export identity, even when their hashes
happen to match. PoB may normalize or reinterpret imported state; numerical consistency does
not establish that every source mechanic survived unchanged.

`--export` writes the selected exact requested XML, without spending another evaluation, only
after fresh numeric verification. The `exact_requested_xml` source label and
`does_not_certify_pob_normalization` flag preserve that distinction. If no feasible result
passes verification, the diagnostic report explains why no XML was written. Existing output
files are never overwritten and JSON/XML destinations must differ. Processing all supplied
states means only that this finite list was processed.

## Legacy calibrated candidate bridge

The test-only [calibrated candidate helper](../crates/poe-optimizer-pob/tests/support/calibrated_candidate.rs)
connects the shared canonical candidate model to the four committed Mace calibration
fixtures, verified by SHA-256. It is compiled only by the candidate materialization test;
the PoB library no longer exports a fixture-specific candidate module. Its fixed hashes and
realization guards remain unchanged for regression coverage. Production document comparison
and build search continue to use caller-supplied input.

The end-state candidate/search contracts retain class, ascendancy, paid passives, equipment,
active skills and supports. This legacy bridge maps only a level-60 Warrior with no ascendancy
or paid passives, one Mace Strike group, one equipped normal weapon, and optional Brutality I.
Other class/tree/skill/item configurations require a general source-preserving projection with
requested-versus-realized tests, rather than extending production fixture allowlists.

### Test helper and identities

The regression test constructs its registry with `PobCandidateCatalog::from_builds(Vec<PobBuildAlternative { id, xml }>)`. A registry may contain any nonempty subset of the four approved sources. Display IDs must be unique; duplicate documents and changed XML bytes fail before calculation. Share strings must first be decoded with the ordinary importer; the registry requires approved raw XML.

`catalog()` exposes an immutable `CandidateCatalog`; `alternatives()` associates display IDs and source hashes with canonical candidates. `materialize(&Candidate)` accepts only exact registered states and returns their original XML bytes. It never patches XML or silently maps an unsupported mutation back to a known build. Changing even one candidate field, including the catalog identity, requires membership in the registry.

The catalog fingerprint binds the versioned projection, upstream revision, source manifest and sorted complete source-hash list. Input order and display labels do not change it; choosing a different finite subset does. Exact item and gem payloads have their own verified SHA-256 digests. Item instance IDs combine source occurrence `1` with the item-text digest, so the shared XML number `itemId=1` cannot alias Wooden Club and Smithing Hammer. The two weapon alternatives each describe one available copy, not two simultaneous copies of one physical item. The active/support instance IDs bind all imported gem attributes.

PoB automatically adds Warrior root `47175`. The canonical candidate's `passives` set remains empty because it contains only paid allocations; the catalog separately identifies the implicit class root. Source `classInternalId=6` is stable even though this pin rewrites its legacy `classId` field during export. This normalization is explicit and does not authorize other class changes.

### Requested-versus-realized checks

The test helper provides `validate_realization(&Candidate, &EvaluationResult)` for its narrow projection. The regression test applies this guard to fresh calculations; production code does not compile or use the helper. The guard checks:

- The pinned PoB backend, rules revision and source-manifest identity, plus the shared recorded-result contract.
- Level, class, ascendancy, implicit root, selected skill group and absence of extra or unresolved groups/minions.
- Active/support game IDs, variants, level, quality, count, enabled state, manual provenance and selected-action ownership. The selected action must remain the ordinary Mace Strike hit action.
- Exported active tree/item/skill sets, exact equipment slot and instance, and the equipped normal item's text. The only accepted item-text addition is the observed `LevelReq: 0` normalization. Item child elements, comments, mixed content, extra attributes, nonempty tree overrides/runes and unsupported gem configuration fail closed.
- The imported mapping encounter: no request overrides, MAIN mode and enemy level 60; the exact pinned set of 70 configuration inputs and 26 placeholders; exported configuration values, required normalized entries and an empty default custom-modifier block. Changed, missing or additional scenario inputs cannot silently change a candidate's score or export.

The frozen configuration tables are observations from the reference host used as legacy drift guards, exercised against all four fresh fixtures. They are **not independent mechanics goldens**. Actor condition tables are not frozen wholesale: support color and item effects legitimately change derived conditions. Existing independent host/extractor goldens separately validate the four numerical outcomes, as described in [calibration-reference.md](calibration-reference.md).

These checks certify the requested finite projection survived this adapter. They do not certify all game mechanics, acquisition rules, complete passive topology, or arbitrary combinations of generated skills. Results retain `diagnostic_only`. The 14 dangling source-tree edges remain visible in evaluator warnings and do not become an assertion of complete cross-class graph coverage.

## Independent calibration provenance

The original four-source reference values demonstrate why supports and weapons must be
considered together:

| Weapon | No support | Brutality I |
| --- | ---: | ---: |
| Wooden Club | 10.404968 | 13.6565205 |
| Smithing Hammer | 18.208694 | 11.0552785 |

Smithing Hammer without Brutality has the highest selected hit DPS within this four-document
corpus. Among support-equipped choices, Wooden Club is better. Brutality removes Smithing
Hammer's elemental damage, so summing independent weapon/support deltas would mis-rank these
combinations. These numbers use `TotalDPS` hit damage; `CombinedDPS` also includes an ailment
contribution and is a different metric. They are numerical regression references, not general
build recommendations or a production default corpus.

## Validation and expansion

[The candidate materialization target](../crates/poe-optimizer-pob/tests/candidate_materialization.rs)
compiles the historical helper and continues to test its source/payload identities, order-independent catalogs, unsupported mutations
and fresh realization of all four alternatives. Drift regressions cover passive allocations,
action/gem identity, item text and mixed content, backend provenance, encounter values,
custom modifiers, runes and stat sets. Its source-hash and realization guards are unchanged.

`tests/cli_search.rs` tests the required supplied catalog, serial/parallel ranking against the
four independent references, unrelated caller-owned Spark mapping/bossing documents and their
separate observed contexts, fresh verification, exact-source export, partial/infeasible runs,
manifest/input/total-size bounds, duplicate identities and output collision handling. These
are correctness checks, not throughput or scaling measurements.

The separate [controlled mutation adapter](controlled-mutations.md),
[experimental CLI](experimental-search.md) and [graph search](passive-equipment-assembly.md)
have their own legality and realization admission contracts. This opaque document-comparison
command neither widens nor weakens those contracts. Broader native integration still needs a
versioned projection covering active/inactive sets, gem parts, generated-skill ownership,
item identity/rolls, passive overrides and weapon-set allocations, with complete fresh
requested-versus-realized checks for admitted mutations.
