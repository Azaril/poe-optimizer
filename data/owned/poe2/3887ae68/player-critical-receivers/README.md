# Player incoming-critical receivers

This packet owns final incoming-critical results on their respective Stats,
explicitly bound to the defending Player. It implements the
[accepted receiver decision](../../../../../docs/owned-actor-enemy-reads-proposal.md)
through the existing native rule engine. Shared-Actor read permissions are
unchanged; no Lua runtime, new operation or import adapter is introduced.

| Result | Stat | Meaning |
| --- | --- | --- |
| Incoming critical chance | `337c` | Percentage points after the declared Enemy flags, Player override, increases, evasion and unlucky behavior |
| Incoming extra critical damage | `337d` | Extra damage in percentage points, separate from total damage |
| Incoming critical damage factor | `337e` | Dimensionless factor using the two preceding results and the Player's reduction |

All three have explicit damage-category guards. Damage-over-time exclusions do
not manufacture otherwise unavailable results. Each program writes exactly its
own Stat on Player; dependencies retain their actual Player or Enemy identity.

The thirteen new definitions occupy `3373`–`337f`. Three are Boolean contribution
channels, six are resolved numeric dependencies, three are final results, and
one is the Player's optional chance override. The override uses the existing
checked selection contract: absent differs from present zero, agreeing sources
are accepted, and conflicting or unknown sources remain unavailable.

The resolved numeric inputs are Player incoming-chance increase (`3376`), Enemy
chance increase (`3377`), Player configured evasion (`3378`), Enemy bonus base
(`3379`), Enemy hit-filtered bonus increase (`337a`) and Player extra-damage
reduction (`337b`). The last input is already resolved by its future producer,
including the source's upper bound; the receiver must not clamp it a second time
or discard valid negative values. These six producers are not supplied here.

Configured chance and extra damage reuse published Enemy external inputs `3371`
and `3372`. Their defaults and saved-field interpretation stay with Import.
This consumer never reintroduces default values 5/30 or a parallel presence model.
The category reuses input `3251`; the damage-over-time option is `3254`.

`extension.json` declares the new typed definitions. `consumer.json` contains
three programs, three Player receivers and four contribution queries.
`dependencies.json` pins the preceding definitions, shared Actor and registry
closures. `bindings.json` names the input/output seams, while `authoring.json`
authenticates all four artifacts and the retained source-vector packet.

The source evidence is the tracked
[52-vector packet](../incoming-critical-effect/source-vectors.json), bound to
the pinned PoB source manifest and acquisition records. It contains actual
source observations and measured operands. Native tests inject those operands
into finite checked fixtures and execute these authored programs; they do not
replace the formulas with test-side Rust arithmetic. Positive override controls
are additional contract tests, not claimed new PoB observations.

This is a final-formula component, not complete incoming-critical support.
All four supplier query memberships remain Partial. Numeric dependency
producers, shared Player coverage, complete build admission and final EHP
bindings remain open. A finite test fixture's complete membership does not
promote the published inventories. The optional PoB reference remains evidence
only; runtime evaluation uses the injected native graph.

The release still has no evaluation bundle. Its future `EvaluationStagesInput`
must declare the frozen `Actor / 337f / Override` channel and the final programs'
dependencies before checked numeric selection can execute. Native component
tests provide that declaration and retain rejection when it is absent; this
publication does not invent a production stage or readiness bundle.

The focused native suite passes nine tests, including all 52 retained source
vectors (70 explicit Boolean completions), real Player/Enemy binding,
scope/type/stage refusals, lazy missing
operands, override agreement/conflicts and fresh/reused/Rayon report equality.
Unobserved Boolean source reads are checked under every finite completion rather
than converted to false. Two ordinary CLI tests authenticate the packet and
its reuse of configured inputs. Execution log:
`runs/owned-player-critical-native-cli-06.log`. Strict Clippy passes in
`runs/owned-player-critical-clippy-03.log`.

Publication and validation results are recorded in the living
[implementation plan](../../../../../docs/implementation.md). The predecessor
input is `1b7cc8d01e7b54a698240ca6fbf6f628b590b161df41ac2a909542af58c9de90`.
The explicit publication passes in 34.19s and preserves all five imports,
existing coverage and 110 queries. Its eighteen files rebuild byte-identically
at `runs/owned-player-critical-publication-02/package`, input
`99262608a1adb63a3abbe00fadd312610daa3c44df2a902b62f1d9900c56d25e`.
Selected issue counts remain 106/117/109/123/4 and complete native builds 0/5.
