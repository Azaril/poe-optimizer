# Player Spirit pool

This component adds the final Player Spirit scalar and the ranged Spirit
modifier's Amulet-copy contribution. It uses the current injected rule model,
five new adjustment channels (3389–338d), two programs, one Player receiver and
nine checked queries. It does not complete an evaluation bundle.

The final consumer reads BASE, INC, MORE, three outgoing conversions, pre-scaling
Extra, post-scaling Total and an optional override. It preserves source arithmetic
order, caps the outgoing conversion sum at 100, then rounds and enforces the
ordinary minimum of one. A present override, including zero, replaces that path.
The ordinary rounding operand is checked in [-f64::MAX, 2^52]. This is the accepted
combined rounding/minimum domain; it does not certify arbitrary upstream sums.

The Amulet-copy program reuses the reviewed copy arithmetic and guards both its
effective input and final output by ±2^37. The shared effective-value producer is
unchanged. Source applicability, equipment Spirit suppression, magnitude
transform coverage and every existing modifier gap remain open. In particular,
`CannotGainSpiritFromEquipment` is an upstream obligation, not an absent flag
inferred from this build's result. Global inventories, contributor membership,
ordering and resource transformation coverage remain Partial.

The retained ranged-Spirit witness has three complete source-build controls in
each JIT mode: original 113, legacy range zero 110, legacy range one 115. The two
reports are byte-identical, with SHA-256
`9ea3244256c7ccba838c8f1317c07bb6f428d8d605907e749c7515dc67f57b29`.
Each contains six BASE records: source initialization zero, three quest rewards,
the actual Amulet amount, and the separate zero-valued Amulet bonus. The native
sum's additive identity represents initialization zero. Its five occurrence
producers preserve the three rewards and two item contributions; this is not a
claim that native and source provenance records are identical. The zero copy
remains an actual contribution and is distinct from an inactive effect.

Four source copy probes cover factors 0, 0.25, 0.5 and 1 on the original case.
The two range-control cases have no copy probes.
These are source arithmetic probes, not newly observed positive build suppliers.
Joined tests use synthetic frozen percentage inputs to replay those probes.
Positive resource transformations and override cases are explicitly synthetic
contract controls. The witness did not observe those supplier domains.

The joined fixture imports the original selected modifier's 24 rolls and selected
reward occurrences, rebinding their local IDs to its existing test lineage. It
adds the modifier to the existing physical Amulet/use and executes the published
programs. Saved request `reference-17` (metric 313c) binds to Player Stat 3166.
The finite test graph explicitly completes only its controlled membership;
restoring production Partial coverage must keep that metric unavailable.

Spirit reservation and legality are separate: the source's original unreserved
Spirit is -67. This checkpoint neither resolves that value nor makes the full
build eligible for optimization. The upstream source license is retained in
`../mana-supplier-sources/upstream-LICENSE.md.gz`.

Validation against `runs/owned-player-spirit-pool-publication-02/package`:

- Source proof, mutation controls and all-five publication passed (3 tests,
  34.84 seconds), `runs/owned-player-spirit-pool-publication-02.log`.
- Five focused Spirit tests passed in 52.31 seconds, `joined-01.log`.
- All 95 ignored joined component tests passed in 65.15 seconds, followed by
  the 11 ordinary joined tests and two ordinary Spirit proof tests. Logs are
  `runs/owned-player-spirit-pool-joined-02.log` and `ordinary-01.log` under the
  same filename prefix. Only the separate explicit replay-export test was not
  executed. Both targets passed strict Clippy in 4.73 seconds (`clippy-01.log`).
- All five import inverses, exact local IDs, 110 query rows and byte-identical
  rebuilds passed. Selected issue counts remain 106/117/109/123/4. No input issue
  or owner was closed; no complete build or evaluation bundle was added.

The resulting input identity is
`dc3371b68bc3ce5f870b6b0c44efe3dad16a0f1c37d2cb39a63492050f9d0d56`,
with compiled rules
`3afba22c8b49ce8fbb6c126bde5419e566348b9c3412fabe12ac3d20c0232e2b`.
The package contains 18 files, 64,418,739 bytes and 186 provenance entries.
It has 8,186 owner programs plus one application, leaving five program slots
under the unchanged 8,192 limit. No fresh source execution was performed.

The first compile failed on test-harness types/borrow order and was repaired.
The first source gate exposed the retained oracle's empty-object encoding for
the two absent probe lists. Their raw evidence remains unchanged; the offline
proof checks those exact empty objects and projects ordinary empty arrays.
These failures preceded any successful publication and required no rule change.
