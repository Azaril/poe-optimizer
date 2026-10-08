# Exact Skill source scaling

This packet publishes three query-backed writers on Skill `02a2` for source
buff-effect increase (`3228`), source buff-effect multiplier (`3229`) and combined
magnitude (`322a`). It uses operations V24 in the existing owned format. No new
definition, import mapping, receiver, evaluator primitive or Lua behavior is added.

Four explicitly complete-empty groups admit only requests with no matching
potential contributors. The ordinary graph census rejects nonempty inventories
before inspecting activation or values, including zero, identity and unread
effects. Partial coverage is unresolved. These groups do **not** establish that
the game has no such sources. The source Skill owner and global query registry
keep their prior Partial closures. Nonempty game origins, inherited adjustments
and support delivery remain unconverted.

The magnitude expression is `(1 + increased / 100) * more`, using typed existing
operations. Its single literal is the formula's dimensionless one; resolved
source factors are never supplied as literals. The two other writers return
their checked query results. Each generated Skill occurrence owns its own values.

Evidence reuses the authenticated observations in
`../buff-effect-recipients/source-vectors.json`, without duplicating the reports.
The 22 case/mode vectors contain 23 source invocations: 17 empty and six nonempty
controls. The unchanged Original05 observations have empty incoming domains.
The nonempty controls test arithmetic and refusal, not production source admission.
`dependencies.json` additionally pins `CalcTools.lua` against the source manifest:
its INC/MORE composition is a game calculation, while its Lua vararg/JIT dispatch
has no role in native execution. Publication checks the retained reports in both
JIT modes; normal Rust checks need only committed evidence and manifests.

`migration.json` updates the current contract to V24. Publication uses the existing
release assembler, checks an exact inverse after that contract change, preserves
all five imported builds and their 110 query rows, and leaves all complete-build
obligations intact. The joined Sniper fixture installs the unchanged published
programs and queries on two real item-prepared Offering occurrences. Test-only
synthetic members exercise nonneutral arithmetic and exact binding; they are not
included in the release.

This supplies one dependency of the existing Offering application. Joining its
activation and application to the item/passive/attack graph and publishing final
damage consumers are separate steps. The earlier application-only fixture remains
useful numerical evidence; the joined graph previously observed Offering's table
and final inputs, rather than executing that application with supplied factors.
