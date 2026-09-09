//! Independent observations of the original public ModParser's stateful behavior.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/mod_parser_public_source.rs"]
mod public_source;
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use mlua::{Table, Value};
use public_source::{Atom, Observation, PublicSource};
fn mods(source: &PublicSource, text: &[u8]) -> Table {
    source
        .raw(text)
        .unwrap()
        .front()
        .unwrap()
        .as_table()
        .unwrap()
        .clone()
}
fn names(table: &Table) -> Vec<String> {
    table
        .clone()
        .sequence_values::<Table>()
        .map(|row| row.unwrap().get("name").unwrap())
        .collect()
}
#[test]
fn original_doubled_mutates_shared_names_but_existing_cache_retains_earlier_results() {
    let source = PublicSource::new();
    let prior = source.observe(b"+7 to Strength and Dexterity");
    assert_eq!(
        names(&mods(&source, b"+7 to Strength and Dexterity")),
        ["Str", "Dex", "StrDex"]
    );
    let doubled = mods(&source, b"Strength and Dexterity is doubled");
    assert_eq!(names(&doubled), ["Str", "Multiplier:StrDoubled", "StrDex"]);
    let third: Table = doubled.get(3).unwrap();
    let types: Table = third.get("type").unwrap();
    let values: Table = third.get("value").unwrap();
    assert_eq!(types.get::<String>(1).unwrap(), "MORE");
    assert_eq!(types.get::<String>(2).unwrap(), "OVERRIDE");
    assert_eq!(values.get::<f64>(1).unwrap(), 100.0);
    assert_eq!(values.get::<f64>(2).unwrap(), 1.0);
    assert_eq!(source.observe(b"+7 to Strength and Dexterity"), prior);
    assert_eq!(
        names(&mods(&source, b"+8 to Strength and Dexterity")),
        ["Str", "Multiplier:StrDoubled", "StrDex"]
    );
    let fresh = PublicSource::new();
    assert_eq!(
        names(&mods(&fresh, b"+8 to Strength and Dexterity")),
        ["Str", "Dex", "StrDex"]
    );
    assert!(source.cache.raw_len() == 0); // exact string keys, not array slots
    eprintln!(
        "Original DOUBLED graph: {}",
        serde_json::to_string(&source.observe(b"Strength and Dexterity is doubled")).unwrap()
    );
}
#[test]
fn original_public_outputs_distinguish_nil_empty_and_copy_cache_results() {
    let source = PublicSource::new();
    assert!(
        source
            .source
            .load("<Items/>", false)
            .get::<bool>("ok")
            .unwrap()
    );
    assert_eq!(
        source
            .source
            .parse("Rarity: Normal\nRusted Greathelm\nQuality: 0")
            .get::<String>("baseName")
            .unwrap(),
        "Rusted Greathelm"
    );
    for text in [
        b"".as_slice(),
        b"not an actual modifier",
        b"20% increased not a stat",
        b"+2 to maximum Life",
    ] {
        let first = source.observe(text);
        assert!(
            matches!(first, Observation::Returned(_)),
            "{text:?}: {first:?}"
        );
        assert_eq!(source.observe(text), first);
        eprintln!(
            "Original public parser {text:?}: {}",
            serde_json::to_string(&first).unwrap()
        );
    }
    let original = source.observe(b"+2 to maximum Life");
    let returned = mods(&source, b"+2 to maximum Life");
    returned
        .get::<Table>(1)
        .unwrap()
        .set("name", "caller mutation")
        .unwrap();
    assert_eq!(source.observe(b"+2 to maximum Life"), original);
    let first = source.raw(b"not an actual modifier").unwrap();
    assert!(matches!(first.front(), Some(Value::Nil)));
    let Some(Value::String(extra)) = first.get(1) else {
        panic!("missing original failure remainder")
    };
    assert_eq!(extra.as_bytes().as_ref(), b"not an actual modifier ");
}
#[test]
fn output_observer_preserves_aliases_cycles_bytes_negative_zero_and_nonfinite() {
    let source = PublicSource::new();
    let values = source.source.lua.load("local shared={}; shared.self=shared; return shared, shared, -0.0, math.huge, -math.huge, 0/0, string.char(255,0,128), function() return shared end").eval().unwrap();
    let graph = source.capture(values).unwrap();
    assert_eq!(&graph.roots[..2], &[Atom::Table(0), Atom::Table(0)]);
    assert_eq!(graph.roots[2], Atom::Number((-0.0f64).to_bits()));
    assert_eq!(graph.roots[3], Atom::Number(f64::INFINITY.to_bits()));
    assert_eq!(graph.roots[4], Atom::Number(f64::NEG_INFINITY.to_bits()));
    let Atom::Number(nan) = graph.roots[5] else {
        panic!("NaN lost")
    };
    assert!(f64::from_bits(nan).is_nan());
    assert_eq!(graph.roots[6], Atom::Bytes(vec![255, 0, 128]));
    assert_eq!(
        graph.tables[0],
        [(Atom::Bytes(b"self".to_vec()), Atom::Table(0))]
    );
    assert_eq!(
        graph.callbacks[0].upvalues,
        [(b"shared".to_vec(), Atom::Table(0))]
    );
}

