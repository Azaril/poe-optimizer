# Shared Player intrinsic support-source domain

This packet supplies source evidence for `AuthoredAssignmentsOnly` on the
existing shared Player Actor `332a`. It changes no allocation, formulas, runtime
API or numerical coverage. The maintained release keeps all nine current Player
programs and the `shared-player-initialization-not-converted` Partial gap. The
existing Actor application targets the actual Player once, independently of
which of the eight Classes is selected.

The finite review covers the complete common initializer
(`CalcSetup.lua` 26–114), the Player intrinsic section (829–899), and the complete
environment constructor (717–2309). There are 148 literal-name modifier calls:
84 common and 64 Player-specific. Their types are 99 BASE, 24 FLAG, 20 INC and
5 MORE. The Class's explicit Str/Dex/Int loop is accounted for separately.
Condition, level and configured-resistance inputs remain inputs; they do not
authorize new support origins or supply native defaults.

The complete `ModStore`, `NewMod`, `createMod`, `ModDB` and `AddMod` bodies show
how those calls preserve the supplied modifier name and type. The proof includes
the actual `ExtraSupport` and `LinkedSupport` LIST factories and their consumers
as positive contrasts. Source completeness comes from the reviewed constructors
and admitted inputs, not from empty native lists or incomplete numerical rules.

Foreign boundaries remain explicit: configuration and party lists, cached
aggregate databases, items, allocated passives, weapon effects, group properties
and support groups retain separate source ownership. The authenticated ConfigTab
branch can parse arbitrary enabled custom-modifier text, including the legacy
text fallback, into the merged configuration list. This packet does not certify
that list or the Encounter. An empty selected custom-modifier list cannot prove
absence across its admitted input domain.

PoB also creates a default melee action when no main skill exists. The evidence
preserves that conditional call and its empty supplied support list, plus the
complete `createActiveSkill` consumer. It does not reproduce this UI fallback in
native evaluation or use it to claim skill, action or support coverage. The
constructor's supplied support list and filtered effect list remain distinct.

Ten full source archives and five retained semantic artifacts are pinned.
Nine archives are reused. The only added archive is `ModTools.lua.gz`: its
decoded upstream content is 9,130 bytes with SHA-256
`1ebd614ca55c052cb0be6dd0d16c8b9a944540a47ad50eca607c21d8913e1246`.
The local Windows checkout used CRLF; the archived LF bytes were independently
checked against the existing source manifest before compression. Other archives
retain their existing exact bytes. New evidence JSON is UTF-8 without a BOM and
uses LF line endings.
The retained PoB sources are covered by the
[upstream license](../support-structural-inventories/sources/LICENSE.md).

`player_support_source.rs` performs bounded archive and span checks, verifies the
finite call census, and retains the observed Life records and ownership laws.
It checks both MAIN and CALCS observations, all ten original/boundary/repeat
cases, exact initialized-record provenance, and the move from eight duplicated
Class bodies to one Player body. Source-generic `Multiplier:Level` overrides are
not declared impossible: the preserved observations contain none, while native
CharacterLevel has its own checked input contract. No local `runs` reports are
required and no Lua parser, evaluator or historical publisher is executed.

`player_support_domains.rs` checks the current maintained package, source
binding, component results, missing-input behavior and Original05's five Pending
requirements. Existing full-plan regression laws remain until they can execute
against the maintained readiness bundle; a component result is not a claim that
the complete build or its contributor inventory is ready. The next independent
source blocker includes Encounter custom-modifier authority.
