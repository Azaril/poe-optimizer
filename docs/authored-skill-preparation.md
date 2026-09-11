# Authored skill preparation

This R2 stage turns imported authored skill groups into processed gem/group state through
shared native code and injected definitions. It preserves the distinction between completing
source loading and constructing effective actions, supports, actors or numerical outputs.
The [implementation log](implementation.md) records the current validation state.

## Source and ownership

Process authored groups in their original load order, including inactive alternatives and
groups later displaced by duplicate set IDs. Preserve the source loader's element/text
array semantics, nested lookup assignments and partial state on failure. Independent selected
views identify which completed groups are selected; selection must not erase preceding
load-time effects or retarget an unsupported choice.

The prepared stage owns the imported source and shares the exact compiled definition owner.
A report is serializable evidence, not a plan handle. Public native preparation consumes this
stage and retains explicit remaining dependencies; an incomplete build cannot enter the
numerical calculation loop. Existing supported numerical adapters keep their declared scope.

## Injected definitions

Skill identities and skill preparation definitions serve different purposes. The preparation
catalog supplies natural maximum levels, exact numeric level keys, requirements, colors,
hidden-skill flags, construction mappings and the requirement formula's coefficients.
Source table length and fallback-key semantics cannot be replaced by the largest level key.
Absent fields, empty tables and distinct key kinds retain their separate meanings.

In the pinned source, `LoadSkill` queries `gemForSkill` with an effect table, while
`ProcessSocketGroup` queries it with a string ID. These are different lookups. The data
contract preserves both instead of projecting them into one convenient string-key map.
Two independent original constructions produce different hash-table traversal orders.
The runtime package stores canonical semantic rows and explicit candidate sets for ambiguous
owners/variants; it never chooses a lexical winner for source-order-sensitive behavior.
Extraction evidence schema 2 separately records the actual traversal and validates its
permutations and observed winners. Unique resolved values remain deterministic.
Name-search ambiguity is a typed outcome; the arbitrary pair named in a source diagnostic
is not a portable selection contract.

Loaded definitions are immutable. Source operations that mutate referenced level rows need
per-build overlays preserving alias relationships. Triggered provider entries can clear an
effect level's cost, but authored `LoadSkill` does not read a `triggered` field. A stage must
not claim that later item/tree providers execute merely because their mutation is representable.

## Completion and diagnostics

Record each entry's processing state and typed identity outcome: resolved gem/effect, empty,
unresolved or ambiguous name, ambiguous definition construction, hidden gem, or unavailable definition. Keep source-linked errors
and the original partial group prefix. An unresolved display name can be an outcome of completed
source processing; it does not imply an effective action exists.

Retire a pre-processing prerequisite only when a native stage supplies the corresponding
result. Support application, provider construction, actor ownership, effective stat sets and
numerical semantics remain separate dependencies. Stage completion never substitutes for the
[whole-build breadth gates](real-build-rollout.md).

## Validation

Compare against the unchanged original `SkillsTab.LoadSkill`, `FindSkillGem`,
`ProcessSocketGroup` and reached `CalcTools` helpers using authentic constructed definitions.
The original Twister and Skeletal Sniper paths advance together; all five supplied sources
remain regression inputs. Reduced sources supplement those originals for errors, unusual
numeric keys, duplicate/reference handling and name-resolution behavior.

A changed valid injected definition must change preparation and agree with the same source
consumer under that data. Preserve owner/data isolation and independent concurrent preparation.
The injected schema 28 catalog includes all 966 constructed gems, 1,436 effects and
22,004 numeric level rows, with row alias identities. Source policy controls defaults,
quality bounds and requirement coefficients. The 28 earlier sections remain unchanged.

The final shared evaluator still requires full numerical comparisons, mutations, provider
removal, export/reimport and serial/Rayon agreement; authored loading alone cannot pass those.
