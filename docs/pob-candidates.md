# Calibrated PoB candidate materialization

`poe-optimizer-pob::candidate::PobCandidateCatalog` connects the shared candidate model to an exact, finite set of PoB documents. The current registry accepts only the four committed Mace calibration fixtures, verified by their SHA-256 hashes. It is an internal integration path for search, locking, evaluation, ranking and verification. It does not import arbitrary builds into a mutable candidate or establish the first general optimizer release.

The end-state candidate/search contracts retain class, ascendancy, paid passives, equipment, active skills and supports. This bridge deliberately maps only a level-60 Warrior with no ascendancy or paid passives, one Mace Strike group, one equipped normal weapon, and optional Brutality I. New class/tree/skill/item configurations need explicit mappings and requested-versus-realized tests before this registry accepts them.

## API and identities

Construct the registry with `PobCandidateCatalog::from_builds(Vec<PobBuildAlternative { id, xml }>)`. A registry may contain any nonempty subset of the four approved sources. Display IDs must be unique; duplicate documents and changed XML bytes fail before calculation. Share strings must first be decoded with the ordinary importer; the registry requires approved raw XML.

`catalog()` exposes an immutable `CandidateCatalog`; `alternatives()` associates display IDs and source hashes with canonical candidates. `materialize(&Candidate)` accepts only exact registered states and returns their original XML bytes. It never patches XML or silently maps an unsupported mutation back to a known build. Changing even one candidate field, including the catalog identity, requires membership in the registry.

The catalog fingerprint binds the versioned projection, upstream revision, source manifest and sorted complete source-hash list. Input order and display labels do not change it; choosing a different finite subset does. Exact item and gem payloads have their own verified SHA-256 digests. Item instance IDs combine source occurrence `1` with the item-text digest, so the shared XML number `itemId=1` cannot alias Wooden Club and Smithing Hammer. The two weapon alternatives each describe one available copy, not two simultaneous copies of one physical item. The active/support instance IDs bind all imported gem attributes.

PoB automatically adds Warrior root `47175`. The canonical candidate's `passives` set remains empty because it contains only paid allocations; the catalog separately identifies the implicit class root. Source `classInternalId=6` is stable even though this pin rewrites its legacy `classId` field during export. This normalization is explicit and does not authorize other class changes.

## Requested-versus-realized checks

Every search calculation and fresh verification calls `validate_realization(&Candidate, &EvaluationResult)` before scores become available. It checks:

- The pinned PoB backend, rules revision and source-manifest identity, plus the shared recorded-result contract.
- Level, class, ascendancy, implicit root, selected skill group and absence of extra or unresolved groups/minions.
- Active/support game IDs, variants, level, quality, count, enabled state, manual provenance and selected-action ownership. The selected action must remain the ordinary Mace Strike hit action.
- Exported active tree/item/skill sets, exact equipment slot and instance, and the equipped normal item's text. The only accepted item-text addition is the observed `LevelReq: 0` normalization. Item child elements, comments, mixed content, extra attributes, nonempty tree overrides/runes and unsupported gem configuration fail closed.
- The imported mapping encounter: no request overrides, MAIN mode and enemy level 60; the exact pinned set of 70 configuration inputs and 26 placeholders; exported configuration values, required normalized entries and an empty default custom-modifier block. Changed, missing or additional scenario inputs cannot silently change a candidate's score or export.

The frozen configuration tables are observations from the production host used as drift guards, exercised against all four fresh fixtures. They are **not independent mechanics goldens**. Actor condition tables are not frozen wholesale: support color and item effects legitimately change derived conditions. Existing independent host/extractor goldens separately validate the four numerical outcomes, as described in [calibration-reference.md](calibration-reference.md).

These checks certify the requested finite projection survived this adapter. They do not certify all game mechanics, acquisition rules, complete passive topology, or arbitrary combinations of generated skills. Results retain `diagnostic_only`. The 14 dangling source-tree edges remain visible in evaluator warnings and do not become an assertion of complete cross-class graph coverage.

## CLI calibration search

From the repository root:

```powershell
cargo run --release --locked -- search-calibration --objective examples/calibration-objective.json --jobs 4 --max-evaluations 5 --timeout-seconds 60 --output runs/calibration-search.json --export runs/calibration-winner.xml
```

Create the output directory first. Existing destinations are never overwritten. `--pob` accepts the pinned source directory when it is elsewhere. The command embeds the four fixture XML documents; it does not accept arbitrary build inputs. Objective JSON uses the same configurable typed metrics and constraints as `evaluate --objective` and `assess`.

The total evaluation budget includes failed attempts and the one attempt reserved for fresh verification. Five attempts cover all four alternatives and verification of the best feasible result. A smaller budget produces a partial report. The shared wall-clock deadline includes search and verification after input preparation; each worker also has a 30-second cap. Isolated PoB processes are supervised by bounded OS threads. Native CPU evaluators can use the generic search engine's Rayon execution path without changing this candidate interface.

The report records the catalog, exact payloads and source hashes, objective, finite candidate constraints, complete search settings/counters, one consistent backend identity, warnings, ranked feasible/infeasible results and verification evidence. `best_verified` is present only when the best feasible candidate passes a fresh evaluation. `--export` writes its exact materialized **source** XML, already checked against the fresh runtime export; this does not spend another evaluation. If no feasible result passes verification, the command still writes its diagnostic report and explicitly omits XML export. Processing all four supplied states means only that this finite domain was processed.

The independent selected-hit-DPS values demonstrate why supports and weapons must be considered together:

| Weapon | No support | Brutality I |
| --- | ---: | ---: |
| Wooden Club | 10.404968 | 13.6565205 |
| Smithing Hammer | 18.208694 | 11.0552785 |

For the supplied hit-DPS objective, Smithing Hammer without Brutality wins globally. Restricting consideration to support-equipped choices makes Wooden Club the better weapon. The support removes Smithing Hammer's elemental damage, so summing independent weapon/support deltas would mis-rank combinations. These numbers use `TotalDPS` hit damage; Smithing Hammer's `CombinedDPS` also contains an ailment contribution and is a different metric.

## Validation and expansion

`crates/poe-optimizer-pob/tests/candidate_materialization.rs` checks exact source/payload identities, order-independent catalogs, rejected unsupported mutations, finite-domain validation, and fresh process-based realization of all four alternatives. Drift regressions alter passive allocation, action/gem identity, item text and mixed content, backend provenance, encounter inputs/placeholders, custom modifiers, runes and stat sets.

`tests/cli_search.rs` checks serial/parallel ranking against all four independent numerical references, fresh verification and source export, partial budgets, infeasible/no-export behavior, invalid objectives/limits and output collision handling. These are correctness checks, not throughput or scaling measurements.

The separate [controlled mutation adapter](controlled-mutations.md) and
[experimental CLI](experimental-search.md) now provide parameterized normal-Mace choices;
this four-hash registry remains the immutable calibration path.

Broader integration should extend the source-preserving, versioned projection that accounts for all active/inactive sets, gem parts/stat sets, generated-skill ownership, item identity/rolls, passive overrides and weapon-set allocation. Each supported mutation must compare intended state with fresh realized state before it enters search. The finite-only CLI and diagnostic labels should remain until those broader mappings and all-dimension parity gates are established.
