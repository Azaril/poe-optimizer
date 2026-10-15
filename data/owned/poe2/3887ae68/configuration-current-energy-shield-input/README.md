# Configured Player current Energy Shield percentage

This packet imports `multiplierCurrentEnergyShield` as one raw Player Scenario
assumption, ExternalInput `3399`, using existing percentage-points Unit `0002`.
It starts from the Demonflame package
`3b3e135ec63ce4abb92105efe56ad1bb7cb802759f8473070a11811f2d1a6e7d`.
The admitted transport domain is ±1,000,000 percentage points. Negative,
fractional and large raw values remain unchanged within that domain. This is a
configured current-resource percentage, not maximum Energy Shield or resource
points.

The existing `NonzeroInputThenPlaceholder` projection selects a nonzero saved
Input, then the saved Placeholder, then the reviewed constructor value 100.
Explicit Input zero therefore falls through; an explicit Placeholder zero is a
known raw candidate. Both saved lanes require the exact scalar-input receipt.
Malformed, duplicate or out-of-domain lanes keep their unresolved import
responsibility, including malformed losing lanes. Missing saved lanes use the
authenticated constructor; a missing validated candidate is unavailable rather
than a known zero.

## Both callback responsibilities are retained

The complete original `ConfigOptions.lua` callback at lines 157–160 contains
exactly two unconditional writes, in order:

1. `Condition:UseCurrentEnergyShield`, FLAG true, on the Player Config list.
2. `Multiplier:CurrentEnergyShield`, BASE equal to the raw candidate, on that
   same list.

The count dispatch at `ConfigTab.lua` lines 1174–1200 invokes this callback only
for a nonzero selected value. Consequently, one validated raw candidate retains
all information needed to derive this source's two outputs: nonzero gives the
flag and the unchanged percentage; zero gives no callback records. Zero does
not emit FLAG false and does not prove the global condition false: other flag
suppliers still require their complete Boolean census. No native Boolean
writer, percentage modifier, consumer, cap or effect is added by this packet.

`source-evidence.json` records this complete derivation explicitly. The verifier
authenticates the full original callback and dispatch bodies, then checks the
two actual default/saved callback observations in each independently retained
source02 report. Each call has argument 100 and appends exactly the two full
ordered records; the complete remaining Config state is unchanged. Catalogue
row 14 has type `count`, constructor Placeholder 100, UI default state zero and
no constructor Input. The `ifCond` property controls UI visibility, not the
callback dispatch or this import's applicability.

The source consumer boundary at `ModParser.lua` lines 3321–3330 confirms the
Player resource meaning and retains the condition-dependent elemental damage
reduction and armour rules. Their 150 limits, numerical transformations,
applicability and delivery remain unimplemented by this input packet; the
original comment about the armour limit's uncertainty is preserved. There is
no new source VM execution, no synthetic control described as a source
observation, and no whole-source supplier or whole-build completion claim.

## Evidence and validation

The packet reuses authenticated complete source archives and their upstream
license from the existing configuration evidence. It adds no duplicated source
archive. All data JSON uses LF without a BOM.

Ordinary Rust tests authenticate target/unit/selector/default and both leaf
receipts; reject altered source pins, a missing callback responsibility,
global-condition inference, native-output claims and a substituted cap; check
the complete observed callback states; and validate the finite source-local
derivation for negative, fractional, zero, above-150 and boundary candidates.
Missing and invalid candidates remain distinct from callback suppression.

The opt-in publication target runs the actual CLI against all five unchanged
original builds and performs exact draft/sidecar inverses, local-ID and
110-query preservation, byte-identical rebuild, 20 source/Encounter controls
and two duplicate-lane controls. No owners, programs, usages or completeness
declarations are added.

Validated publication:
`runs/owned-configuration-current-energy-shield-publication-01/package`, input
`5f4eac0385d5785bf5534f8ae0be00e4a7c718dcf87c18057eeeabb7391a75f0`.
The registry ends at `3399`; the package contains 18 files / 64,457,998 bytes.
Original05's configuration origins move from `(20, 6, 14)` to `(19, 5, 14)`
by accounting for source ordinal 441. Each original gains one assumption;
selected issue counts remain `[106, 117, 109, 123, 4]`. All 110 queries and
prior values, targets, usages, local IDs and completeness gaps remain intact.

- `runs/owned-configuration-current-energy-shield-ordinary-01.log`: four
  ordinary tests passed in 2.06 s.
- `runs/owned-configuration-current-energy-shield-clippy-01.log`: strict
  target Clippy passed in 1.10 s.
- `runs/owned-configuration-current-energy-shield-publication-01.log`: all-five
  publication, 20 source/Encounter controls, two duplicate controls and exact
  rebuild passed in 62.44 s.

No source acquisition or test retry was needed. The actual source observations
remain the retained constructor/default and saved calls at 100; mutation
controls validate this import and its source derivation, not new PoB runs.
