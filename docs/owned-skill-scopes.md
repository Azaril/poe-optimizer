# Authored skill loadout scope

Loadout scope records where a skill is available with respect to weapon loadouts.
It is independent of enabled state, global-effect selection, support applicability,
selected preset, generated providers, legality and numerical coverage. A known
`LoadoutScope::Shared` resolves this one fact; it does not activate or validate the skill.

## Injected conversion policy

`NormalizationPolicy.skill_scopes` optionally supplies a `SkillScopePolicy` with
an exact parent-group attribute name and a finite list of `SourceComponent` values
that prove shared scope. The current reviewed [policy](../data/owned/poe2/3887ae68/skill-scope-inputs/policy.json)
uses `slot_attribute: "slot"` and admits only `SourceComponent::Missing`.

The converter runs only after existing rules admit a manual group, a physical gem,
and its SkillUse role. It reads that SkillUse's own source parent, compares exact
source syntax with the injected policy, and emits `Shared` only on a match. The
skill and its parent retain occurrence provenance. No fixture identity, skill name,
selected PoB UI group, or evaluator result supplies scope authority.

The policy is optional for compatibility. Omission preserves the prior Pending
`skill-scope-not-converted` behavior and omits the field from canonical serialized
policy data. An empty admission list likewise resolves nothing. Present policies
validate the attribute name, rule count, unique source values and string bounds;
conversion charges bounded source inspection and matching work.

Namespaced parent context, ambiguous attributes, decoding failures and unmatched
values remain Pending. Missing, an explicit empty string, and a named slot are
separate source values. The current reviewed policy admits neither empty nor named
slots. Source-generated groups remain outside this authored-skill conversion; a
saved provider label is not proof of an active item or passive grant.

## Source evidence and native boundary

At pinned PoB revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`, `CalcSetup.lua`
resolves a group's slot object and compares its weapon-set property with the active
item set. A missing slot has no weapon-set restriction. `enableGlobal1` and
`enableGlobal2` instead address granted-effect-list ordinals, with separate effect
activation rules. They must not be read as weapon-set switches.

PoB also evaluates its selected main group despite some group availability flags.
That display/evaluation exception does not redefine owned scope. Native evaluation
receives only the owned scope and declared runtime data; source attribute names,
Lua execution and PoB UI selection are confined to optional import/reference work.
Future named-slot conversion must establish exact owned loadout identities before
emitting `Selected` scope. Provider availability retains a separate proof obligation.

## Original-build proof boundary

The five original XML files contain 158 source-free groups, all omitting `slot`.
Their already admitted physical SkillUses total **140**, split **9 / 63 / 9 / 13 / 46**.
The missing-slot policy is intended to close those scope fields through the same
normalizer used for arbitrary supplied builds. It does not admit every raw saved
gem or convert generated skill caches into authored skills.

Optional Rust source tests execute the complete unchanged PoB loader on all five
SHA-bound originals and directed cold/warm cases. Those tests distinguish group
and gem enabled values, global flags, missing/empty/named slots and preserved source
labels. Native tests independently exercise policy omission, exact matching,
unresolved cases and provenance. This is component evidence; complete native build
parity remains **0/5**. Scoped validation results and the resume point belong in the
living [implementation plan](implementation.md), not in the semantic contract.

Remaining gem inputs and the proposed [support receiving and activation contract](owned-support-activation.md)
retain their own proof obligations. No whole-build coverage gate or original query
is relaxed by resolving skill scope.
