# Configuration inputs and native consumers

The configuration census is a dependency of the five original builds, not an
independent UI feature. Source configuration combines authored overrides,
constructor defaults, callback-derived placeholders, conditional modifier records,
skill choices and calculation policies. The native model must preserve their
meaning without executing source callbacks or retaining a PoB UI dependency.

The first implementation uses existing typed Scenario assumptions and native
rule programs. It adds no public Core or Engine contract. All game identifiers,
units, bounds, defaults and programs are injected through the
[configuration resistance family](../data/owned/poe2/3887ae68/configuration-resistance-inputs/README.md).

## Preserve raw input and derived meaning separately

For each of the four enemy resistance overrides, one Boolean input records raw
presence and a second quantity records the supplied value. Absence omits the
quantity. The native program selects the explicit quantity or its injected
encounter default, then emits an Enemy BASE contribution. Zero is an explicit
value. A missing or malformed presence proof cannot choose a default.

This distinction has a real downstream consumer. Pinned `CalcOffence.lua`'s
`calcResistForType` reads the explicit resistance input when choosing the maximum
resistance, independently of the accumulated BASE value. Equal effective BASE
values therefore do not prove equivalent input semantics. Retain the raw channels
for that later cap calculation; the initial contribution is not final resistance,
damage mitigation or a DPS metric.

The source-bound Import profile recognizes reviewed fresh ConfigSet syntax and
the exact admitted encounter. It decodes explicit numeric Inputs, validates numeric
Placeholder syntax without treating it as authored authority, and rejects ambiguous
or unsupported lanes. The source callback overwrites these resistance placeholders.
It neither reads cached build outputs nor imports source-evaluated defaults.
Omitting the profile preserves the previous normalization behavior.

The assumptions inventory stays Pending. Known members do not account for the
remaining inputs, choices, conditional effects or custom modifier routes. Partial
encounter and contribution coverage also remains explicit in the native package.

## Build-driven scope of the census

All five originals have one saved ConfigSet and the same 34 numeric Placeholder
keys. Their authored Input counts are 17/15/9/9/8. Original05's eight Inputs are
quest choices, but its configuration is not otherwise empty: the constructor
installs 28 non-reward Input defaults as well as the 17 reward controls.

`ConfigTab.BuildModList` dispatches the ordered callback catalog without consulting
control visibility. Numeric `count` zero falls back to its placeholder; integer,
float and `countAllowZero` retain zero. Some controls, including incoming damage,
penetration, critical values and hit time, are read directly by calculation code.
An absent callback or a hidden widget is not evidence that an input has no effect.

The complete-source witness retains saved and effective maps, ordered callback
metadata, exact Config-owned Player/Enemy modifier records and their delivery to MAIN/CALCS,
conditions, source objects, original selections and outputs. It does not replace
business methods or claim per-callback attribution from generic `Config` sources.
Its controls distinguish explicit values, missing values, zero, defaults, aliases,
duplicate/unknown input, custom blocks, boss selection and ConfigSet selection.

Cross-run evidence excludes the UI cursor's blink timestamp and unrelated
modifier-database insertion order. The observer checks the full snapshots exactly
within each run before emitting its comparison record. Every Config record must
have a distinct exact downstream match; its database depth is retained. Inputs,
defaults, contributions and numerical outputs are not rounded or normalized to
make the two JIT modes agree.

## Next dependencies after the resistance contribution

| Dependency | Existing evidence and required work |
| --- | --- |
| Enemy armour/evasion defaults | Completed contribution parity: Operations V14 reads canonical `EnemySpec.level`; injected complete level tables, means and native programs match 32 Pinnacle components from the source witness. All five unchanged requests are preserved by the rating publication. No duplicate enemy-level input or fixed-level assumption is introduced. Hit chance, physical reduction and complete configuration inventories remain separate consumers. |
| Incoming damage and defence assumptions | Damage, penetration, critical chance/bonus and hit time bypass Config modifier callbacks. Preserve their separate units and producer chains, including conversion and modifiers. Hit time is milliseconds; critical damage 30 is an extra-damage percentage. |
| Conditional state/count inputs | Preserve Player/Enemy scope, Combat/Effective conditions, counts and downstream clamps. Nearby ordinary and rare/unique counts have distinct contributions. |
| Mechanic and calculation choices | Twister element, Whirlwind stages, averaging policies, cooldown overrides and child-skill enables require explicit consumer ownership; settings cannot be dropped because the current skill is absent. |
| Complete role disposition | Account for all defaults, source-only observations and custom modifier paths before retiring configuration choices or assumptions inventories. Unknown semantics remain obligations. |

The separate occurrence-input, preparation-readiness and skill-preset usage
contracts were accepted on 2026-10-02;
Core now persists and composes its typed preference layer. The first optional
source projection carries primary-effect Boolean activation; proof of complete
configuration and usage inventories remains separate work.
This configuration work does not relax those contracts. At publication,
re-finalize every unchanged original selection
and retain all 110 queries. Record contribution parity separately from complete
native build evaluation.
