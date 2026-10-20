# Campaign progression input

This packet imports the saved count of unique campaign bosses defeated into one
typed progression Reward selected by the existing Choice preset. Its owned data
contains no source-language execution, build fingerprint, current-enemy input or
character-specific default. The reviewed source field is `configBossFaceBroken`;
the native fact is an integer number of defeated campaign bosses.

The original five committed imports contain Placeholder values **55, 57, 57, 56,
56**. These are source evidence and test cases, never production fallbacks.
`source-evidence.json` authenticates their files and the complete pinned source
bodies. No new PoB execution or numerical oracle vector is claimed here.

## Source selection and domain

The numeric reward adapter uses the explicit count reader contract: nonzero Input
first, then nonzero Placeholder. Both present lanes must decode and fit the slot,
including a losing lane. Duplicate lanes, malformed values, unsupported domains,
missing values and all-zero selections remain unresolved. The source control has
no constructor default. This differs from other source readers that retain a
zero Placeholder; their selection policies are not shared implicitly.

The required Reward parameter admits integers from zero through 2^53 - 1. This is
the owned exact-integer representation range, not a game progression cap. Source
loading is more permissive: it applies `tonumber` and does not itself enforce an
integer/nonnegative domain. Negative, fractional and overflowing saved controls
are deliberately outside this owned count adapter's supported domain. Neither
the callback nor the observed five builds proves a maximum feasible game count.
Any later gameplay progression cap needs separate evidence and rules.

Native authors may explicitly supply zero. A zero skipped by this source callback
does not establish an authored native zero, and a missing progression source is
not a count of zero.

## Ownership and post-composition authority

The schema-only extension allocates Reward `33b4`, required parameter `33b5`,
canonical fact Stat `33b6`, and authority-count Stat `33b7`. The producer reads the
Reward's own raw parameter, emits the fact as Override, and contributes exactly
one authority. The checked fact query requires agreeing values without an empty
default. The separate authority query uses exact integer addition over exact
Reward occurrences. After its channel is frozen, a Reward-owned Requirement
demands exactly one authority.

One Reward occurrence referenced by Character and Choice is deduplicated by
ordinary composition. Distinct equal or conflicting occurrences violate the
authority Requirement; agreeing numeric values alone cannot establish uniqueness.
Partial membership remains unavailable. The assessment reports these requirements
alongside any calculable metrics; it is not a proof of complete game legality.
A consumer requiring the progression fact must still reject absent selection.
There is no requirement invocation if no Reward is selected.

`consumer.json` contains the new owner, its two programs, two contribution queries
and reviewed support-source domain. Exact native query membership concerns only
these owned fact/authority producers. It is not a certificate that every possible
PoB configuration responsibility or every game's progression mechanic is ported.
No existing owner's closure changes. In particular this packet supplies no
Facebreaker damage calculation, complete item supplier coverage, optimizer
admission or complete-build evaluation.

## Publication and validation

Publication extends the schema, then installs consumer owners and queries together
before validating the final package. The reward mapping, projection, two source
inventory lanes and two accounting rules are installed together. `bindings.json`
provides an isolated checked prepare/execute schedule for the native fixture;
this component publication does not create an evaluation group for the whole
release. The predecessor is input
`5b611f43a0eedbf556799bcbd2c295a85d58d3264a7306c24602fd4d41e6e7b6`.

The source test authenticates original bytes, source spans, saved fields and typed
packet contracts. The native test executes the packet unchanged in a finite
synthetic world and covers original values, representable boundaries, duplicate
authority, absent and unknown membership, and fresh/reused/parallel scratch.
Existing generic Import tests cover malformed losing lanes, duplicate lanes,
exact parameter ownership and no-default behavior. The publication fixture
reimports all five builds and checks source-to-owned correspondence across the
new Reward's local-ID allocation, retaining unrelated fields and query targets.
The tests are implemented separately; test execution and publication outcomes
belong in the living implementation checkpoint, not in this source review.
