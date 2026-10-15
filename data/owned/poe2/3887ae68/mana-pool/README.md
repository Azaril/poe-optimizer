# Guarded Player Mana consumer

**Published as a guarded component.** This arithmetic fragment uses the existing typed rule engine,
including the [checked numeric-domain operation](../../../../../docs/owned-numeric-domain-proposal.md)
approved October 14. Transformation supplier admission and complete source
coverage still gate a production result. Publication establishes no complete-build
coverage and adds no acquisition or runtime backend.

The program reads the existing intrinsic/inherent Mana groups, reward/passive
increase groups, MORE group, five typed adjustment collectors and coherent
override selection. All game values and formulas remain in the injected data.
It derives the existing Mana stat `29f9` through a Player receiver.

The arithmetic order is:

1. Sum base contributions and increase groups independently.
2. Sum the three outgoing conversion percentages and cap their combined value
   at 100%; do not introduce a lower clamp.
3. Retain that fraction of base Mana, then add pre-scaling additional Mana.
4. Apply the increased/reduced factor, then the MORE/less multiplier.
5. Add post-scaling Mana, check the actual operand against the injected inclusive
   range `[-f64::MAX, 2^52]` in Mana units, round, then apply the ordinary minimum
   of one. An operand outside the proved range is explicitly unsupported.
6. A known override, including zero, bypasses ordinary arithmetic. Unknown
   override presence remains unavailable, and whole-request source coverage
   remains mandatory even when a scalar branch is unused.

The formula uses ordinary `Add`, `Scale`, percentage conversion,
`CheckNumericDomain`, `Round`, `Maximum` and lazy `Select` operations. It does
not copy a Lua callback or add a resource-specific Rust kernel. The domain
check is neither an effect-inactivity guard nor a game Requirement. Its bounds
are calculation data; the engine has no Mana-specific numeric limit.

`tests/owned_intelligence_mana.rs` composes this fragment with the real native
level, attributes, reward and override producers in the existing finite
component fixture. A separate scalar-boundary test injects all retained
original-source operands and compares source results; that test does not
claim those supplied aggregates are native producer implementations.

Validation: all **21 ordinary tests** in that existing target pass; five
publication tests remain opt-in. The current guarded publication also passes
explicitly against all five originals. The joined
fixture derives **638 Mana** from its real producers, responds to level and
attribute changes, and retains a present zero override. All **16** retained
source vectors match in forward/reverse order. Missing and Partial dependencies,
stage ordering and fresh/reused/four-worker comparisons remain checked. The
new controls cover inclusive and adjacent rounding bounds, extreme finite
operands, explicit unsupported results through the real contribution graph,
and a zero override that bypasses ordinary operands while retaining coverage
requirements. Evidence: `runs/owned-mana-pool-tests-01.log` (5.19s),
`runs/owned-mana-pool-publication-01.log` (20.98s), and strict target Clippy in
`runs/owned-mana-pool-clippy-01.log`. These controls do not prove complete
transformation suppliers or make the original request executable.

Resource transformations into the Extra/Total channels, item-granted origins,
global Partial closures and the original build's outstanding input obligations
remain separate work. The canonical package is
`runs/owned-mana-pool-publication-01/package`, input
`fa2829dc2d699ff8aaa0cf5d1e9e13209c466d6d0e979b0bf8450483196dc200`.

The existing full-release assembler adds exactly this owner, its Player receiver,
a rule-release label and one provenance row. Removing those additions restores
the complete predecessor authoring input. This is an explicitly reviewed release,
not a monotonic successor: adding a receiver to an existing complete receiver
inventory needs explicit review. No previous owner, contribution membership or
input inventory becomes Complete. No evaluation/stage bundle is added. Only
`rules.json`, `manifest.json` and `release.json` change; all eighteen artifacts
rebuild identically. All 110 query rows and selected issue counts
106/117/109/123/4 survive. The authoring/dependency packets stay offline.
