# Generated singleton test refusal-stage correction

The first focused run is preserved at
`runs/generated-singleton-import-focused-20261021-01.log`: 18 passed, 2 failed.
The executed child test file SHA-256 was
`5cb7a1b3c6fc86f82e5b59c5ca51eb929463b101bfecb138c46067e1a04fe445`.
The parent registration remains
`4b9b3d685dba7e14cd6f2f44d75ceb6940118c7b9c01df1dc53b8b91a0879194`.

The corrected child test file is
`crates/poe-optimizer-import/tests/support/owned_generated_catalog_accounting.rs`,
SHA-256 `84af26122149f436e1925d1315825252c05de4570d82239acd65907b6b009361`.
It has not been executed by this agent. Root owns serialized validation.

The two failed tests asserted Pending usage even where no exact generated source
was admitted. `source_shape::plain_row` rejects undecodable entity attributes;
unknown Gem/Skill identity tuples cannot match the exact generated provider
rules. The existing passing escaped-enabled regression already verifies that
this earlier refusal leaves empty Complete usage, empty generated targets,
Pending `generated-skill-inputs-not-converted`, and retained configuration
fallback. Complete empty usage is not inventory retirement or a complete build.

Only enumerated escaped-token and unknown-identity cases use the new earlier-stage
assertion. Escaped cases independently inspect all four exact raw attributes and
require decoding failure. Unknown-identity controls require their exact decoded
literal replacements. Every other negative still requires Pending usage.
Both paths retain exact whole draft, allocator and sidecar comparison, permitting
only the policy commitment difference; no Complete-or-Pending union is accepted.
Later-stage assertions now report the exact synthetic input on failure.

The separate `Reordered` fixture correction reverses its constructed display
effect list to match its already declared `[2, 1]` display order. Its refusal
expectations remain unchanged. No production source, policy or data was changed.
