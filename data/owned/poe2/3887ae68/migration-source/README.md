# Temporary final-release source capture

This is an R1 migration bootstrap, not the intended long-term authoring format.
It captures the reviewed final semantic state without replaying historical test
helpers. Replace the bootstrap with maintained declarative source as those
families migrate, and delete this temporary capture by R4. No compatibility
reader or PoB runtime dependency is introduced by these files.

`release.kdl` names every production input relative to this directory. Its
sixteen semantic constituents retain the existing JSON DTOs inside ordinary
gzip transport; the seventeenth input is the exact provenance array extracted
from the captured endpoint. The command must enforce compressed and decoded
bounds. Future builds need only tracked files, not a local predecessor or a
`tests/support::stage` function. The command generates `manifest.json` and
`release.json`; neither is a production input here.

`reference-release.json` is the byte-exact historical receipt used for independent
artifact comparison. It is not an executable predecessor. Its ordered inventory
commits all seventeen output constituents, including the derived recipe manifest.
The 227 provenance rows remain unchanged metadata, not instructions to replay
227 migrations. No concatenated endpoint or duplicate full output package is
stored here.

Captured identity:

- Input: `a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905`.
- Endpoint SHA-256: `337266997cd5ead1949288f29aad62b9b9604b2acda2b96fb0aacf0cea505ad2` (66,145,838 bytes; capture reference only).
- Reference receipt SHA-256: `46f233e58f7449f789a93e0b9d589892b4883fe40dbff672954aa6af22faa538` (58,133 bytes).
- Upstream source revision remains `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`.

## Preserved state and limits

The registry retains 13,264 active allocations. The schema contains 9,391
definitions and 3,873 slots. Rules retain 6,550 owners, 8,212 program occurrences,
18 integer tables, 59 receivers, 114 contribution queries, one effect application
and one existing-Actor binding. The Import role artifact retains 966 Gem rows
and its observed 1,436-route reverse-primary census. The five query sets retain
their exact original names and order, with 22 queries each.

There is no evaluation bundle. The recipe manifest still reports 5,827 partial
rule owners and five partial routing outputs. This capture and release
reassembly do not establish whole-build coverage or numerical parity, retire any
gap, alter selected-build issues, or make an incomplete build executable.

## Owner-qualified Life preservation

Two owners use the program ID `contribute-inherent-strength-life`; they must
remain different:

- Shared Player Actor definition `332a` has the seven-node program from
  `../life-base-domain/program.json`. It checks the inherent amount in the
  inclusive range [-137438953472, 137438953472] in Life units before contribution.
  Its `when enabled` guard still suppresses evaluation when the contribution is
  disabled. The checked program's SHA-256 is
  `f04aa1647694ef0f50018f1cd1c35ea29f5647b1de9fe155ff9a3f7c2e393f65`.
- Supplied Actor slot `001f`, declared by Skill `0012`, retains the older six-node
  program from `../inherent-life-contribution/program.json`.
- Life Stat `311a` retains the refined remaining gap
  `unproved-life-conversion-order-domain`. Both Actor owners remain Partial.

The existing `../inherent-life-contribution/inherent-life.rule` represents the
older six-node form. Do not replace the Player program with it unchanged, or
replace both owners by matching only the program ID. Any later DSL migration
must preserve exact owner membership, lazy guards, domains and coverage.

## Capture verification

All seventeen original output constituent lengths and SHA-256 hashes matched
the reference receipt. The eleven core/Import values and five query sets matched
the endpoint, and its provenance matched the receipt. Each gzip was generated
twice with identical bytes and a zero timestamp header, then decompressed and
checked against its exact original bytes. No source content was normalized or
reserialized; provenance retains its original JSON array text.

The Rust release integration test now reconstructs this source and checks every
artifact length/hash against the separate reference receipt, exact constituent
bytes, the derived manifest, provenance and input identity. The production
`poe-author release build release.kdl --output <new-directory>` command also
reproduces all seventeen constituent artifacts and the byte-exact receipt.
No local predecessor or test staging helper is needed. This validates release
reproduction, not whole-build coverage or completion of the format migration.

The Authoring corpus test also raises all 8,212 program occurrences through
bounded KDL names/packets and the formula DSL, proves graph equivalence and exact
post-assembly owner membership, and compiles the complete reconstructed package
with the existing Engine. Every non-program field and partial-coverage declaration
is preserved. This is the migration gate for replacing the captured rule JSON;
the production release still consumes that capture until maintained declarations
and `.rule` source replace it. Test-generated aliases are not production names.

Decoded hashes below identify JSON source bytes. The compressed files are capture
artifacts using standard gzip; builds consume these tracked bytes and need not
regenerate identical compressed streams on every platform.
Total captured decoded source bytes: **66145467**. Total compressed bytes: **1230522**. The sixteen semantic inputs contribute 66,097,762 decoded bytes; provenance is the remaining input.

