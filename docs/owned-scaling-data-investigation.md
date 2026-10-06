# Investigation: exact tables, curves and segmented scaling data

**Status:** Initial investigation complete; representation choice and implementation pending.
**Requested:** 2026-10-05. This extends the generated-artifact design, not the rejected database/ORM plan.

## Finding and recommendation

Some level data is a resolved discrete table; other data is evaluated from a
formula. A website displaying a table does not establish which produced it.
Keep the semantic function, its persisted encoding and its execution layout
separate. A small segmented artifact can still compile to an immutable dense
array for native, parallel and WebAssembly evaluation.

Investigate constant and affine segments with explicit literal exceptions as
**lossless encodings over a declared domain**. Do not fit a curve and use it to
replace authoritative values. Use genuine runtime curves only when a demonstrated
mechanic needs them and its input, rounding and boundary semantics are established.
Neither representation grants interpolation, extrapolation or a game level cap.

The current five-build work continues with exact tables. No wire format, rule
operation, historical receipt or numerical output changes in this investigation.

## Evidence

At pinned PoB revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`:

- [Skill export](../vendor/path-of-building-poe2/src/Export/Scripts/skills.lua),
  lines 618–627, uses `BaseResolvedValues` and emits interpolation mode 1 when
  there are more than five stat rows. Other branches retain scaling coefficients.
- [Calculation tools](../vendor/path-of-building-poe2/src/Modules/CalcTools.lua),
  lines 183–218, distinguish exact values, effectiveness scaling and piecewise
  linear interpolation. The effectiveness branch includes actor-level-dependent
  linear and exponential factors followed by rounding. The linear branch itself
  describes its interpretation as a guess: this is reference behavior to
  investigate, not sufficient evidence of a game law.
- [Minion data](../vendor/path-of-building-poe2/src/Data/Skills/minion.lua),
  `DeathStorm`, supplies a contrasting single-row effectiveness curve. Ice Nova
  and Pain Offering instead use resolved rows despite also carrying scaling
  metadata. Do not reactivate unused coefficients or confuse gem level, required
  character level and calculation actor level.

A targeted read of [PoE2DB's Pain Offering page](https://poe2db.tw/us/Pain_Offering)
on 2026-10-05 shows forty level rows with a change in damage progression after
level 30. Its [experience page](https://poe2db.tw/us/Experience) displays another
discrete progression; that presentation alone supplies no generating formula.
These are discovery examples, not a version-pinned replacement for current
artifacts. No scraping or bulk extraction was performed.

Targeted inspection of already-local pinned item data gives a useful contrast:

- [Amulet bases](../vendor/path-of-building-poe2/src/Data/Bases/amulet.lua),
  lines 33–39 and 61–67, store Lapis's required character level 8 separately
  from its 10–15 Intelligence roll, and Solar's level 30 separately from its
  10–15 Spirit roll.
- [Item modifiers](../vendor/path-of-building-poe2/src/Data/ModItem.lua),
  lines 96–98, store three named life-prefix tiers with level fields 33/60/75
  and roll ranges 3–4/5–6/7–8 percent. These rows contain discrete tiers and
  ranges, with no generating-curve metadata.
- [Item calculation](../vendor/path-of-building-poe2/src/Classes/Item.lua),
  lines 2235–2252, derives a character requirement using the base/rune minimum
  and `floor(mod.level * 0.8)`, then evaluates the selected roll range separately.
  That is a source algorithm to investigate, not an interpolation rule between
  tier levels. The records alone do not establish crafting legality enforcement.

These examples distinguish character requirements, affix-level metadata and
within-tier roll ranges. A segmented representation may encode their thresholds,
but must retain tier identity and eligible choices instead of smoothing them.

The checked Prolonged package's `rules.json` contains **16 tables / 835 cells**.
Its table section is approximately 119 KB when compactly serialized, against
60.8 MB for the full package. Tables are therefore not currently the principal
distribution-size cost. The development corpus is also too small to establish
the best encoding for the full game.

A read-only local census (`runs/curve-storage-audit-01.json`) exhaustively checked
exact rational affine reconstruction over every cell in those sixteen tables:

| Table | Cells | Minimum non-overlapping affine segments | Implication |
| --- | ---: | ---: | --- |
| Offering damage increase | 40 | 2 | Strong exact-segment pilot |
| Twister attack-speed percent | 40 | 1 | Constant run |
| Sniper actor level | 40 | 1 | Exact affine progression |
| Twister required character level | 40 | 6 | Bounded piecewise progression |
| Sniper Spirit reservation | 40 | 11 | More boundaries; measure encoded cost |
| Ice Nova ordinary cold minimum | 40 | 19 | Mostly short runs; literals may be smaller |
| Allied damage by actor level | 100 | 50 | Segments alone offer little benefit |

The census minimizes segment count, not bytes or lookup time. Two arbitrary
adjacent samples always form an affine segment; that is not evidence of an
underlying curve. In particular, rational reconstruction of decimal values and
direct binary64 multiply/add are different numerical contracts. Direct floating
evaluation differs at seven Twister damage-factor samples in this experiment.
Changing arithmetic order, using fused operations or fitting a smoother curve
cannot silently change existing results.

Offering's exact finite example is `18 + 2L` on integer levels 1–30 and `48 + L`
on 31–40. These two expressions reproduce the current forty values; they say
nothing about fractional levels, level 41 or future patches.

## Proposed boundaries to evaluate

The current [IntegerRuleTable](../crates/poe-optimizer-core/src/owned_rules.rs)
is an explicitly bounded dense integer lookup. Data validates complete row
coverage; Engine indexes the immutable table and refuses unavailable keys.
Native expressions already supply arithmetic and explicit rounding, but do not
provide a general exponential operation. Retain that working contract while
evaluating storage alternatives.

| Layer | Responsibility |
| --- | --- |
| Offline acquisition | Preserve pinned resolved values and genuine formula metadata with provenance; distinguish observed values from inferred patterns. |
| Owned authoring | Declare input meaning and domain, units, exact values or justified formulas, boundaries and rounding. Source interpolation codes remain in conversion. |
| Artifact encoding | Optionally choose literals, constant runs, affine segments or sparse exceptions; require lossless decode and bounded expansion. Share repeated unit/type metadata and measure ordinary compression as baselines. |
| Native compilation | Initially expand compact discrete encodings once to shared arrays. A later runtime curve or segment search requires demonstrated benefit and deterministic evaluation. |
| UI and search | Read the same versioned definitions/accessors. UI curves are visualizations of supported values, not an independent interpolator or new calculation authority. |

Gem-level indices, quality amounts with separately established granularity,
item-level requirements, tier availability, roll ranges, weights and actor levels
are different domains. A minimum item-level
threshold is not a sampled numerical stat curve. Preserve discrete choices,
intentional gaps and inclusive/exclusive boundaries rather than smoothing them.
Multi-input mechanics need explicitly named axes; a one-dimensional level table
must not absorb unrelated conditional logic.

Any proposed encoding must define canonical serialization, units and numeric
representation; reject gaps/overlaps and duplicate exceptions as appropriate;
bound row counts, arithmetic and allocation before expansion; and preserve
out-of-domain refusal. Exact finite verification can justify compression without
proving the original generating formula. It cannot authorize new inputs.

Keep storage identity and semantic identity explicit. Existing byte-bound
receipts must remain replayable; encoding changes need an explicit migration,
not an unchanged identity around newly interpreted bytes. Future semantic
deduplication may share equal tables internally without merging their public IDs.

## Follow-up milestones and decision gate

1. **S1 — representative census.** Initial sixteen-table scan is complete.
   Expand using already available pinned exports, including constant/piecewise
   stats, irregular damage, genuine actor-level curves, item-tier thresholds,
   roll intervals and quality granularity/continuous inputs. Record evidence and uncertainty for
   each domain. PoEDB remains targeted reference material, not a bulk data source.
2. **S2 — exact prototype and measurements.** Build Rust prototypes outside the
   production contract for dense typed arrays, compact arrays, segments plus
   exceptions, and ordinary compressed serialization. Verify every discrete
   value, boundary and refusal against canonical artifacts; report compressed
   bytes, cold load time, retained memory and repeated random lookup throughput.
   Include single-thread/Rayon workloads and native/WASM determinism. Formula
   tests must distinguish source observations from established game semantics.
3. **S3 — review the representation.** Present measurements and a proposed
   owned-format/compiler decision. Prefer compact storage expanded to arrays
   unless direct segment/curve evaluation demonstrates an advantage. Discuss
   any public format, domain or numerical-semantics change with the owner first.
4. **S4 — migrate and retire.** Implement the agreed representation, preserve
   historical decoding/receipts where needed, prove full-domain parity and
   original/holdout-build results, then remove superseded execution paths.

S1 and relevant semantic findings may proceed alongside D2/D3. Storage
optimization and broad benchmarks belong to D6 and must not displace the next
measured five-build blocker. A real mechanic requiring a runtime curve moves
that semantic work earlier; it does not justify speculative curve fitting.
