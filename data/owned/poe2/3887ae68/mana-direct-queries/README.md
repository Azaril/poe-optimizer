# Direct checked Mana queries

The final Player Mana consumer now reads the five existing checked adjustment
queries directly. This removes five computed collector programs and their
Player receivers for `3357`–`335b`. Their typed Stat definitions remain as
contribution channels, with the exact same units, query groups, membership,
ordering and empty identities. The historical guarded Complete-empty queries
are preserved; this cleanup establishes no new supplier coverage.

The only final-consumer changes are five read-source substitutions. Every
expression, rounding-domain guard, override selection and final effect remains
identical. Explicit zero overrides still take precedence, unavailable demanded
inputs stay unavailable, and the ordinary calculation is still lazy when an
override supplies the result. Direct queries retain their contribution freeze
requirements. There is no runtime compatibility branch or new engine operation.

The predecessor is the authenticated Player Life release:
`aedc17306523ec3c513f6e04c2d1937774705585b9c33c9c1cc9006b0c6452a1`.
Authoring verifies that the five collectors are unconditional one-query
pass-throughs, that only the final Mana consumer reads their computed values,
and that no routing, imported query, metric or support-source contract depends
on the removed producers. A later unexpected rule or application reader refuses
the component proof. Definitions, allocator state and global coverage remain
unchanged.

The existing standalone release assembler applies this bounded rewrite.
Restoring only the five collectors/receivers, the five prior read sources and
the provenance/release label must reproduce the entire predecessor input.
The all-five publication gate checks all 110 saved query rows, exact imported
local IDs and unchanged selected issue counts, followed by a byte-identical
rebuild. No schema rebinding or importer change is needed.

The old `mana-pool` and `mana-pool-adjustments` packets remain historical
publication and source-evidence inputs. Current native fixtures install only
the retained channels, queries and freezes, then execute this direct-query
consumer. No obsolete collector graph is installed or executed. Existing
original-source vectors, level/Intelligence/reward changes, fractional synthetic transfers, overrides,
missing inputs, numeric-domain boundaries and reused/Rayon worker checks apply
to the rewritten consumer.

Run the ordinary `owned_mana_direct_queries` and `owned_intelligence_mana`
targets. Publication additionally sets
`POE_OPTIMIZER_TEST_MANA_DIRECT_QUERIES_PRIOR` and a fresh
`POE_OPTIMIZER_TEST_MANA_DIRECT_QUERIES_OUTPUT`, then runs
`publish_direct_mana_queries_preserving_all_five_originals` with
`--ignored --exact`.

Validation passed: 21 Mana tests and three direct-query tests, plus strict Clippy
for both targets. Six explicit historical/publication tests remain ignored in
the ordinary run. The all-five publication and byte-identical rebuild passed
separately at `runs/owned-mana-direct-queries-publication-01`.
Its input is `c0bccf10cee8a70027e1bcfd923961dfd26766d56cb31bff419833cbe3244dfc`.
All 110 saved queries and exact local IDs remain unchanged; selected issue counts
remain 106/117/109/123/4. The catalogue uses 8,185 programs of the unchanged 8,192
limit and totals 64,382,445 artifact bytes, 5,592 fewer than its predecessor.

Logs: `runs/owned-mana-direct-queries-tests-03.log`,
`runs/owned-mana-direct-queries-clippy-01.log`, and
`runs/owned-mana-direct-queries-publication-01.log`.
No compiler, storage or evaluation limits change. No complete Mana supplier
census, evaluation bundle or complete native build is claimed.
