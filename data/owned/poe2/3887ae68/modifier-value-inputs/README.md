# Canonical modifier value inputs, revision 2

This package separates canonical raw modifier components from effective numeric recipes.
It covers cold resistance, all elemental resistances, four local weapon percentage
families and five local weapon flat damage pairs. The source policies migrate 27 reviewed
line grammars to eleven canonical owners. They do not certify complete modifier effects,
complete item membership or whole-build parity.

## Ancestor and compatibility

Build this revision from the ordered modifier-transform ancestor whose registry ends at
**9533**. It adds 321 entries, ending at 9854: the prior four shared stats, eleven canonical
modifier definitions and 295 parameter slots, plus eleven new required corrupted-base
factor slots 9844..9854. All existing allocated IDs retain their meaning; the eleven
canonical owners declare their known parameter membership Partial with the explicit gap
`modifier-eligibility-inputs-unconverted` so later eligibility inputs have an honest seam.

The earlier revision ending at 9843 is an immutable, superseded, unconsumed artifact.
Do **not** extend that package with this revision: its canonical parameter memberships
were Complete, and weakening them would violate the extension contract. Rebuild from 9533
and bind the fresh schema identity. Old packages remain reproducible under their existing
identities. This is a new authored revision, not reinterpretation of a previously loaded
package. All predecessor owners, nominal slots and programs from 9533 remain unchanged.

The extension adds canonical membership beside existing predecessor membership on exactly
338 item templates. Their Partial closures and gaps remain exact; no compatibility is
invented for other templates. `bindings.json` records old-to-canonical owner/slot mappings,
component roles, factor inputs and sixteen value compiler bindings. Those mappings do not
authorize copying an old rounded numeric input into a canonical raw slot.

## Raw source admission

`items.json` uses line-policy v4 numeric projections. Fixed values are captured anew from
the admitted source text with exact decimal handling. Ranged components use literal
`a + fraction * (b - a)` with validated quantity units, endpoint ordering and a resolved
finite fraction in [0,1], then fourteen-significant-digit decimal formatting/reparse to
match the pinned source interpolation boundary. They do not apply internal precision
rounding, corruption or modifier magnitude. Old rounded `interpolate_offset` outputs
are not reused. Direct resistance components retain their sign. Damage pairs require
unsigned fixed captures and unsigned range endpoints. Cross-zero ranges remain unresolved,
even when one selected value would be positive, until post-format admission is modeled.

Increased/reduced families emit both a nonnegative magnitude and the direction Boolean
from the same numeric projection. The direction uses the post-reparse sign, inverted by
the textual reduced qualifier; this preserves source antonym handling, including signed
zero, without reversing range endpoints. Fixed captures do not receive the ranged-value
formatting step. Fixed qualifiers allow an optional minus but reject an explicit plus,
which the pinned source parser does not accept. The source range prefix for the reviewed
weapon templates is still unknown, so those source lines remain Pending even when their
component conversion can be checked with an explicit fraction. Mixed fixed/ranged damage
pairs, ranged flat critical chance and other
unreviewed text remain outside these existing grammars.

Every property and qualifier slot is explicit and required. The source policy preserves
the prior property labels, fractured/desecrated flag bindings, template defaults and
layout rules. Unknown flags, `{corruptedRange:...}`, unsupported controls, combined-line
ambiguity, variant/crafted/advanced syntax and rune lifecycle stay blocked by source
attribution. Every known required roll must be valid and present. Known rolls retain
Pending collection closure in the owned draft because parameter membership is Partial;
recognizing their values cannot prove the input set complete.

Each admitted ordinary line explicitly emits corrupted-base factor 1 into its new
required roll. The existing strict source parser rejects the line-level controls that
would change this factor before this recipe can emit. An item's `Corrupted` header is
independent of the `{corruptedRange:...}` line control and does not imply another factor.
The current header policy does not recognize `Corrupted`, so such source items remain
blocked; this checkpoint does not add header admission.
This is a reviewed Import fact for a raw grammar match, not a runtime default. The slot's
0..1,000,000 range is a computational envelope, not a legal game-roll claim.

