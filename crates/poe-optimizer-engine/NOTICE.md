# Upstream attribution

The defence kernels in `src/defence.rs` are translated from Path of Building
Community PoE2 at revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`:

- `src/Modules/CalcDefence.lua:33-69`: hitChance, monsterHitChance,
  deflectChance, armourReductionF and armourReduction.
- `src/Modules/Common.lua:722-728`: round, without its optional decimal argument.
- `src/Modules/Data.lua:251,261`: deflection cap and armour ratio.

Source: https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2

The upstream application's notice from `LICENSE.md` is retained below. It covers
these derived portions; it does not choose a license for the rest of this project.
The original repository and complete third-party notices remain in the submodule.

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