#[test]
fn original_public_copy_breaks_internal_shared_tag_aliases() {
    let source = PublicSource::new();
    let text = "+7 to Strength and Dexterity while dual wielding";
    let returned = mods(&source, text.as_bytes());
    assert_eq!(names(&returned), ["Str", "Dex", "StrDex"]);
    let cached: Table = source.cache.get::<Table>(text).unwrap().get(1).unwrap();
    let tag = |mods: &Table, index: i64| mods.get::<Table>(index).unwrap().get::<Table>(1).unwrap();
    assert_eq!(tag(&cached, 1).to_pointer(), tag(&cached, 2).to_pointer());
    assert_ne!(
        tag(&returned, 1).to_pointer(),
        tag(&returned, 2).to_pointer()
    );
    assert_ne!(tag(&returned, 1).to_pointer(), tag(&cached, 1).to_pointer());
    tag(&returned, 1).set("var", "caller mutation").unwrap();
    assert_eq!(
        tag(&returned, 2).get::<String>("var").unwrap(),
        "DualWielding"
    );
    assert_eq!(
        tag(&cached, 1).get::<String>("var").unwrap(),
        "DualWielding"
    );
}

#[test]
fn original_or64_pairwise_numeric_behavior_matches_cold_and_live_source_traces() {
    use poe_optimizer_engine::lua_bits::or53;
    let source = PublicSource::new();
    let function: mlua::Function = source.source.lua.globals().get("OR64").unwrap();
    assert_eq!(function.info().line_defined, Some(153));
    let mut cases = vec![];
    let boundaries = [
        0.0,
        -0.0,
        0.5,
        -0.5,
        1.5,
        -1.5,
        2.5,
        -2.5,
        3.5,
        -3.5,
        2_147_483_647.0,
        2_147_483_648.0,
        2_147_483_648.5,
        -2_147_483_648.5,
        4_294_967_295.0,
        4_294_967_295.5,
        4_294_967_296.0,
        4_294_967_297.0,
        -1.0,
        -4_294_967_295.0,
        -4_294_967_296.0,
        -4_294_967_297.0,
        4_503_599_627_370_495.0,
        4_503_599_627_370_496.0,
        9_007_199_254_740_991.0,
        9_007_199_254_740_992.0,
        1e300,
        -1e300,
        f64::MAX,
        -f64::MAX,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    for a in boundaries {
        for b in boundaries {
            cases.push((a, b));
        }
    }
    let mut state = 0xbb67ae8584caa73bu64;
    for i in 0..5000 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let a = if i % 2 == 0 {
            f64::from_bits(state)
        } else {
            (state % 0x40_0000_0000_0000) as f64 / 4.0 - 0x20_0000_0000_0000u64 as f64
        };
        let b = f64::from_bits(state.rotate_left(29));
        cases.push((a, b));
    }
    // Compare the actual numbers admitted by LuaJIT's host API. mlua's
    // lua_pushnumber canonicalizes NaN payloads before original Lua sees them.
    let identity: mlua::Function = source
        .source
        .lua
        .load("return function(a,b) return a,b end")
        .eval()
        .unwrap();
    let mut canonicalized = 0;
    for (a, b) in &mut cases {
        let (received_a, received_b): (f64, f64) = identity.call((*a, *b)).unwrap();
        canonicalized += usize::from(a.to_bits() != received_a.to_bits())
            + usize::from(b.to_bits() != received_b.to_bits());
        *a = received_a;
        *b = received_b;
    }
    eprintln!("OR64 host ingress canonicalized {canonicalized} NaN payloads");
    let mut failures = 0;
    for &(a, b) in &cases {
        let expected: f64 = function.call((a, b)).unwrap();
        let actual = or53(a, b);
        if actual.to_bits() != expected.to_bits() {
            if failures < 30 {
                eprintln!(
                    "OR64 mismatch a={a:?} b={b:?}: original={:016x} native={:016x}",
                    expected.to_bits(),
                    actual.to_bits()
                );
            }
            failures += 1;
        }
    }
    assert_eq!(failures, 0, "cold OR64 cases differ");
    let input = source.source.lua.create_table().unwrap();
    for (index, (a, b)) in cases.iter().enumerate() {
        input
            .set(
                index + 1,
                source.source.lua.create_sequence_from([*a, *b]).unwrap(),
            )
            .unwrap();
    }
    source
        .source
        .lua
        .globals()
        .set("oracle_or_cases", input)
        .unwrap();
    let observed: Table = source
        .source
        .lua
        .load(include_str!("support/mod_parser_or_warm.lua"))
        .set_name("@test-only-original-or64-warm")
        .eval()
        .unwrap();
    for row in observed
        .get::<Table>("results")
        .unwrap()
        .sequence_values::<Table>()
    {
        let row = row.unwrap();
        let index = row.get::<usize>(1).unwrap() - 1;
        let expected = row.get::<f64>(2).unwrap();
        let (a, b) = cases[index];
        assert_eq!(
            or53(a, b).to_bits(),
            expected.to_bits(),
            "warm OR64 pair {index}"
        );
    }
    assert!(
        observed
            .get::<Table>("live")
            .unwrap()
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .any(|row| row.get::<usize>("line").unwrap() == 129),
        "original or2 absent from live traces"
    );
    eprintln!(
        "Original OR64 exact cold/warm observations: {} / {}; live source traces {}",
        cases.len(),
        observed.get::<Table>("results").unwrap().raw_len(),
        observed.get::<Table>("live").unwrap().raw_len()
    );
}
#[path = "support/mod_parser_native_observer.rs"]
mod native_observer;
#[path = "support/mod_parser_scan_source.rs"]
mod scan_source;