| File under bootstrap/ | Decoded bytes | Decoded SHA-256 | Compressed bytes | Compressed SHA-256 |
|---|---:|---|---:|---|
| registry.json.gz | 3574293 | c8bb0a1d573c000da12d1b0feffe49b659cfe8abc8a8b48fd0734accdac448e7 | 87859 | 1bf78d7e20c6ed354e7b2d4f3df9577a7d9a79a8b6a270f346a991189a6cd7ff |
| schema.json.gz | 32184798 | 31a10555000d7d3a85ad71418d4845894480e57a602d003acd7c383a79b41522 | 429028 | 944c01ac3f15f46291ce1a0446423ac3c06e09e52fb48c8d3216d9fe3a5c3760 |
| rules.json.gz | 17488370 | 6ebd514b3c9e57a41a025426d9bf3c80f1fc64749fa665c9e8f82ebcf41b1366 | 310150 | a3677d489672e202045ba081e1e2cbdf2aa39b11e31059de6dfd8c6307804a29 |
| routing.json.gz | 17085 | 3ea5aa536cba1c921f1235b2b039f262f07722ce2e6cba36390ffd467dcdc508 | 1029 | 80eb1dc9ed0a3ed970c5fcd9abd4d81fd4ca89a45d18d11f2f60ae3d268f8466 |
| mapping.json.gz | 3947392 | d51820b0c5e85f26e1410b35b68d3c6579627b9422090c9789560d59f45e6075 | 94443 | dbff11bf488b52d3b20a26024db73723a907e02e33bfc5622dba68d196fa0b84 |
| roles.json.gz | 1705565 | bb33a808938d50f75e8bebed949e7c79b23047f6996afc9238cc295d5b960401 | 75646 | f97c898d523d30d2b9dc8ba6cf1659a5af922a338d940546f8cbd4f2f3a091e5 |
| normalization.json.gz | 1962666 | 19abc04e1d4dd223a6bdcfab6931c437ab9e24bb5a073f036f61d4d1d72c4644 | 69626 | c547a93f935e26110ec07701b6849a4db65a61da28a7e1af84e3372c18b68009 |
| rewards.json.gz | 28931 | 5a62fc8621e77971b32fd5d1201daa60873b5ccbabc803f3e82f239f7337d6b1 | 2963 | 9099f496b23e94835dda88aa0044a81d1297314d9f8433d7b7e118969ae194da |
| items.json.gz | 1643390 | 7b8be31f3323274e52932e39aa6ead731e35c150e49e6e025c8f3cab56fe8652 | 44753 | c0a6a33a5df4e482ab7a12de028cb3554e898533e3c0430cfed4fdb656cc96d2 |
| item-source.json.gz | 1412286 | 2dbc016ab5df8f1090bd4f51e7110cef78c4436f36bb1664ca6889054869a999 | 38839 | 0b9eb195b8a860621b48c1b7ede63659119437efab6c568289ad5ad0e0dd133b |
| queries-original-01.json.gz | 4710 | 896ddbaf72e107977ecb7e09bda39241d8d8be27c0c7d192181c21555d5fd4f7 | 376 | 481fccd81542020abbad1c94d6505aa6e60d89fcbdc4c2a852f0762f436b9865 |
| queries-original-02.json.gz | 7130 | eb2691aa77552e1729de12621b7564ce45ba78f6f97d0d880f0dfba474b007ca | 675 | bec9f17ca7766ac69f866dbec128c64c561233999cc37d64b1adf55b27c927df |
| queries-original-03.json.gz | 4710 | 896ddbaf72e107977ecb7e09bda39241d8d8be27c0c7d192181c21555d5fd4f7 | 376 | 481fccd81542020abbad1c94d6505aa6e60d89fcbdc4c2a852f0762f436b9865 |
| queries-original-04.json.gz | 4710 | 896ddbaf72e107977ecb7e09bda39241d8d8be27c0c7d192181c21555d5fd4f7 | 376 | 481fccd81542020abbad1c94d6505aa6e60d89fcbdc4c2a852f0762f436b9865 |
| queries-original-05.json.gz | 9732 | 4b520d4204aa9a4124b9a8963af1c7cc445c18218766655cca21686993f69fa0 | 717 | aea2b0d7636493f6d3e5145daad06af056e7476c3cb601fff653be8b5cee9ba6 |
| tree-normalization.json.gz | 2101994 | e63be8419cf9c49b9d33acc74526ff4e5c0b40636ce1f871b8a94a5c8fd13a44 | 51909 | a397f8d212348ec4bb96916713f4fdc98c22dc110d345cb29b857c635c66d215 |
| provenance.json.gz | 47705 | 3c38dae829466fb6b5ea75d6038b5598b2aef966512a34c263030fe55a4a7f26 | 21757 | 6e5ec0781061fecd62087978ad3767fbdebc7a31fa62b0e718f0eb1762379384 |
