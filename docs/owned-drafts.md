# Owned drafts and normalization boundary

Status: structural draft/selection checkpoint implemented and locally validated. The controlling design remains the
[domain architecture](domain-architecture.md); [implementation](implementation.md)
records verified delivery. This is D1 input work, not numerical evaluation or the D2
rule compiler. Full native coverage remains **0/5** original builds.

## Purpose and dependency boundary

Import can identify a supplying item or skill occurrence while its semantic definition,
actor output or choice is still unknown. Keep that uncertainty in an owned draft rather
than copying PoB controls into the evaluator, guessing a default, or dropping the row.
Direct authoring and future UI edits use the same draft model. PoB source occurrences,
text spans, names and mapping explanations belong to a separate import sidecar.

The flow is source collection and offline policy in Import, then DraftSession in Core,
then explicit finalization to the existing OwnedEvaluationRequest. Definition binding,
actor/action resolution, legality, numerical evaluation and search follow that complete
request boundary. Native evaluation never runs a draft, an import policy or source code.

The draft types are fixed domain records and closed selector variants. They do not add
an expression language, callbacks, dynamic field maps, control flow or a second rules
interpreter. The separate future rule-representation experiment compares domain effects,
compiled kernels and compact execution on the same semantic contract.

## Representation and identity

`DraftField<T>` contains a known complete value or one pending issue with zero or more
complete candidate values. A candidate is evidence for a later explicit choice, never
an automatically selected default. `Known(null)` is explicit where absence is a meaningful
value; omission is malformed. Partially known equipment, provider, actor and action
selectors preserve known siblings when an output or slot is unresolved.

`DraftList<T>` preserves present members and separately declares whether membership is
complete. A known empty list differs from a list with no known members whose remainder
is unresolved. Ordered grant paths and query rows retain their order. Conversion returns
no complete value if any required field, member or membership closure is pending.

The session keeps independent character, equipment, allocation, skill, choice, scenario
and query presets. An explicit `EvaluationSelection` names their stable occurrence IDs;
there is no source-index pairing, implicit active selection or Cartesian preset expansion.
Items and gems remain shared records referenced by uses. Scenario and query presets have
independent identities. Query correlation IDs retain their authored order and meaning.

Every unresolved field/list has its own `DraftIssueId`, allocated from the persisted host
lineage/watermark alongside real occurrences. Validation reports its owned display path
and stable top-level owning occurrence. Nested modifier issues belong to their containing
item record; query issues belong to their query preset. Global registry closure issues have
no row owner. Source evidence may refer to these IDs through an import-only sidecar.

## Validation and explicit finalization

The immutable `DraftSession` constructor bounds entries, candidates, paths and issues;
registers actual typed occurrences; then validates known leaves and candidate values.
Candidates are not live records and cannot establish ownership or graph edges. Known
references must have the correct domain, lineage and allocation watermark. Known
ownership, duplicate assignments and containment contradictions are rejected. Shared
complete-input structural checks are reused; no fake character is constructed.

Saved scenario/query selectors may address removed historical providers, following the
existing request contract. A live ID in the wrong domain still fails. This preserves
unavailable selectors rather than retargeting them to a convenient actor or interpreting
them as zero. Definition/package compatibility remains the separate binder's task.

Finalization validates explicit preset IDs and follows the selected rows plus their backing
item/gem records. It does not automatically select an omitted skill, support, reward,
allocation or equipment provider from another preset. Selected disabled and off-loadout
rows remain required authored input; activation is not used to hide unresolved fields.

A pending global registry closure or unselected alternative does not block selecting known
records. A pending member list within a selected preset does block completion. The pending
result includes the selected issues and the entire ordered query draft. It never emits a
successful subset of requested measurements. Inactive alternatives stay in the session.

When selected facts are complete, finalization uses the existing BuildProject, compose,
ScenarioSpec, QuerySpec and OwnedEvaluationRequest constructors. Their combined-reference
checks remain authoritative. Inventory adoption and physical-copy assignment remain
separate operations; finalization does not claim stock availability or legality.

The finalization receipt binds the exact draft snapshot, explicit selection and complete
request digest. The snapshot includes order, inactive alternatives, revision and allocator
watermark. It is not a numerical-plan cache key: an inactive edit can change the snapshot,
and advancing the shared revision/watermark also changes the complete request digest.

## Persistence and hosts

Owned drafts have a separate version-1 JSON envelope, `{schema_version, draft}`, with
bounded UTF-8 input and output. Unknown/duplicate fields and unsupported versions reject.
Encoding preserves the ordered authoring state; only the existing complete constructors
canonicalize the finalized request. Deserialization cannot bypass validation or confer
prepared-plan authority.

`check-owned-draft` is a thin CLI consumer. It checks a caller-supplied draft, optionally
uses a separate explicit selection document, and can save checked draft or complete owned
request output. A pending selection cannot create a complete request file. The report
separately states that definitions are unbound, legality unchecked and calculation unrun.
Future GUI/web hosts should use these Core APIs instead of reproducing their logic.

## Import value policy and remaining work

Import's `OwnedValueCodec` converts bounded exact text according to injected token tables,
decimal grammar, whitespace policy, units and rational scale. Integer decoding checks
integrality and safe-integer bounds in decimal arithmetic before any floating conversion.
Quantities use finite floating-point values with explicit units and scale. Malformed present
values return errors; codec construction/decode does not choose precedence tiers or defaults.
No game-specific configuration key, skill name or source UI callback appears in this layer.

The next normalization slice must collect every source occurrence even when a convenience
projection rejects duplicate/noncanonical fields. Bind exact occurrences through durable
owned-ID mappings, then apply explicit versioned precedence/duplicate/role policies. The
original-five audit defines the test cases; it must not become hardcoded runtime builds.
Compile gameplay effects to a separately versioned owned package at acquisition/build time.
PoB source schema and Lua are confined to conversion tooling and the optional parity oracle.

Revisioned repair/edit/adoption is a subsequent checkpoint. Restoring a session uses its
saved allocator watermark, including deleted IDs. Pending resolution must not permit a
Known-to-Pending-to-Known identity replacement. Editing a shared item affects all its uses;
a one-use replacement or fork allocates explicit new identity and leaves historical queries
unchanged. Batch edits need exact snapshot conflict checks and atomic publication. Raw DTO
conversion is not an editor and provides none of that authority.

After normalization exercises all five originals, compile representative local-item,
conditional, support and grant effects, implement shared player/minion resolution, and
retire the replaced legacy profile/source consumers with their meaningful numerical tests.
The [retirement inventory](legacy-retirement.md) remains authoritative; this draft checkpoint
does not replace or remove an active Spark/Mace numerical evaluator.
