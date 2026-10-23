# Attribute-choice intrinsic support domains

This packet records the reviewed source basis for the complete family of 293
attribute-choice passive nodes at Path of Building revision
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`. Every node offers all three choices:
Strength, Dexterity and Intelligence, each adding five of that attribute. The
source-domain claim concerns intrinsic support-origin creation by these owners.
It does not depend on which choices an imported build selected.

`evidence.json` references existing full archives; it does not introduce copied
Lua fragments as an alternative authority. The pinned source manifest
authenticates the executed `src/TreeData/0_5/tree.lua`, `PassiveTree.lua`,
`PassiveSpec.lua` and `ModParser.lua`. The sibling raw `tree.json` has a separate
fixed digest because the manifest inventories Lua files. The packet also pins
the retained tree catalog, exact mapping, tree normalization policy and Count
stage bindings. Those historical files establish source identities and channel
relationships; their old sealed package headers are not repaired or loaded as a
current release.

The maintained test is
`crates/poe-optimizer-authoring/tests/release/attribute_support_source.rs`.
For every raw `isAttribute` node it checks the complete 11-field record and the
complete four-field records for all three options. The options are exactly the
reviewed `+5 to Strength`, `+5 to Dexterity` and `+5 to Intelligence` rows, including
their source option identifiers. It renders only that fixed record shape and
compares the entire resulting Lua literal with its authenticated source span.
This catches additional fields, callbacks, escaped text, reordered options and
changed connections as well as changed stat text. The spans identify entries in
the root `nodes` table; numeric keys also occur in the unrelated `groups` table.
The raw identity set, executed attribute literal census and catalog attribute
set must agree exactly. The count 293 supplements these checks.

The reviewed dispatch chain includes the entire `PassiveTree` constructor,
`ProcessStats`, `ProcessNode`, `PassiveSpec.ReplaceNode`,
`SwitchAttributeNode`, saved override dispatch and override application. Every
option is processed, inherits the original node identity, replaces its modifier
rows and is processed again when selected. The additional constructor path for
nodes connected to class starts creates a condition flag. It does not create a
support candidate. The three bare numeric attribute lines use the parser's
`BASE` form and literal `Str`, `Dex` and `Int` names. The full parser source and
complete parsing function, scanner, form table and cache wrapper are pinned;
landmark assertions make the reviewed route visible without executing Lua or
building a second parser. The earlier special-modifier and jewel-function
dispatches remain included in the authenticated source review. No such dispatch
matches these three bare option lines. The raw `+5 to any Attribute` parent line
has an explicit empty special-modifier rule at `ModParser.lua:6289`; its exact
entry and early-return path are separately checked. This is intentional source
behavior before selecting an option, rather than a claim based on parser
failure. It does not make an absent required choice a known numerical input in
the maintained evaluator. A failed or partial parse is never promoted to
completeness.

Positive contrasts retain the actual `ExtraSupport` and `LinkedSupport`
constructors. External jewel transformations and replacements are separate
origins with separate obligations; the source proof does not erase them, certify
inherited candidates, promote other owner domains, or certify numerical
closure. Missing required choices remain unresolved under the existing checked
schema and evaluator. The sibling `attribute_support_domains.rs` test derives
the 293 current typed owners from the authenticated catalog and exact mappings,
checks their complete declared structure, and exercises every option through
the maintained compiled rules using the six Count channels. Production behavior
continues to come from the maintained declarations and DSL.

The source controls reject changed unselected options, added raw behavior,
reordered choices, missing or relocated node evidence, changed historical
binding commitments and an omitted parser span. No PoB checkout, Lua runtime,
publication predecessor replay or current build-specific default is required.
