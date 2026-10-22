# Next Sniper usage blocker after the four Firebolt fields

Read-only audit against the current a75 package, October 21. No adapter or
publication execution is claimed here. Root owns validation and checkpointing.

## Current evidence

- Package: `runs/owned-reverse-primary-cutover-01/package`, input
  `a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905`.
- `normalization.json` SHA-256
  `19abc04e1d4dd223a6bdcfab6931c437ab9e24bb5a073f036f61d4d1d72c4644`.
- Fresh selected report: `runs/owned-reverse-primary-validation-01/report-05.json`,
  SHA-256 `97de9d05133f0056266ebc156ef62e024003294937019cd2b361ed7575424e4a`.
- Fresh original draft/sidecar are under that directory's `original-05/`;
  file hashes are respectively
  `9e29da7d0b21818b5afc64f3f99de1e7571c7d05bb6cd0245480993c4d21b837`
  and `acb46799a63a484eb415c44440c56977f8d449926a7a6abc816dbb159182b781`.
- Unchanged input: `tests/fixtures/builds/breadth-20260908/build-05.xml`,
  SHA-256 `442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089`.

The selected source SkillSet is id4 (Act 3), owned preset01dd. Its usage issue is
051b, not the earlier 0507. Five selected issues remain in this baseline. Its
generated-input inventory is Complete with three exact bindings, while usage is
Pending with 38 bindings. The other selected issues are configuration01f2 and
scenario0208/0209/020a; this memo only audits the next usage blocker.

## Concrete next rows

| Saved source | Exact source occurrences | Provider and owned identity |
| --- | --- | --- |
| Sand Djinn | group208 / Gem209, `Tree:13289` | allocation037a, Gem0656, Skill0322, grant32d2 |
| Water Djinn | group226 / Gem227, `Tree:32705` | allocation0368, Gem0657, Skill032c, grant32d4 |

Both groups have literal enabled=true and includeInFullDPS=nil. Both Gems have
level1, quality0, count=nil, enabled=true, enableGlobal1=true and enableGlobal2=nil.
Sand selects child skill2 for MAIN and1 for CALCS; Water selects1 for both.
Their current origin links point to exact generated input, preset01dd and usage
051b, without a remaining configuration fallback link on those four rows.

The current usage policy has 26 declared fields per generated Djinn row, of which
25 retain Deferred destinations. The exception is groupCount, which already has
its typed usage destination. Independent count326a/326b, participation332b with
332c/332d, and reporting338e/338f destinations are already declared alongside the
relevant Deferred responsibilities and must remain.

The complete current reverse census observes one primary candidate for each
Djinn, but each construction has TWO effects: summon plus its Command. Missing
reference lists are empty and display order is ordinary. The new Firebolt
singleton predicate must therefore continue refusing these rows. Do not omit
Command, turn enableGlobal2=nil into an unread switch, or infer group selection
from a singleton that does not exist.

## Smallest useful follow-up

Audit and publish the already-supported exact generated intrinsic/reference
fields for these two live providers through the existing inventory mechanism,
while retaining all independent typed consumers. Source links and complete raw
bindings are evidence of the current path, not serialized private receipts;
focused positive/refusal tests must establish which destinations can retire.

There is also a concrete existing-code seam for missing selector accounting:

- Both actual Gems omit `statSetIndex`, `statSetIndexCalcs` and `skillMinionCalcs`,
  while their policy inventories explicitly declare all three fields.
- `skill_input_disposition::account_reference` returns `CheckedSourceReference`
  with checked `absent_fields()`; physical/manual inventory consumes those facts.
- Generated `dispositions::account` deliberately calls `into_children()` and
  drops that absence information. `intrinsic_attributes` records only present
  attributes. Simply replacing all Deferred destinations with InputDisposition
  would consequently leave those declared absent fields unproved.
- Inspect the generated reference-dispatch proof and preserve only its actually
  justified absence facts on the SAME exact live generated receipt. Do not infer
  absence from empty descendants or borrow the manual dispatcher's proof. The
  current unique reverse route and actual two-effect construction are relevant
  source observations; this is not a reason to weaken the singleton adapter.

Present intrinsic fields and the reporting projection can be audited separately
from the missing-selector facts. Group main selectors, effective count consumers
and the secondary Command switch still require their own two-effect source
proofs after that. No public/native model extension has been demonstrated by
this audit, and no selected issue is predicted to retire from partial field work.

## Current adapter/test review

The new singleton adapter keeps source/provider/output revalidation before the
unique reverse-candidate and exact construction joins. It charges its route
lookup and comparisons, does not create absent fields, and reuses the existing
independent count proof rather than adding an uncharged per-usage target scan.
The generated group selector branch is read-only and exact to one live receipt.
Native semantics and completeness are unchanged.

The seven new integration tests use compiler-produced roles and independent
catalog reproduction, compare full draft/sidecar inverses, retain typed count
consumers, distinguish absent/escaped/invalid values, and test the precise work
limit boundary. The author noticed that the Reordered synthetic catalog should
store its effect list in post-display order; correction is coordinated with root
before execution. Earlier-rejection mutations must be interpreted at their actual
stage if runtime shows no live usage obligation; do not relax Pending assertions
without establishing and preserving the entire unchanged refusal result.
