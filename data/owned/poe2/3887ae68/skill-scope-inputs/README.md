# Reviewed shared skill scope

This injected import policy recognizes only an absent, unnamespaced `slot`
attribute on an eligible source skill group as shared across weapon loadouts.
An empty attribute, a named equipment slot, an unknown value, malformed lexical
input, and namespaced source contexts do not match this policy. The normalizer's
existing source eligibility checks still apply; this is not a default for every
skill or a rule inferred from a display name.

Author this value into `NormalizationPolicy.skill_scopes` after validating the
predecessor tree against its original normalization policy. Publish the unchanged
recipe and import artifacts with the new normalization input through the existing
compact successor API, installing the previously checked tree content under the
new policy binding. This is explicit policy authoring; it does not claim that the
prior normalization bytes or transition history are preserved. Omitting the
optional field retains the earlier pending behavior and canonical policy bytes.

The five original fixtures contain 9, 63, 9, 13 and 46 skill occurrences whose
scope becomes Shared. The same imports retain 12 complete and 466 pending gem
parameter lists, all 110 queries, 64 admitted item modifiers and 53 display
observations. This closes a source relation only: support activation, selected
weapon loadouts, allocation access, item coverage and complete build evaluation
remain separate obligations. No source checkout or Lua execution is required.
