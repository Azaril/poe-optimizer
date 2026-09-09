# Upstream attribution

The bundled class, ascendancy, root, node and stat records derive from Path of Building
Community PoE2 revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`:

- `src/TreeData/0_5/tree.lua`: exact source records, identities, attributes and stats.
- `src/Classes/PassiveTree.lua` and `src/Classes/PassiveSpec.lua`: topology and automatic
  source-switch interpretation.

The injectable `data/game-data.json` package additionally transcribes the current native
profiles' skill/gem identities, weapon bases, quests, character/defence parameters,
monster tables and typed owned passive effects. Its manifest records complete SHA-256 source
hashes for `Data/Misc.lua`, `Data/QuestRewards.lua`, `Data/Gems.lua`, skill/base tables,
`ModParser.lua`, `ConfigOptions.lua`, `Modules/Data.lua` and the calculation modules.
The configuration, skill-identity and item-loading sections retain complete definition
catalogs from the corresponding pinned source tables, including item bases, modifier
tables and raw unique prototypes. Item loading policies derive from `Classes/Item.lua`,
`Classes/ItemsTab.lua`, `Modules/Data.lua` and `Modules/Common.lua`; their exact source
spans and file hashes accompany the package. Recognized definitions do not imply native
mechanic coverage. Numerical coverage is explicitly partial, and the package contains
no precomputed build output.

The item scalability section preserves original `src/Data/ModScalability.lua` records
and formatting assignments/defaults from `src/Modules/ItemTools.lua`,
`src/Modules/Data.lua` and `src/Classes/Item.lua`, with authenticated source provenance.

Source: https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2

Passive-tree/game data are (c) Grinding Gear Games. Complete source file hashes,
extractor/model fingerprint and subset/full snapshot digests accompany the data.
This attribution preserves the origin of game data and does not relicense it.

The upstream application notice below covers derived application portions. It does not
choose a license for the rest of this project. The complete upstream third-party notices
remain in the optional reference submodule's `LICENSE.md`.

Copyright (c) 2016 David Gowor

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
