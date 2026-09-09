# Upstream attribution

The ordered loading and source syntax in `src/item_loading/` translate portions of
Path of Building Community PoE2 at revision
`3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`:

- `src/Classes/Item.lua`: item syntax, variant selection, ParseRaw state transitions,
  modifier-list selection and explicit assembly boundaries.
- `src/Classes/ItemsTab.lua`: ordered item text and ModRange loading.
- `src/Modules/Common.lua`: item display-string escaping.

Game definitions and named loading policies are supplied by the separately attributed
`poe-optimizer-data` package. These translations are partial; they do not implement
all upstream item parsing, assembly or numerical calculations.

Source: https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2

The upstream application notice is retained below for these derived portions. It does
not select a license for the rest of this project. Complete upstream third-party notices
remain in the reference submodule's `LICENSE.md`.

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
