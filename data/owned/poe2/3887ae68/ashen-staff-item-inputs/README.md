# Ashen Staff physical inputs

This authoring composes the checked Spell Damage family, the checked Firebolt raw
grant, and the existing ordinary-item input proofs for Original 05's Item 28
(source 594, template `1d75`). The exact starting endpoint is
`a1931151bd06b9950816b052ef7915f7f6fbe89db2b2503d579f9989407bbfe8`.
Complete-source validation passed in both JIT modes with identical observations:
16 full XML loads and 50 isolated complete-method controls per mode. Both real
publication tests pass with 20 CLI probes. The checked endpoint is
`runs/owned-ashen-staff-item-inputs-04/package`.

The saved item has one implicit `Grants Skill: Level (1-20) Firebolt` member and
one explicit `128% increased Spell Damage` member. The existing paired-template
proof is added to the current V3 policy without changing the previous singleton,
pair or finite-census rows. It preserves each physical occurrence and its source
order. Extra, missing, disabled or unrecognized members must prevent completion.

The six new slots are rarity `31cb`, fresh corruption `31cc`, optional raw
`LevelReq` `31cd`, saved empty-socket capacity `31ce`, catalyst kind `31cf` and
catalyst amount `31d0`. Existing header codecs and the fresh-item proof classify
the saved Rare state, `LevelReq: 26` and four `S` sockets with four `Rune: None`
rows. Capacity is distinct from installed augments. `Crafted` and saved affix
labels remain construction evidence; they are not invented raw Boolean inputs.
The witness confirmed that removing the saved affix label preserves both physical
members. Reusing ParseRaw can retain stale item-level/corruption state, so the
absence and false-value claims apply to the checked fresh construction only.

Ordinary quality `0006` is appended to Ashen Staff's allowed quality members so
the explicit `Quality: 20` can be represented. The allowed-kind closure remains
Partial. This also independently admits saved quality 9 on Original04's selected
staff; its occupied rune and modifier inventory remain unresolved.
No absent-quality default is authored. Fresh missing item level is
eligible for an `Absent` default only after the complete source layout is proved.
The existing None-kind / helper-effective-20 catalyst defaults and transport
program are reused; they do not manufacture catalyst property tags on either
modifier.
Source load normalizes missing ordinary quality to zero, but this package keeps
those distinct raw encodings and supplies no missing-quality default. The active
source list also contains derived `QualityOnWeapon1 BASE20`; that is not a third
physical modifier or a newly authored native effect.

The helper validates the original endpoint, calls the checked Firebolt pipeline,
then appends this physical profile. It checks registry-prefix preservation, exact
descriptor and program additions, every prior policy row and all original query
bytes. Static template input/owner coverage, Firebolt action configuration,
effective skill inputs and receiving damage calculations remain incomplete.
Completing these physical lists is not a whole-build evaluation result.
The five selected issue counts are 127 / 128 / 120 / 157 / 24; all five remain Pending.
