# Configuration reward selection inventory

This optional Import profile proves completeness of one configuration's reward
selection list. It binds the exact source revision, reward policy and exhaustive
generated-control census. The existing partial reward policy still converts
individual selections; this profile supplies separate authority to close a list.
It adds no game constants to Core/Engine and no new definition IDs.

The reviewed source generator uses 17 of 29 quest records: nine checkboxes and
eight option lists. The other twelve records describe weapon-set passive points
and remain outside this configuration inventory. The checked policy must account
for every generated control exactly once. Each actual decision must resolve to
an exact emitted reward or a proved None. Missing data, unknown options, malformed
rows and unmapped outcomes cannot be treated as None.

Fresh construction, saved-input loading and callback dispatch matter. Missing
checks default true, missing lists default None, and string-valued Placeholder
rows can write the input map. A source dropdown does not sanitize an unknown saved
option; its callback can parse an unlisted stat string. The full-source witness
therefore authenticates the generated controls and their actual runtime behavior.
The full-source witness passes in both JIT modes: 30 complete loads and 39 finite
callback cases per mode, covering all 81 selected reward decisions. The publication
fixture requires the passed `source_validation` record and the exact witnessed
reward recipes.

Completion preserves all emitted reward IDs, values and order, and retains the
allocator watermark. Only the configuration inventory obligation is retired.
Reviewed input rows retain explicit ChoicePreset provenance, including None;
unrelated rows retain their existing authority or the same configuration's open
choice obligation. Custom modifiers are separate contributors and remain outside
this inventory. Omitting the profile preserves historical normalization behavior.

Schema transitions first validate prior authority, then rebind only the retained
reward-policy dependency to the checked successor. The control census and source
commitment remain exact. Standalone stale profiles, stale explicit replacements
and changed-source authority reject. Numerical reward owners and global/character
rewards, configuration choices, encounter assumptions and skill usage remain
unresolved until their own dependencies are implemented and validated.