fn compare_structural_cases(
    cases: std::collections::BTreeSet<Vec<u8>>,
    minimum_comparisons: usize,
) {
    use poe_optimizer_engine::{
        lua_pattern::MatchBudget,
        modifier_parser::{CompiledModifierParser, ParserError},
    };
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let native = CompiledModifierParser::new(snapshot.modifier_parser()).unwrap();
    let mut source = PublicSource::new();
    source.bind_builtin_symbols(
        snapshot
            .modifier_parser()
            .data()
            .callbacks
            .iter()
            .filter_map(|callback| {
                if let poe_optimizer_data::modifier_parser::ParserCallbackKind::Builtin { symbol } =
                    &callback.kind
                {
                    Some(symbol.as_str())
                } else {
                    None
                }
            }),
    );
    let mut compared = 0usize;
    let mut deferred = std::collections::BTreeMap::<String, usize>::new();
    let mut failed = vec![];

    for text in &cases {
        let actual = native.parse(text, &mut MatchBudget::default());
        // DOUBLED mutates source dictionaries even though this native checkpoint
        // defers it. Its stateful semantics have a separate sequence regression.
        if let Err(ParserError::Deferred { stage, .. }) = &actual {
            *deferred.entry((*stage).into()).or_default() += 1;
            continue;
        }
        let expected = source.observe_primitives(text);
        match actual {
            Ok(actual) => match native_observer::capture(&actual, snapshot.modifier_parser()) {
                Ok(graph) => {
                    compared += 1;
                    if expected != Observation::Returned(graph.clone()) {
                        if failed.len() < 15 {
                            eprintln!(
                                "STRUCTURAL MISMATCH {:?}\n original={}\n native={}",
                                String::from_utf8_lossy(text),
                                serde_json::to_string(&expected).unwrap(),
                                serde_json::to_string(&graph).unwrap()
                            );
                        }
                        failed.push(text);
                    }
                }
                Err(error) => panic!("native observer failed for {text:?}: {error}"),
            },
            Err(ParserError::Deferred { stage, .. }) => {
                *deferred.entry(stage.into()).or_default() += 1;
            }
            Err(ParserError::Ambiguous { dictionary, .. }) => {
                *deferred
                    .entry(format!("ambiguous {}", dictionary.source_name()))
                    .or_default() += 1;
            }
            Err(ParserError::SourceError(_)) if matches!(expected, Observation::SourceError(_)) => {
                compared += 1;
            }
            Err(error) => {
                if failed.len() < 15 {
                    eprintln!(
                        "STRUCTURAL ERROR {:?}: native={error}; original={expected:?}",
                        String::from_utf8_lossy(text)
                    );
                }
                failed.push(text);
            }
        }
    }
    eprintln!(
        "Original public structural cases {}: paired {}, deferred {:?}, mismatches {}",
        cases.len(),
        compared,
        deferred,
        failed.len()
    );
    assert!(
        compared >= minimum_comparisons,
        "native source coverage regressed: {compared} < {minimum_comparisons}"
    );
    assert!(
        failed.is_empty(),
        "{} structural parity mismatches",
        failed.len()
    );
}
#[test]
fn ordinary_forms_names_flags_tags_and_wrappers_match_original_public_parser() {
    let scan = scan_source::ScanSource::new();
    let mut cases = std::collections::BTreeSet::new();
    let generate: mlua::Function = scan
        .source
        .lua
        .load(include_str!("support/lua_pattern_witness.lua"))
        .eval()
        .unwrap();
    let names = scan.tables.get::<Table>("modNameList").unwrap();
    for row in names.pairs::<mlua::LuaString, Value>() {
        let name = row.unwrap().0.as_bytes().to_vec();
        for (prefix, suffix) in [
            (b"+7 to ".as_slice(), b"".as_slice()),
            (b"13% increased ", b""),
            (b"17% less ", b""),
            (b"+3 to ", b" while dual wielding"),
            (b"+4 to ", b" for your minions"),
        ] {
            cases.insert([prefix, &name, suffix].concat());
        }
    }
    for (table, prefix, suffix) in [
        ("modFlagList", "13% increased damage ", ""),
        ("modTagList", "+7 to strength and dexterity ", ""),
        ("preFlagList", "", " 13% increased damage"),
        ("preSkillNameList", "", "13% increased damage"),
        ("skillNameList", "13% increased damage ", ""),
    ] {
        for row in scan
            .tables
            .get::<Table>(table)
            .unwrap()
            .pairs::<mlua::LuaString, Value>()
        {
            let pattern = row.unwrap().0;
            let witness = if table == "modFlagList" {
                pattern
            } else {
                generate.call(pattern).unwrap()
            };
            cases.insert(
                [
                    prefix.as_bytes(),
                    witness.as_bytes().as_ref(),
                    suffix.as_bytes(),
                ]
                .concat(),
            );
        }
    }
    for text in [
        "",
        "unsupported words",
        "20% increased not a stat",
        "+1..5 to maximum Life",
        "Strength and Dexterity is doubled",
        "Strength is doubled",
        "Gain physical thorns damage",
        "Adds 2 to 5 Fire Damage",
        "Adds 2 to 5 Cold Damage to Attacks",
        "Adds 2 to 5 Lightning Damage to Spells",
        "Adds 2 to 5 Chaos Damage to Attacks and Spells",
        "3 to 8 physical thorns damage",
        "Gain 5 maximum Life",
        "Lose 3 maximum Mana",
        "Grants 7 maximum Life",
        "Grants 7 additional maximum Life",
        "Removes 5 maximum Life",
        "Regenerate 4 Life per second",
        "Regenerate 4% of Life per second",
        "Lose 4 Energy Shield per second",
        "Lose 4% of Mana per second",
        "7 fire damage taken per second",
        "Penetrates 12% Fire Resistance",
        "Skills cost 3 Mana",
        "Skills cost -3 Life",
        "Skills cost 3 Spirit",
        "Costs 5 Mana",
        "You have Fortify",
        "You are Onslaught",
        "+12% chance to Poison",
        "Immune to Freeze",
        "Immune to Freeze and Shock",
        "Immune to being Ignited",
        "Cannot be used manually",
        "Immune to very long first phrase and shock",
        "Immune to one and two words",
        "Immune to one and two",
        "Added Small Passive Skills also grant: AbC!",
        "Maximum Life is 12",
        "Your minions deal 13% increased damage",
        "Auras from your Skills grant 5% increased damage",
        "Nearby Enemies have 5% reduced movement speed",
    ] {
        cases.insert(text.as_bytes().to_vec());
    }
    for value in [
        "+0",
        "-0",
        "+0.0",
        "-0.0",
        "+0..0",
        "+1.5",
        "-1.5",
        "+99999999999999999999999999999999999",
    ] {
        for name in ["maximum Life", "strength and dexterity"] {
            cases.insert(format!("{value} to {name}").into_bytes());
        }
    }
    for byte in [0, 128, 192, 255] {
        cases.insert(
            [
                b"Added Small Passive Skills also grant: ".as_slice(),
                &[byte, b' ', b'A'],
            ]
            .concat(),
        );
        cases.insert([b"+7 to maximum Life ".as_slice(), &[byte]].concat());
    }
    // The scanner object is a separately constructed source module. Exercise its
    // exact scan entry as a harness guard before using its dictionary inventory.
    let _: mlua::MultiValue = scan
        .scan
        .call((
            "+7 to maximum Life",
            scan.tables.get::<Table>("modNameList").unwrap(),
            true,
        ))
        .unwrap();
    assert!(!scan.keys().is_empty());
    assert!(!scan.warm().is_empty());
    compare_structural_cases(cases, 6753);
}
#[test]
fn every_original_static_special_and_unsupported_row_is_observed() {
    let scan = scan_source::ScanSource::new();
    let generate: mlua::Function = scan
        .source
        .lua
        .load(include_str!("support/lua_pattern_witness.lua"))
        .eval()
        .unwrap();
    let mut cases = std::collections::BTreeSet::new();
    for table in ["specialModList", "unsupportedModList", "jewelFuncList"] {
        for row in scan
            .tables
            .get::<Table>(table)
            .unwrap()
            .pairs::<mlua::LuaString, Value>()
        {
            let (key, value) = row.unwrap();
            if matches!(value, Value::Function(_)) {
                continue;
            }
            let witness: mlua::LuaString = if table == "unsupportedModList" {
                key
            } else {
                generate.call(key).unwrap()
            };
            cases.insert(witness.as_bytes().to_vec());
        }
    }
    compare_structural_cases(cases, 1335);
}

