# Player attribute contribution inputs

This extension connects the four existing formatted item attribute producers to the
existing Player Integer input channels. It adds four programs and six `Contribute(Add)`
effects, with no definitions, allocations, receivers, routes or native opcodes. Game
identities and target selection are injected data; there is no attribute-name dispatch
in the evaluator.

| Modifier owner | Existing formatted output | Player contribution channel |
| --- | --- | --- |
| `295c` Strength | Quantity `295b`, Count unit `295a` | Integer `1d2e` |
| `2974` Dexterity | Quantity `295b`, Count unit `295a` | Integer `1d2f` |
| `298c` Intelligence | Quantity `295b`, Count unit `295a` | Integer `1d30` |
| `29a4` all Attributes | Quantity `295b`, Count unit `295a` | All three channels |

Keys use the `def.000000000000` prefix in `poe2/owned-mechanics-v1`. An all-Attributes
modifier remains one physical occurrence: its program reads and converts the amount
once, then emits one effect per attribute. It does not create an extra All attribute
or grant Player attributes to a minion.

The program runs in the existing equipment-use context, reads `RuleEntity::Modifier`
and targets `RuleEntity::Player`. These are contribution inputs for the future staged
attribute calculation, not final Strength, Dexterity or Intelligence. The exact four
Partial owner closures remain unchanged. Item completeness, source eligibility,
equipment activation, attribute requirements and allocation access remain independent
gates; this extension grants none of that authority.

## Exact conversion boundary

Each bound `effective-amount` producer already applies display precision zero. Its
`formatted` output is the selected floor/ceiling result at quantum **1 Count**, divided
by literal **1**. Consequently every successful finite output is an integral Count,
including raw fractional components after their existing signed rounding and ordered
magnitude processing. The internal precision factor is also 1; this is not a new
interpretation of source precision.

The projection uses the existing `QuantizeInteger` opcode with quantum **1 Count** and
`Floor`. For this bound producer contract the conversion is exact for both signs. It
preserves zero and negative results, and reports IntegerOverflow outside the browser
exact Integer range. Missing input stays unresolved; wrong types and units are rejected.
No gameplay Requirement or activation condition substitutes for numeric validity.

This is not a general conversion policy for arbitrary or fractional attribute effects.
A future producer with fractional output needs its own reviewed representation and
conversion contract. Rust tests pin the integral producer tail, retain every upstream
program byte-for-byte in the successor, and compose the actual formatter output into
these projections. The component API's explicit input facts are test witnesses, not
authority to bypass build/provider resolution.

## Publication and regression checks

The authored predecessor is the native package after `skill-scope-inputs` (the local
checkpoint is `runs/owned-skill-scopes-01/package`). Use the checked native publisher
with an explicit predecessor and a fresh destination:

```text
poe-optimizer extend-owned-recipe PRIOR --extension data/owned/poe2/3887ae68/actor-attribute-inputs/extension.json --output NEW
```

The extension uses existing owned data contracts, so publication does not load PoB or
source Lua. The binding manifest records the four producers and target channels for
review and regression tests. The publisher validates the supplied prior package and
extension; the manifest is not a substitute for those checked bindings.

The Rust helper exercises all four compiled programs, the one-to-three compound case,
consecutive evaluations with changed inputs, signed and fractional-source formatter
composition, missing facts, type/unit rejection and integer overflow. It also verifies
append-only publication, idempotent replay, refusal to overwrite, and unchanged source
attribution and build fields across all five originals. Counts remain 140 Shared skill
scopes, 12 Complete and 466 Pending gem parameter lists, 110 queries, 64 admitted modifier
occurrences and 53 observed displays. No pending field is filled by this rule addition.

This is a reusable actor-input dependency in the owned native calculation chain. It does
not establish final player attributes, resources, action output or whole-build parity.
The original builds still require the remaining shared mechanics and coverage gates.