The six existing original-build admissions are preserved as raw facts: original 02 has
physical 11/19, cold 49/74 and lightning 6/179; original 03 lightning 13/298; original 04
cold 5/13; original 05 ranged cold resistance 20..30 at 0.5 -> 25. The other 22 matched lines
retain their source blockers. A fractional contrast such as 20..30 at 0.55 must preserve
raw 25.5 instead of the old rounded 26. These values do not establish final cache history.

## Numeric programs and unresolved authority

The input extension appends fifteen programs: eleven `corrupted-base-factor` programs
copy each explicit required factor roll to the shared Modifier-scoped factor stat 2541,
and the two resistance owners retain their four reviewed catalyst/ordered-magnitude
programs. The numeric compiler appends sixteen further programs. No local catalyst
producer, local magnitude producer or magnitude default is added. Missing inputs remain
unavailable, and all eleven canonical parameter memberships remain Partial.

The sixteen value bindings retain internal precision 1 and final decimal precision 0,
except flat critical chance, which uses 100 and 2. Shared Modifier-scoped output stats are
253e (effective percent),253f (damage minimum) and 2540 (damage maximum). They read distinct
corrupted-base 2541 and final ordered magnitude 253d; catalyst-only 0a1b is not a substitute.
Full keys are `def.000000000000` plus the hexadecimal suffix. Each stat belongs to its
exact modifier occurrence; shared definitions do not merge unrelated occurrences.

The raw grammar result is not proof that PoB accepts the final formatted modifier text.
For example, its parser rejects bare positive/zero resistance values; a leading plus
on a range survives positive selections but disappears at zero. The four resistance
range recipes therefore preserve raw facts under unresolved final admission authority.
The numeric formatter oracle records parser acceptance separately from component parity.
Do not turn these raw facts into final contributions until that distinction is resolved.
Pinned source has different fixed/ranged recomputation paths, and fixed values can retain
history from the last nonidentity magnitude step. Raw admission therefore does not justify
substituting the final factor into every source cache path. Rune serialization may bake
values and remains blocked. All existing Partial rule gaps, including
`canonical-input-admission-unproved`, stay present. These programs do not emit a character
contribution or declare modifier eligibility. Whole-build coverage remains 0/5, with the
110 query rows and allocation/rune/item-order Pending gates preserved.

## Published revision and reproduction

The validated revision is `runs/owned-canonical-admission-01/package-final`: **9854 registry
entries**, **39,085,400 artifact bytes**, and schema identity
`b123017cad2ea552ca95ae943c18b2cb05f055d0db5557df6ca14afbfbb729ce`.
The artifact-byte total excludes the publication transition report. This is an owned
component bundle; publication does not run character calculations or establish parity.

Reproduce the checked publication through the ordinary validator/compiler boundary:

1. Extend the 9533-entry ancestor with `extension.json` into `inputs-package`. This adds
   321 entries and fifteen programs and establishes the exact successor schema identity.
2. Bind compiler `policy.json` and v4 `items.json` to that schema. Validate both policies,
   then bind `item-source.json` to the validated item-line digest.
3. Run `compile-owned-modifier-values` with that bound policy against `inputs-package`,
   producing `numeric-package` with sixteen additional programs and no new definitions.
4. Use an empty admission extension against `numeric-package`, supplying the exact new
   item-line/source policy pair, to publish `package-final` atomically. This final step changes
   the source recipes without adding definitions, programs or receivers.

The run directory records `inputs-publication.json`, `numeric-publication.json` and
`publication-final.json` for these three publications. Schema identity is unchanged by the last
two stages; source-policy and rules identities are validated independently. Never guess a
digest, extend the old 9843-entry artifact, or substitute a prior source recipe revision.

## Remaining work

Next, prove magnitude/eligibility and fixed/ranged cache-path authority as separate owned
contracts, then validate weapon/character channel assembly. Introduce additional source
encodings only through reviewed policies and Rust contrasts. Retire predecessor owners
through an explicit migration after every consumer moves; do not reuse their IDs or
remove them from previously Complete declarations. All new tooling/tests remain Rust;
existing Python checks are unchanged.
