# Source build containers and calculation admission

The source model must represent arbitrary caller imports before choosing a native
calculation pipeline. It preserves authored bytes, section order, duplicate/unknown
sections and saved display settings. A structural projection does not establish build
legality, resolve an actor/action, or evaluate any mechanic. Game definitions remain
injected through the [data boundary](game-data-boundary.md).

## Three distinct stages

1. **Decode and preserve.** The import layer retains exact XML and its SHA-256. The
   immutable `build_source::RootProjection` borrows source slices and records shallow
   root/section attributes, child shape, and ordered auxiliary records. Unknown nested
   payloads remain opaque source, not partially interpreted mechanics. Namespaced
   lookalikes cannot acquire the meaning of unnamespaced PoB records.
2. **Project authored values.** `Skills` uses the separate [skill source projection](skill-source-and-identities.md),
   preserving all saved sets, groups, instances and selectors without resolving them.
   `Config` uses the separate source-preserving configuration
   model. `Calcs/Input` uses the same typed scalar reader; an invalid scalar remains
   visible with a local diagnostic in a generic root projection. No UI default,
   Placeholder migration, legacy setting migration, or saved selector is applied here.
3. **Admit an explicit calculation context.** A calculation consumer validates the
   supported fields and effects. MAIN admission is separate from the CALCS display
   context. The shared private admission result is constructed from an actual document;
   callers cannot deserialize a claimed valid projection. Core skill, item, tree and
   configuration checks still run after the root-container check.

The production CLI requires the caller's input; no fixture, class, source hash or cached
build statistics can supply a hidden default. For source-only inspection:

```powershell
cargo run --no-default-features --locked -- inspect-build path/to/caller-build.xml --output runs/new-build-inspection.json
```

The command needs neither PoB nor a game-data package. Its report binds source ranges to
exact input/XML hashes and separates configuration projection errors from other preserved
containers. It explicitly reports calculation context, native admission, mechanics and
legality as unresolved/not checked. The library exposes exact borrowed XML slices;
JSON omits repeated opaque XML payloads and retains their ranges. Existing output files
are never overwritten.

Add `--with-definitions` to look up configuration and skill references in the bundled
portable catalog, or `--data PACKAGE` to select injected definitions. This optional lookup
loads a data snapshot without native compilation or a reference runtime. Source projection,
identity recognition and game-mechanic admission remain separate stages.

## Pinned consumer semantics

These are source format and context rules, not build-specific constants. Their source is
PoB revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.

| Container | Meaning and admission consequence |
| --- | --- |
| Import | Realm, league, account/character hashes, saved import link and generated-item-text preference are workflow metadata. Loading them does not download a build. `exportParty=true` changes calculation branches and requires explicit future coverage. |
| Party | An empty container with the three known UI attributes has no imported buff payload. `ImportedBuffs` can add actor/enemy effects independently of `Import.exportParty`; all Party child payloads require effect coverage before native admission. |
| Calcs | `skill_number`, `misc_buffMode` and `showMinion` are CALCS/display choices. MAIN uses its own selected group and EFFECTIVE context. These three keys are absent from the 26 legacy Calcs-to-Config mappings. Other/legacy keys cannot be assumed inert merely because authored Config appears nonempty. `Section` rows describe layout. |
| TreeView | Search, zoom/position and stat-difference settings affect display. A paired zoom position has its own Load behavior; authored source must remain distinct from resolved UI defaults. |

Strict native admission currently accepts only the listed empty Import/Party/TreeView
shapes and typed Calcs display inputs/layout rows. It rejects unknown attributes/children,
duplicate singleton containers and duplicate Calcs input names. Boolean syntax is exact;
Calcs selection must be a positive u32-valued whole number and view coordinates finite.
These numeric syntax bounds are conservative coverage limits, not game legality. A saved
CALCS selection can differ from MAIN and is never rewritten to fit the current native
profile. Arbitrary Party effects and legacy migrations remain outside this gate.

The source loader reads the first Build first, then most sections in authored order and
Tree/legacy Spec afterward. The source saver emits Build first and uses Lua `pairs` for
other savers, so saved order is not promised. Import links can also be filtered or replaced
on save. Native exact-source exports and PoB-normalized exports are different artifacts;
source preservation cannot establish intended-versus-realized identity by itself.

## Bounds and validation contract

Projection enforces the shared 8 MiB XML / 100,000-node limits, at most 128 root sections,
4,096 auxiliary records, 128 attributes per projected element, 1,024-byte names,
64 KiB attribute values and a 2 MiB aggregate projected attribute/name budget. Unknown
opaque descendants remain subject to global limits. The PoB lexical subset rejects
forms the pinned reader would silently reinterpret. Native attribute whitespace remains
strict except for separately proven configuration Input string ranges.

Independent tests execute original XML and container Load/Save methods, MAIN/CALCS
selection blocks, legacy migration, and actual Party numeric/flag parsing. Full-build
reference tests transplant only auxiliary sections into supported complete builds and
compare against fresh PoB calculations and unchanged baselines. These derived cases
must never replace the original broad corpus or claim support for its gameplay mechanics.
Exact candidate materialization/finalist guards remain required through this boundary.
See the [living checkpoint](implementation.md#source-build-containers--validation-checkpoint)
for delivered scope and evidence, and [breadth validation](breadth-validation.md) for
remaining whole-build coverage.
