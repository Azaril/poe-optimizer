# Amulet Minion-level copy fragments

This packet adds the separate Amulet copy of an already resolved ordinary
Minion-level item modifier. It reuses Modifier `30ca`, effective Count `295b`,
and Player/Add Minion-level channel `30ab`. The original direct contribution
remains unchanged and distinct from the copy, including when the copy is zero.
No new item text family or raw item parameter is introduced.

The copy runs once per exact modifier occurrence and EquipmentUse. Its required
equipment eligibility comes from Boolean channel `32e3`. Its required Player Stat
`32e4` is the **pre-Amulet percent snapshot**, with percent unit `0002`. The
program divides that percent by 100, then preserves the effective value when the
factor is exactly one or the canonical unscalable flag is true; otherwise it
floors `effective × factor` in Count units. It contributes the result separately
to `30ab` only when the equipment is eligible. A missing snapshot is not zero.

The snapshot Stat and its contribution stream are separate value kinds. Mystic
Attunement, source node 7068 / owned Passive `1b09`, supplies the actual 25-percent
contribution. This packet does not supply the snapshot aggregation program or
certify all incoming contributors. Its finite native fixture may explicitly
aggregate a closed input set or provide a scalar boundary. The copy cannot feed
its own factor: source evaluation captures the percent before iterating the
Amulet's active modifier list. Other three Mystic Attunement outputs remain
unconverted, and its owner retains its original Partial closure.

The six template eligibility programs reflect independently reviewed immutable
source types. Lapis `1f45` and Solar `2343` are eligible. Their legal equipment
slot inventories become Complete with only Amulet `006a`; their socket
destinations become Complete empty. Iron Crown `1f1c` and Kamasan Tiara `1f37`
are ineligible and similarly gain complete Helmet-only `0066` placement and empty
socket destinations. Owned socket declarations on the helmets are preserved:
an item's installed augment slots are different from destinations into which
that item itself can be placed.

Rattling Sceptre `1fe3` and Sacred Focus `22f5` are ineligible because
their proved source type is not Amulet. Their conditional weapon placement
inventories remain Partial. False eligibility does not assert complete legal
placement or waive build validation. True eligibility depends on the reviewed
Amulet-only placement; an Amulet placed in another slot cannot use the fragment
as a substitute for schema validation.

There are two new Stats (`32e3..32e4`), four narrowly replaced placement
descriptors, and eight appended ordinary rule programs. All previous program
bodies, declarations and owner closures remain intact. Every affected owner
remains Partial. The canonical Minion-level parameter inventory and all source
admission rules are unchanged; in particular, the current admitted unscalable
field is false. A native true-flag control tests the finite branch and does not
authorize importing unsupported source annotations.

The selected Original05 Iron Crown and Solar Amulet already have complete
physical inputs. This numerical fragment does not retire one of that build's
eleven input issues. Full contributor membership, snapshot aggregation,
remaining item scaling, complete owner inventories and whole-build evaluation
remain open. No evaluation bundle is added, and native build completion remains
0/5. Placement and source-copy proof does not select a canonical parity lifecycle.

`migration.json` contains owned schema and rule data, using the existing V3
migration with schema V5 and operations V18. `dependencies.json` copies exact
predecessor descriptors. Other JSON files are authoring and verification receipts,
not source formats interpreted by the native evaluator. The predecessor is
`runs/owned-ice-nova-source-inputs-01/package`, input
`35bffde445888c725e153f42b43fe1f30330bd7216db2bc7ee780b9ab2a54fd3`.

The packet reuses passing original source observations. The Minion witness
contains two independent retained values, 1 and 3, copied at factors 0, 0.25, 0.5 and 1;
its unchanged original outputs retain distinct Crown, Amulet and Amulet-copy
records. Each JIT report is authenticated separately. Their substantive
observations agree, while references to older mode-specific evidence have
different paths and hashes. The newer property witness has byte-identical JIT
reports and separately observes direct2+copy 0 and direct2+copy1 for its Spell
controls. Those control item lines are evidence of the copy mechanism, not new
Spell-family import authority. Bounded pointer/value selections retain exact
observations from both witnesses and the independent static placement/passive
review. No source runtime was rerun to author this packet.

Publication and native validation pass: one authoring test, the full publication
gate and nine native tests. The checked package is
`runs/owned-amulet-level-copy-02/package`, input
`f604070539924fd7961becce302d0db5ae49d683a88bdb73c57ed59e0c63b809`.
It contains 18 files / 60,790,891 bytes / 82 provenance rows; all five selected issue
counts remain 113/116/108/121/11. Native tests also prove that a known non-Amulet
needs no Amulet snapshot. The optional source proof reconstructs each exact base
block (header through first unindented closing brace, no trailing LF) and the
retained tree-node excerpt (including its final LF) from normalized pinned files. The publication gate preserves
all five original normalized graphs, sidecars and saved selections, all 110
queries, every unrelated schema/program row and the predecessor's files, then
rebuilds the result byte-for-byte. Use a new, nonexistent output directory:

```powershell
$env:POE_OPTIMIZER_TEST_AMULET_LEVEL_COPY_PRIOR = 'runs/owned-ice-nova-source-inputs-01/package'
$env:POE_OPTIMIZER_TEST_AMULET_LEVEL_COPY_OUTPUT = 'runs/owned-amulet-level-copy-replay'
cargo test --test owned_amulet_level_copy_cli -- --include-ignored --test-threads=1
```
