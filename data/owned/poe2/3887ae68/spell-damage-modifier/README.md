# Fixed increased Spell Damage

This authoring adds a numeric modifier for Original 05's Ashen Staff (Item 28,
source 594, line 20): `128% increased Spell Damage`. It binds the exact Fine Belt
physical-input endpoint `a1931151bd06b9950816b052ef7915f7f6fbe89db2b2503d579f9989407bbfe8`.
The complete-source witness passed in both JIT modes, with 16 full XML loads and
50 isolated complete-method controls per mode. Original selections, source
functions and cached outputs were preserved. Publication checks remain pending.

The pinned parser recognizes increased percentages as `INC`, `damage` as
`Damage`, and `spell` as the `Spell` modifier flag. This restriction is distinct
from catalyst properties. An untagged saved line must not acquire the `caster`
property from its English name or saved affix label. The numerical stage does not
yet project damage to an actor or skill.

The 25 allocations are modifier `31ad`, raw amount `31ae`, twenty property flags
`31af`–`31c2`, unscalable `31c3`, corrupted factor `31c4`, and category `31c5`.
Existing category Options, catalyst inputs and transform channels are reused.
Raw and effective amounts have percentage-point unit `0002`; effective stat
`253e` is already a modifier-scoped quantity with that unit. It is not a rate.
The existing numeric compiler applies precision 1 / display precision 0 and the
existing corrupted-base, catalyst and ordered-magnitude stages.

The initial admission is a fixed unsigned integer from 0 through 1,000,000,
without source scaling tags or generated buff members, with initial scaling one.
The category comes from the checked source layout. The first complete-source
run observed zero as one clean, independent `Damage INC 0` member. Fractional,
ranged, signed and transformed source encodings are outside this admission.
Both JIT modes produced identical observations. The 24-member parameter inventory
is Complete only for the admitted canonical family. Fractured/desecrated flags
survive in source and are rejected by the existing unconsumed-property guard;
they are not silently discarded. Unknown categories or source encodings withhold
canonical conversion.

Measured component controls are 128 unchanged for the actual untagged line,
192 after an explicit 50% magnitude or corrupted-range factor, and 153 for an
explicit caster property with Sibilant catalyst 20. An untagged line, zero
catalyst amount or mismatching catalyst remains 128. The latter tagged/scaled
controls demonstrate numeric behavior; they do not broaden source admission.
Three native tests pass using the published programs and actual admitted rolls,
with the existing finite component fixture. They cover those contrasts, missing
inputs, source-occurrence isolation, reused scratch and retained Partial gaps.

The helper appends only this family's membership to Ashen Staff, compiles the
ordinary numeric program, and preserves all previous query bytes, policies,
owners and declarations except checked dependency commitments. Static rule-owner
closure remains Partial for receiving Spell-restricted damage, additional
transforms, source encodings and contributor membership. Complete item inventory,
item parameters, aggregate damage and whole-build evaluation are not established
by this numeric family.
