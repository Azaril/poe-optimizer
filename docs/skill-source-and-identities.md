# Authored skills and injected identities

Skill inputs must be represented independently of the skills the native evaluator currently
supports. Within the shared bounded XML/lexical subset, the import layer preserves the caller's
skill groups, saved sets, gems and nested selectors. A separately injected catalog provides
identity evidence. Neither stage chooses an actor/action or grants numerical mechanic support.

The [general build-input proposal](general-build-input-proposal.md) describes the broader
resolution and calculation architecture under discussion. This source/catalog boundary is
a prerequisite; it does not implement that migration. Delivery status and measured coverage
belong in the [implementation log](implementation.md).

## Source projection

`skill_source::project_xml` returns an immutable projection bound to exact decoded XML and
its SHA-256. Every Skills container and saved SkillSet remains in authored order, including
duplicates, inactive sets, legacy direct groups, unknown elements and unknown attributes.
Source ranges retain original bytes; decoded attribute values follow the pinned PoB XML
reader rather than standard XML whitespace normalization. JSON records ranges instead of
copying every opaque XML fragment.

Syntactic names and source-consumer roles are separate. The original `LoadSkill` consumes
every unnamespaced child of a Skill as a gem record, regardless of its tag. Likewise, it
consumes every child of a minion-index lookup as a map entry. These positional roles remain
visible alongside an unknown-name diagnostic. Namespaced lookalikes and descendants of
ignored containers do not acquire these roles.

Projection retains requested selectors without applying source defaults or lossy overwrite
rules. In particular, it does not resolve active sets, convert duplicate numeric set IDs
into a map, repair invalid selectors, match names, or process socket groups. Main and CALCS
selectors are separate. Legacy scalar stat-set selectors remain source evidence even where
the original loader subsequently resets their maps before consuming nested selectors.

The shared 8 MiB XML / 100,000-node limits apply. Skill projection additionally bounds
containers, sets, groups, gem occurrences, projected nodes, depth, attributes, diagnostics
and aggregate projected strings. Limits reject before an unbounded report can be built;
they are input-resource limits, not gameplay legality rules. The whole document first passes
the shared lexical gate, including opaque sections. Unsupported entities/attribute spelling
or ambiguous comment/CDATA forms can therefore prevent all projection; per-section error
isolation applies only after that gate succeeds.

## Identity catalog

The portable package's `skill_identities` section contains ordered declarations and final
constructed gem/effect identities. The catalog distinguishes external game IDs, variants,
internal gem keys, effect IDs, additional effects and additional stat-set references.
It retains source spans, duplicate declarations, construction winners, actual effect-list
order and missing references. Optional source fields preserve absence rather than inventing
false values or defaults.

The offline extractor executes authenticated original Data.lua construction and its actual
skill-module dependencies in bounded mlua hosting. Original setup code supplies generated
additional effects and display ordering. It does not replace level data with numerical
stubs to obtain identities. Catalog construction and declaration inspection are distinct:
declarations cannot alone establish the final tables after source construction.

`GameDataSnapshot::skill_identities()` provides an immutable indexed catalog. Runtime
inspection loads only this portable snapshot; it does not construct a native skill program
or start Lua. A caller-selected package changes reference lookup through the same seam.
The section's capability is explicitly `identity_only`. Metadata recognition does not prove
support compatibility, level legality, effect calculation or complete action resolution.

## Reference lookup

`skill_definitions::lookup_definitions` binds a projected source to the selected snapshot
identity and trust policy. It reports authored references **before socket-group processing**.

| Authored reference | Lookup behavior |
| --- | --- |
| Present `gemId`, including an empty string | Resolve external ID and variant. Do not fall through to `skillId`. |
| Known external ID and exact unique variant | Report the matching constructed gem. |
| Missing or unknown variant with one possible gem | Report an explicit single-candidate fallback and its reason. |
| Multiple variant candidates or colliding external/variant identities | Retain all candidates and report ambiguity. Do not invent a portable winner from Lua `pairs` order. |
| No `gemId`, present `skillId` | Match the exact effect and report every possible primary gem owner. Do not choose a hidden `gemForSkill` winner. |
| Name alone | Preserve the name and mark name resolution as not run. |
| Nested granted-effect selector | Report exact effect identity when available; leave stat-set, minion-index and actor/action interpretation unresolved. |

An unknown external identity at this stage is not a claim that the final PoB build leaves
the gem unresolved: later ProcessSocketGroup name matching can change effective state.
Names that resemble minion actions also cannot be substituted for their owning gem or
actor. The source and processed interpretations must remain separately inspectable.

Lookup reports retain container occurrence and exact set/group/record ranges. They cap
expanded matches at 65,536 and accumulated copied strings at 2 MiB. Ambiguity expansion is
bounded even for a custom catalog. Reports are evidence, not deserializable native admission
tokens; the private calculation admission boundary remains separate.

## Caller-driven inspection

```powershell
cargo run --no-default-features --locked -- inspect-build path/to/caller-build.xml
cargo run --no-default-features --locked -- inspect-build path/to/caller-build.xml --with-definitions --output runs/skills.json
cargo run --no-default-features --locked -- inspect-build path/to/caller-build.xml --data path/to/reviewed-package.json --data-sha256 REVIEWED_SHA256
```

Omitting both definition options performs source-only inspection without a game-data
package. `--with-definitions` uses the bundled snapshot; `--data` selects a supplied package
and enables lookup. Reports include actual source/data identities and explicitly mark
native admission, legality, actor resolution and calculations as not checked or not run.
Report schema 2 contains independent configuration, skill and
[item-source projections](item-source-and-loading.md). A local configuration or item
projection error does not erase preserved skills after the global document gate succeeds.
Item text/range instructions, saved equipment sets and passive-spec jewel references remain
source evidence; no Item.ParseRaw, equipment selection, item-grant assembly or allocation
check runs. Optional definition lookup still covers skills/configuration only. The corpus
runner writes schema 4, retaining older schema-1 inspection reports with item evidence
explicitly unavailable. Existing output paths are never overwritten. No production fixture
or hard-coded skill supplies the input.

## Validation contract

Independent tests execute the original XML parser, SkillsTab loading and socket-group
processing, original data construction, and relevant calculation helpers. Tests compare
pre-processing evidence separately from effective processed groups, and compare all
declarations and final catalog rows against an independently assembled reference.
Complete supplied builds remain intact; synthetic cases isolate fallback, duplicate,
legacy, ordering, namespace and missing-reference behavior.

The [breadth corpus](breadth-validation.md) must retain all saved sets and all source
occurrences. Identity coverage, native admission, realized actions, mechanic coverage and
numerical parity remain separate results. The independent numerical goldens and the pinned
source remain unchanged when adding identity metadata.
