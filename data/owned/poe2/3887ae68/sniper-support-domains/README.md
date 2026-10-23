# Sniper intrinsic support-source domains

This packet records a narrow source review at PoB revision
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. It supports eight
`AuthoredAssignmentsOnly` declarations in the maintained release. This means
these owners do not themselves introduce additional support origins within the
reviewed domain. It does not mean their incoming support candidate list is
empty, that every candidate applies, or that the composed build's support-source
census is complete.

`evidence.json` authenticates eight existing full source archives, exact reviewed
spans, the pinned upstream manifest, and four existing binding/catalog artifacts.
No Lua code is executed by this packet. The Rust regression tests in
`crates/poe-optimizer-authoring/tests/release/sniper_support_domains.rs` verify
those commitments and compare the independently specified typed subjects with
the maintained release. Changed source bytes require another review; substring
checks are landmarks within authenticated bodies, not a new Lua parser or a
general absence classifier.

The declarations are ordered as follows. All addresses use the
`poe2 / owned-mechanics-v1` namespace; slot addresses include the stated owner.

| Subject | Owned address | Source meaning |
| --- | --- | --- |
| Skill | `def.0000000000000012` | `SummonSkeletalSnipersPlayer` |
| Actor slot owned by Skill `…0012` | `def.000000000000001f` | Supplied Sniper population |
| Actor | `def.0000000000003091` | `RaisedSkeletonSniper` |
| Skill | `def.0000000000000021` | `MinionMeleeBow` |
| Action output owned by Skill `…0021` | `def.0000000000000022` | Basic attack output |
| Skill | `def.0000000000000024` | `GasShotSkeletonSniperMinion` |
| Action output owned by Skill `…0024` | `def.0000000000000025` | Gas arrow output |
| Action output owned by Skill `…0012` | `def.0000000000000015` | Summoner output |

The three Skill identities are checked against exact Import mappings and the
retained identity catalog. The Actor, population and child-output bindings are
checked against the independently retained `actor-ability-supply/bindings.json`.
Every subject must resolve to the same typed address and declaration owner in
the actual current schema. No IDs or schema entries are allocated here.

The reviewed source boundary is:

- The full Summon, basic attack and Gas Arrow definitions and the full
  `RaisedSkeletonSniper` definition describe the intrinsic chain. The Actor's
  intrinsic skill list contains the two reviewed child skills, and its intrinsic
  modifier list contains only comments.
- The shared `SkillStatMap` is included because the skill loader installs a lazy
  lookup into it. A finite review covers all 33 distinct quality,
  alternate-quality, constant and ordinary stat keys across the three skills'
  complete stat sets. Twelve have pinned mapping bodies; 21 have no entry in
  the authenticated literal table. The selected definitions have no local
  `statMap`, `baseMods`, `qualityMods` or `levelMods` override. The mapped bodies
  and Data's three modifier constructors cannot introduce `ExtraSupport`,
  `LinkedSupport` or `ExtraMinionSkill` from these keys. An absent mapping proves
  absence only along that source route; it does not establish numerical
  completeness or a zero value for the underlying game stat.
- `createActiveSkill` stores the incoming `supportList` and builds a separate
  `effectList` by testing candidates. The latter also contains the active effect;
  the two lists are not equivalent.
- `createMinionSkills` passes `activeSkill.supportList` directly to each child's
  `createActiveSkill` call. Nonempty lists, their existing source objects and
  their order are preserved at this boundary. Existing native end-to-end laws
  in `owned_support_inherited_delivery.rs`, including
  `inherited_duplicate_positions_survive_without_reselection_and_deliver_twice`,
  separately exercise nonempty inherited candidates, repeated positions and
  exact origins. This packet does not replace those laws with a copied Lua
  calculation or a synthetic list-copy implementation.
- `CalcSetup` constructs candidates before that call. Its `ExtraSupport` and
  `LinkedSupport` branches, together with positive factories in `ModParser`,
  demonstrate that external suppliers are real and remain independent proof
  obligations. The `ExtraMinionSkill` branch can also add descendants beyond the
  Actor's intrinsic two skills; this packet does not close those suppliers or
  arbitrary descendant membership.

The regression controls reject missing or relocated spans, changed source pins,
and changes that replace the inherited parent list with an empty list or a
cross-group list. Full source hashes cover changes outside the displayed
landmarks as well.

Numerical coverage remains partial for the seven previously partial owners. The
summoner output was already complete with no programs and remains so. The Gem's
`source-command-supply-unresolved` gap remains: the retained catalog references
`CommandSkeletalSniperPlayer`, but that effect is missing from runtime skill
construction. The separately reviewed export classifies it as an active effect,
not a support; that does not construct or validate its missing gameplay.

The release still has no executable evaluation bundle. Adding these eight rows
raises the source-domain inventory to 994 declarations: 965 known domains and
29 unmapped domains. It supplies no numerical rule, readiness classification,
support eligibility default, complete-build certificate, or result-parity claim.
