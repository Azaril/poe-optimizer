# Upstream attribution

The defence kernels in `src/defence.rs` and numeric modifier queries in
`src/modifiers.rs`, conditional evaluation in `src/conditions.rs`, explicit multiplier/stat scaling in `src/multipliers.rs` and `src/stats.rs`, shared actor attributes/resources in `src/actor.rs`, and the closed profiles in `src/spark.rs` and `src/mace.rs`, are translated from Path of Building
Community PoE2 at revision `3887ae68a6a6b8bb7b41d1b61998f1aa184201e4`:

- `src/Modules/CalcDefence.lua:33-69`: hitChance, monsterHitChance,
  deflectChance, armourReductionF and armourReduction.
- `src/Modules/Common.lua:722-728`: round, with and without its decimal argument.
- `src/Modules/Data.lua:251,261`: deflection cap and armour ratio.
- `src/Classes/ModDB.lua:135-294,344-390`: numeric query aggregation and conditional dispatch.
- src/Classes/ModStore.lua:201-218,261-278,302-319: query defaults and dispatch.
- src/Classes/ModStore.lua:74-81,409-415,742-799: actor lookup, explicit condition-table lookup, Condition and ActorCondition tags.
- `src/Classes/ModStore.lua:417-423,489-604,739-741`: explicit multiplier queries and numeric Multiplier/MultiplierThreshold/Limit tags.
- `src/Classes/ModStore.lua:425-468,605-652,704-722`: ordinary stat lookup and numeric PerStat/StatThreshold tags.
- `src/spark.rs::SOURCE_FILES` lists the complete versioned skill, class, character, quest, configuration and calculation sources for the closed Spark profile.
- `src/mace.rs::SOURCE_FILES` additionally records mace bases, Mace Strike/Brutality skill data, skill-stat flags and local weapon quality calculations.
- `src/character.rs` and the class/entrance profile branches derive ordinary class attribute bonuses, global defence bases, flagged damage and speed rounding from the existing profile source lists. Exact entrance stat semantics are checked against `src/Modules/ModParser.lua` and class-specific `src/TreeData/0_5/tree.lua` options; both full source hashes are retained.
- `src/Data/Global.lua:122-332`: supported flag and keyword matching semantics.
- `src/Modules/Data.lua:597-603`: MORE precision entries.
- `src/actor.rs` translates the ordered attribute/inherent-bonus stages in `CalcPerform.lua`, base/quest assembly in `CalcSetup.lua` and `ConfigOptions.lua`, maximum-resource calculation in `CalcDefence.lua`, and the global Accuracy prerequisite in `CalcOffence.lua`. Actor constants, precision and quest records are extracted from this pinned source into the injected data package.

The pure item formatter in `src/item_tools.rs` and `src/item_tools/` translates
`src/Modules/ItemTools.lua` applyValueScalar, formatValue and applyRange, with rounding
helpers from `src/Modules/Common.lua` and catalyst scaling from `src/Classes/Item.lua`.
Its tables and precision/default policies come from the injected data package.

The compiled scanner in `src/modifier_scan.rs` translates selection and capture
semantics from `src/Modules/ModParser.lua:6592-6614` at the same pinned revision.
Patterns and payloads remain caller-supplied data. The structural driver in
`src/modifier_parser.rs` and `src/modifier_parser/` translates ordinary forms,
merging, wrappers and public copy behavior from `ModParser.lua:6619-7035,7404-7424`,
`ModTools.lua:57-91` and `Common.lua:495-506`. `src/lua_bits.rs` translates the
pairwise OR operation in `Data/Global.lua:129-160`.

Source: https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2

The upstream application's notice from `LICENSE.md` is retained below. It covers
these derived portions; it does not choose a license for the rest of this project.
The original repository and complete third-party notices remain in the submodule.

Item and skill/stat data transcribed from upstream are (c) Grinding Gear Games, as
identified by the corresponding upstream data-file headers. Source identity records
are retained with each closed profile.

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


## Lua-compatible byte patterns

`src/lua_pattern.rs` translates the string pattern semantics in LuaJIT's
`src/lib_string.c`, with fixed ASCII character classes from `src/lj_char.c` and
`src/lj_char.h`. `src/lua_number.rs` follows the number reader in `src/lj_strscan.c`;
`src/lua_bits.rs` also uses LuaJIT's numeric bit-conversion semantics. The reference is the locked `luajit-src 210.7.3+1ee778a` source
used by this project's optional PoB host. The Rust module executes independently
of Lua; this attribution does not introduce a production runtime dependency.
The original copyright notice is retained below.

```text
===============================================================================
LuaJIT -- a Just-In-Time Compiler for Lua. https://luajit.org/

Copyright (C) 2005-2026 Mike Pall. All rights reserved.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.

[ MIT license: https://www.opensource.org/licenses/mit-license.php ]

===============================================================================
[ LuaJIT includes code from Lua 5.1/5.2, which has this license statement: ]

Copyright (C) 1994-2012 Lua.org, PUC-Rio.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.

===============================================================================
[ LuaJIT includes code from dlmalloc, which has this license statement: ]

This is a version (aka dlmalloc) of malloc/free/realloc written by
Doug Lea and released to the public domain, as explained at
https://creativecommons.org/licenses/publicdomain

===============================================================================
```
