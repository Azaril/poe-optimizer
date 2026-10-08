# Checked contributions from Actors and rewards

Status: **accepted 2026-10-08; runtime membership implemented in operations V23**.
Real game-data membership and the final Player Life consumer remain pending.
The owner approved
extending the existing graph, with exact sources and recipients, duplicate
occurrences, explicit numeric ordering and incomplete-coverage failures.

## Concrete blocker

Original05 now calculates both attribute passes from its actual Class and
selected passive choices. The approved component connects final Strength
and the five inherent Boolean flags to the existing inherent-Life producer.
That does not finish the Life pool.

Final Life must combine contributions from several existing owners. Shared
Player Actor `332a` supplies intrinsic Life; reward selections supply quest
Life; physical items supply their rolled Life. The Strength receiver derives the
inherent amount `331a` without emitting a contribution itself. The published
shared Player Actor rule reads that amount and delivers it once to canonical Life `311a`,
reusing the accepted ExistingActor ownership model. These remain distinct domain
sources. Stat receivers retain their single final-derive responsibility; no new
Receiver contribution authority is needed for this connection.

Before this implementation, the checked query contract admitted only direct
Character, Allocation, EquipmentUse and ItemModifier origins. Data's
[`ordered.rs`](../crates/poe-optimizer-data/src/owned_rules/ordered.rs)
rejected other definition/slot owners. Engine's
[`ordered.rs`](../crates/poe-optimizer-engine/src/owned_plan/compile/ordered.rs)
required `RuleOrigin::Provider` with an empty grant path. Consequently it could
not admit the actual shared Player invocation, reward provider, or supplied
Actor-slot invocation. V23 supplies this authority in the same graph; it does
not by itself supply final resource arithmetic or completeness.

Life `311a` is intentionally one canonical stat for Player and minion Actors.
The candidate census validates every concrete recipient channel for a registered
stat/kind, including channels the selected metric does not read. Therefore a
Life query also encounters Sniper intrinsic Life (Add), received minion Life
increase (Increase) and Gigantic (Multiply), on exact Actor slots `0012/001f`.
Adding only Player/reward cases would leave that candidate invalid. Creating a
separate Player-Life alias or ignoring the minion channel would conceal the gap.

## Recommended direction

Extend the **existing checked contribution graph** to these exact native origin
families. Keep one stat model, occurrence graph, compiler and worker executor.
Reuse existing `ActorKey`, `OwnedActorKey`, `ProviderKey`, reward selections,
declared slots and invocation identities; do not manufacture provider roots for
shared rules or copy shared Life back into every Class.

- **Definition and occurrence authority:** a member authenticates its actual
  owner/program/effect and permitted origin. Shared Actor rules bind exactly
  once to their existing Actor. A reward binds to its actual selected reward
  occurrence. A supplied Actor binds through its declared slot and exact
  supplying provider/grant path. Existing Actor membership authenticates the
  applicability declaration; reward membership authenticates the selected reward
  definition. A supplied Actor's key stores its parent provider, so follow the
  validated grant relation rather than compare paths by raw equality or a fixed
  depth. A definition or display name alone is
  insufficient. This does not implicitly admit every grant, support, application
  or socket origin.
- **Recipient authority:** preserve the actual contribution destination and
  original read context. A Player contribution does not inherit into minions;
  Gigantic stays on its eligible minion recipient. Validate membership separately
  for each concrete recipient while preserving the candidate-wide census.
- **Multiplicity and ordering:** retain distinct occurrences. Boolean `Any`
  remains unordered. Numeric folds require explicit owned grouping/order or an
  exact-domain proof, with ambiguous semantic ties rejected. Opaque IDs, source
  traversal order, thread order or deduplication by definition are not numerical
  semantics. Distinct Actors already have separate contribution keys; their IDs
  need not order each other's reductions. Current reward selections can repeat
  a definition, so their numeric ties must reject unless an authored domain law
  covers that multiplicity. Supporting an origin does not establish every
  possible grouping law.
- **Coverage:** account for inactive, zero-valued and late-discovered effects as
  well as active ones. Missing owners, unknown recipients, incomplete discovery
  or unsupported origins cannot become neutral values. A bounded first packet
  may reject unsupported candidates, but must not discard their contributions.
- **Execution:** resolve membership and identities during checked plan binding.
  Immutable indices and worker-local scratch keep evaluation native and parallel.
  Relevant source/recipient/membership changes invalidate plan identity. Rebuild
  current artifacts instead of preserving old development-format branches.

This is distinct from the separately accepted
[exact Skill query proposal](owned-skill-contribution-queries-proposal.md): that
also extends query *read* scope to Skills for Offering scaling. The two should
share membership machinery, but neither decision silently authorizes unrelated
Actor-to-Enemy reads or support-source composition.

## Alternatives and first delivery

Extending the existing graph requires more validation work now, but removes the
structural obstacle for Life and later Mana, defences and other Actor resources.
A separate resource aggregator would duplicate coverage, occurrence and ordering
rules. Using unchecked contribution reads would bypass the completeness proof.
Deferring final Life avoids that public change now but leaves a blocker for the
first complete build.

The minimal declarations are `ExistingActor { application }`, `Reward` and
`SuppliedActor { slots }`. Their implementation is shared by cold binding and
support-suffix validation, with effect-plan domain20. The published package
remains V22 pending game-data adoption. Validation covers exact source/recipient
identity, multiple minions
and providers, independent rewards, duplicates, partial/unavailable sources,
inactive/late effects, ordering ties, cache identity and fresh/reused/Rayon
equivalence. The [runtime checkpoint](implementation.md#latest-runtime-checkpoint-checked-actor-and-reward-contributions)
records focused regressions and the actual joined Life reduction probes. Those
probes preserve canonical `311a` and the real shared Player owner, but do not
publish a final-Life receiver or certify complete game membership.

The separate contributor checkpoint joins the actual imported Life-bearing items
and reward selections in the existing finite Sniper fixture. It adds to the
Crown and Solar Amulet preparation but is not complete equipment coverage.
Original05's retained
source evidence includes a Tattered Robe, Rope Cuffs, two uses of the Sapphire
Ring record, Fine Belt and two quest rewards. Authenticate these through import
and independent source evidence; do not paste the observed subtotal/final pool
into a producer. Origin admission does not prove the final-Life formula or the
absence of other conversion, override or conditional families. Full owner,
receiving, input and metric coverage remain required before declaring the build
complete. The [Player consumer witness](implementation.md#player-life-reference-checkpoint)
now passes eleven controls through 66 fresh loads. It captures original consumer
locals in fixed JIT-off acquisitions and requires exact uninstrumented reference
agreement across both JIT modes. Original05 has base 1257, Increase 5 and final
Life 1320; the observed ExtraLife/Total/post-clamp conversion are zero, and
override/Chaos Inoculation are absent. That is finite source evidence, not proof
of globally empty supplier families. Positive Extra/Total/conversion/override
suppliers remain unexercised. The actual rounding operand is now observed and
replayed through production native Round and Maximum for 1,584 invocations.
The [rounding audit](legacy-retirement.md#player-life-rounding-bound-2026-10-08)
proves combined-operation equivalence for finite operands at or below 2^52;
it does not prove that the actual contributor/input domain stays within that
bound, or validate the preceding arithmetic. The raw conversion Sum remains
unobserved. These arithmetic and coverage requirements remain separate from the
accepted origin-authority implementation.
