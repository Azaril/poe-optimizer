# Reviewed source-only records

This packet enables bounded Import dispositions for saved PoB layout and cached
output. It does not supply native game rules, values or complete input inventories.
The active publication harness uses the current checked release and retains its
existing dispositions, changing only the `cached_build_buffs` opt-in.

The new family accepts one ordinary flat `Buffs` child of one root `Build`.
Only the saved `buffList`, `combatList` and `curseList` attributes are admitted.
Pinned `Build.Load` does not consume that child; `Build.Save` writes calculated
lists into it. The strings can be nonempty without representing configured buffs.
Actual skill, scenario and party buff settings retain their semantic obligations.
Unknown attributes, namespaces, nested content, duplicate frames and wrong parents
retain fallback. The containing `Build` and its other children are never disposed
by this family. Existing semantic links take precedence over a source-only proof.

`policy.json` opts in explicitly. False or omission authorizes no cached-output
disposition; false is omitted from serialization so an unchanged inherited policy
retains its content identity. There is one current policy and no old-version branch.
`source-facts.json` retains exact source excerpts; `authoring.json` authenticates
those artifacts, the pinned manifest and the immediate predecessor release.

Validation uses Rust:

- `poe-optimizer-import` target `owned_normalize`, filter `source_presentation`,
  checks the strict source frames, independent switches and unchanged original
  drafts/allocators.
- `owned_source_presentation_cli::presentation_authoring_is_pinned_and_has_no_native_rules`
  validates the packet without source execution.
- The ignored `presentation_publication_preserves_all_five_original_requests`
  test reconstructs a checked successor, verifies byte-identical publication,
  reimports all five originals and checks the complete source/draft/request inverse.
  Set `POE_OPTIMIZER_TEST_PRESENTATION_PRIOR` to the checked predecessor package
  and `POE_OPTIMIZER_TEST_PRESENTATION_OUTPUT` to a fresh directory.

Publication results and the current endpoint are recorded in
[the implementation plan](../../../../../docs/implementation.md).

The current publication passed in 20.79s at
`runs/owned-cached-build-buffs-01/package`, input
`b559c6c55aedfe5cf6e947ca210103c14444647b42f5f0483a32d5b7fa8c3d88`.
It disposes exactly one additional row per original and reduces Original05's
configuration-linked origins from 73 to 72. Its five selected input obligations
and all complete-build coverage gates remain unresolved. All 297 Import
normalization tests and strict workspace Clippy pass for this change.