#[test]
fn full_original_setup_gem_exposes_base_name_collision_for_both_legal_orders() {
    use std::collections::{BTreeMap, BTreeSet};
    let source = PublicSource::new();
    let text = runtime::verified("src/Modules/Data.lua").unwrap();
    let first = text.find("local function setupGem(gem, gemId)").unwrap();
    let end = text[first..].find("\nlocal toAddGems = { }").unwrap() + first;
    let padding = "\n".repeat(text[..first].bytes().filter(|b| *b == b'\n').count());
    let factory: mlua::Function = source
        .source
        .lua
        .load(format!(
            "return function(data)\n{padding}{}\nreturn setupGem\nend",
            &text[first..end]
        ))
        .set_name("@test-only-complete-original-setupGem")
        .eval()
        .unwrap();
    let original_data: Table = source.source.lua.globals().get("data").unwrap();
    let originals: BTreeMap<String, String> = original_data
        .get::<Table>("gemForBaseName")
        .unwrap()
        .pairs()
        .map(Result::unwrap)
        .collect();
    let mut mappings = vec![];
    let mut alternatives = BTreeMap::<String, BTreeSet<String>>::new();
    for reverse in [false, true] {
        let gems: Table = source
            .source
            .lua
            .globals()
            .get::<mlua::Function>("LoadModule")
            .unwrap()
            .call("Data/Gems")
            .unwrap();
        let mut keys = gems
            .clone()
            .pairs::<String, Value>()
            .map(|row| row.unwrap().0)
            .collect::<Vec<_>>();
        keys.sort();
        if reverse {
            keys.reverse();
        }
        assert_eq!(keys.len(), 966);
        let data = source.source.lua.create_table().unwrap();
        data.set("skills", original_data.get::<Table>("skills").unwrap())
            .unwrap();
        for name in [
            "gemForSkill",
            "gemsByGameId",
            "gemNameForModSource",
            "gemForBaseName",
        ] {
            data.set(name, source.source.lua.create_table().unwrap())
                .unwrap();
        }
        // Observe every write, including overwritten middle candidates. The
        // real setupGem writes this table but does not read from it.
        let assigned = source.source.lua.create_table().unwrap();
        let last = source.source.lua.create_table().unwrap();
        let last_copy = last.clone();
        let assigned_copy = assigned.clone();
        let meta = source.source.lua.create_table().unwrap();
        meta.set(
            "__newindex",
            source
                .source
                .lua
                .create_function(move |lua, (_, key, value): (Table, String, String)| {
                    last_copy.set(key.as_str(), value.as_str())?;
                    let values = match assigned_copy.get::<Value>(key.as_str())? {
                        Value::Table(t) => t,
                        _ => lua.create_table()?,
                    };
                    values.set(value, true)?;
                    assigned_copy.set(key, values)?;
                    Ok(())
                })
                .unwrap(),
        )
        .unwrap();
        data.get::<Table>("gemForBaseName")
            .unwrap()
            .set_metatable(Some(meta))
            .unwrap();
        let setup: mlua::Function = factory.call(data.clone()).unwrap();
        let sanitise: mlua::Function = source.source.lua.globals().get("sanitiseText").unwrap();
        for key in keys {
            let gem: Table = gems.get(key.as_str()).unwrap();
            let name: mlua::LuaString = sanitise
                .call(gem.get::<mlua::LuaString>("name").unwrap())
                .unwrap();
            gem.set("name", name).unwrap();
            setup.call::<()>((gem, key)).unwrap();
        }
        let mapping: BTreeMap<String, String> = last.pairs().map(Result::unwrap).collect();
        for row in assigned.pairs::<String, Table>() {
            let (key, values) = row.unwrap();
            alternatives
                .entry(key)
                .or_default()
                .extend(values.pairs::<String, bool>().map(|row| row.unwrap().0));
        }
        mappings.push(mapping);
    }
    assert_eq!(
        mappings[0].keys().collect::<Vec<_>>(),
        mappings[1].keys().collect::<Vec<_>>()
    );
    assert_eq!(
        mappings[0].keys().collect::<Vec<_>>(),
        originals.keys().collect::<Vec<_>>()
    );
    let differences = mappings[0]
        .iter()
        .filter(|(key, value)| Some(*value) != mappings[1].get(*key))
        .map(|(key, value)| {
            (
                key.clone(),
                BTreeSet::from([value.clone(), mappings[1][key].clone()]),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let ids = |suffixes: [&str; 2]| {
        suffixes
            .into_iter()
            .map(|s| format!("Metadata/Items/Gems/SkillGem{s}"))
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(
        differences,
        BTreeMap::from([
            (
                "lightning bolt".into(),
                ids(["LightningBolt", "UniqueBreachLightningBolt"])
            ),
            (
                "mace strike".into(),
                ids(["PlayerDefault1HMace", "PlayerDefaultMaceMace"])
            ),
            (
                "spear stab".into(),
                ids(["PlayerDefaultSpear", "PlayerDefaultSpearOffHand"])
            ),
        ])
    );
    for (key, value) in &originals {
        assert!(
            alternatives[key].contains(value),
            "unobserved source winner for {key}"
        );
    }
    let ambiguous = alternatives
        .iter()
        .filter(|(_, values)| values.len() > 1)
        .collect::<BTreeMap<_, _>>();
    eprintln!(
        "Complete original setupGem:966gems each order, {}base names, exact ambiguous sets {ambiguous:?}",
        originals.len()
    );
}

#[test]
fn every_caller_corpus_item_parser_request_is_compared_or_explicitly_deferred() {
    use sha2::{Digest, Sha256};
    let source = PublicSource::new();
    let directory = runtime::repository().join("tests/fixtures/builds/breadth-20260908");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json")).unwrap()).unwrap();
    let mut cases = std::collections::BTreeSet::new();
    let mut requests = 0usize;
    let mut items = 0usize;
    for row in index["builds"].as_array().unwrap() {
        let path = directory.join(row["xml"].as_str().unwrap());
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            row["xml_sha256"].as_str().unwrap()
        );
        let loaded = source
            .source
            .load(std::str::from_utf8(&bytes).unwrap(), false);
        assert!(
            loaded.get::<bool>("ok").unwrap(),
            "original item load failed for {}",
            path.display()
        );
        items += loaded.get::<Table>("items").unwrap().raw_len();
        for event in loaded
            .get::<Table>("events")
            .unwrap()
            .sequence_values::<Table>()
            .map(Result::unwrap)
        {
            if let Some(calls) = event.get::<Option<Table>>("calls").unwrap() {
                for call in calls.sequence_values::<Table>().map(Result::unwrap) {
                    cases.insert(
                        call.get::<mlua::LuaString>("text")
                            .unwrap()
                            .as_bytes()
                            .to_vec(),
                    );
                    requests += 1;
                }
            }
        }
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    assert_eq!(items, 116);
    assert!(requests > 500);
    eprintln!(
        "Complete original corpus: {items}items, {requests}parser requests, {}unique text inputs",
        cases.len()
    );
    compare_structural_cases(cases, 437);
}

#[test]
fn original_named_ipairs_primitive_retains_a_separate_raw_runtime_observation() {
    let mut source = PublicSource::new();
    source.bind_builtin_symbols(["ipairs"]);
    let function: mlua::Function = source.source.lua.globals().get("ipairs").unwrap();
    let values: mlua::MultiValue = function
        .call(source.source.lua.create_table().unwrap())
        .unwrap();
    let iterator = values.front().unwrap().as_function().unwrap();
    let raw = source
        .capture(mlua::MultiValue::from_vec(vec![Value::Function(function)]))
        .unwrap();
    assert_eq!(raw.callbacks[0].builtin.as_deref(), Some("ipairs"));
    assert_eq!(raw.callbacks[0].upvalues, vec![(vec![], Atom::Function(1))]);
    let iterator_graph = source
        .capture(mlua::MultiValue::from_vec(vec![Value::Function(
            iterator.clone(),
        )]))
        .unwrap();
    assert_eq!(raw.callbacks[1], iterator_graph.callbacks[0]);
    assert!(raw.callbacks[1].upvalues.is_empty());
    // Also prove the actual pointer, not merely two indistinguishable C spans.
    let (_, captured): (mlua::LuaString, mlua::Function) = unsafe {
        // SAFETY: one validated source Function, exactly one observed upvalue,
        // names copied immediately, then only the name/value results remain.
        source
            .source
            .lua
            .exec_raw(
                source
                    .source
                    .lua
                    .globals()
                    .get::<mlua::Function>("ipairs")
                    .unwrap(),
                |state| {
                    let name = mlua::ffi::lua_getupvalue(state, 1, 1);
                    mlua::ffi::lua_pushstring(state, name);
                    mlua::ffi::lua_insert(state, -2);
                    mlua::ffi::lua_remove(state, 1);
                },
            )
            .unwrap()
    };
    assert_eq!(iterator.to_pointer(), captured.to_pointer());
}

#[test]
fn original_uncached_ordinary_results_match_after_completed_source_jit_traces() {
    use poe_optimizer_engine::{lua_pattern::MatchBudget, modifier_parser::CompiledModifierParser};
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let native = CompiledModifierParser::new(snapshot.modifier_parser()).unwrap();
    let source = PublicSource::new();
    let live = source.warm();
    assert!(
        live.clone()
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .any(|row| row.get::<usize>("line").unwrap() == 6619),
        "original parseMod6619 absent from completed live traces"
    );
    let mut count = 0;
    for value in -30i32..30 {
        for text in [
            format!("{value:+} to strength and dexterity while dual wielding"),
            format!("{}% increased damage", value.abs()),
            format!(
                "Adds {} to {} Cold Damage to Attacks",
                value.abs(),
                value.abs() + 3
            ),
            format!("Grants {} Life per Enemy Hit", value.abs()),
        ] {
            source.cache.set(text.as_str(), Value::Nil).unwrap();
            let expected = source.observe_primitives(text.as_bytes());
            let actual = native
                .parse(text.as_bytes(), &mut MatchBudget::default())
                .unwrap();
            assert_eq!(
                expected,
                Observation::Returned(
                    native_observer::capture(&actual, snapshot.modifier_parser()).unwrap()
                ),
                "warm original {text}"
            );
            count += 1;
        }
    }
    eprintln!(
        "Original public uncached warm comparisons {count}; completed live source trace observations {}",
        live.raw_len()
    );
}
