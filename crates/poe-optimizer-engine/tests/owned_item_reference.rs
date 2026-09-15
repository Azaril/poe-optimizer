//! Optional original-source item component evidence. Run explicitly with:
//! cargo test -p poe-optimizer-engine --test owned_item_reference -- --ignored --nocapture
//! No Rust Spark/Mace/profile evaluator supplies expected values.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, Table};
use roxmltree::{Document, Node};
use serde_json::json;
#[path = "support/owned_item_reference.rs"]
mod owned_item_reference;
use owned_item_reference::*;
#[path = "support/owned_item_rule_fixture.rs"]
mod owned_item_rule_fixture;
const ORIGINAL_02: &str =
    include_str!("../../../tests/fixtures/builds/breadth-20260908/build-02.xml");
const ORIGINAL_05: &str =
    include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
const HASH_02: &str = "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631";
const HASH_05: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
fn element<'a, 'input>(doc: &'a Document<'input>, name: &str) -> Node<'a, 'input> {
    doc.descendants().find(|n| n.has_tag_name(name)).unwrap()
}
fn identified<'a, 'input>(doc: &'a Document<'input>, name: &str, id: &str) -> Node<'a, 'input> {
    doc.descendants()
        .find(|n| n.has_tag_name(name) && n.attribute("id") == Some(id))
        .unwrap()
}
fn item_lines(item: Node<'_, '_>) -> Vec<String> {
    item.children()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}
fn modifier_lines(lines: &[String]) -> &[String] {
    let start = lines
        .iter()
        .position(|s| s.starts_with("Implicits: "))
        .unwrap();
    &lines[start + 1..]
}
fn quality(lines: &[String]) -> f64 {
    lines
        .iter()
        .find_map(|s| s.strip_prefix("Quality: "))
        .unwrap()
        .parse()
        .unwrap()
}
fn selected_slot<'a, 'input>(doc: &'a Document<'input>, slot: &str) -> Node<'a, 'input> {
    let items = element(doc, "Items");
    let selected = items.attribute("activeItemSet").unwrap();
    identified(doc, "ItemSet", selected)
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some(slot))
        .unwrap()
}
fn copy(oracle: &ItemOracle, t: &Table) -> Table {
    oracle
        .lua
        .globals()
        .get::<Function>("copyTable")
        .unwrap()
        .call(t.clone())
        .unwrap()
}

#[test]
#[ignore = "optional pinned PoB source oracle; requires vendor submodule"]
fn original_spear_local_values_use_authored_speed_and_preserve_nonlocal_evidence() {
    assert_eq!(sha256(ORIGINAL_02.as_bytes()), HASH_02);
    let doc = Document::parse(ORIGINAL_02).unwrap();
    assert_eq!(
        element(&doc, "Skills").attribute("activeSkillSet"),
        Some("6")
    );
    assert_eq!(
        identified(&doc, "ItemSet", "6").attribute("useSecondWeaponSet"),
        Some("true")
    );
    assert_eq!(
        selected_slot(&doc, "Weapon 1 Swap").attribute("itemId"),
        Some("26")
    );
    assert_eq!(
        selected_slot(&doc, "Weapon 1").attribute("itemId"),
        Some("15")
    );
    let raw = item_lines(identified(&doc, "Item", "26"));
    assert_eq!(&raw[2], "Grand Spear");
    let lines = modifier_lines(&raw);
    assert_eq!(lines.len(), 9);
    assert!(
        raw.iter()
            .any(|line| line == "Suffix: {range:1}LocalIncreasedAttackSpeed8")
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.as_str() == "49% increased Attack Speed")
            .count(),
        1
    );
    let mut baseline = None;
    for repeat in [false, true] {
        let oracle = ItemOracle::new(repeat);
        let base = oracle.base(&raw[2]);
        let weapon: Table = base.get("weapon").unwrap();
        assert_eq!(weapon.get::<f64>("PhysicalMin").unwrap(), 56.0);
        assert_eq!(weapon.get::<f64>("PhysicalMax").unwrap(), 84.0);
        assert!(
            oracle.source["Data/ModItem.lua"]
                .lines()
                .find(|s| s.contains("[\"LocalIncreasedAttackSpeed8\"]"))
                .unwrap()
                .contains("(26-28)% increased Attack Speed")
        );
        let (mods, evidence) = oracle.lines(lines);
        assert_eq!(evidence.iter().filter(|e| e.excluded_bonded).count(), 1);
        assert_eq!(
            evidence
                .iter()
                .find(|e| e.excluded_bonded)
                .unwrap()
                .original,
            "{enchant}{rune}Bonded: 20% increased effect of Fully Broken Armour"
        );
        for line in evidence.iter().filter(|e| !e.excluded_bonded) {
            assert!(
                line.unparsed.is_none(),
                "{}: {:?}",
                line.original,
                line.unparsed
            );
            assert!(!line.rows.is_empty(), "{}", line.original);
        }
        let (increase, second, remaining) = oracle.local(&mods, "PhysicalDamage", "INC", 0);
        assert_eq!(increase, 119.0);
        assert_eq!(second, 0.0);
        assert!(
            !remaining
                .iter()
                .any(|m| m.name == "PhysicalDamage" && m.kind == "INC")
        );
        let mut snapshots = vec![];
        for (quality, expected) in [
            (0.0, [123.0, 184.0]),
            (1.0, [124.0, 186.0]),
            (quality(&raw), [147.0, 221.0]),
        ] {
            let mut result = oracle.assemble(&base, &raw[1], quality, &mods);
            for _ in 0..if repeat { 32 } else { 0 } {
                let next = oracle.assemble(&base, &raw[1], quality, &mods);
                assert_eq!(next, result);
                result = next;
            }
            assert_eq!(result.damage["Physical"], Some(expected));
            assert_eq!(result.damage["Cold"], Some([49.0, 74.0]));
            assert_eq!(result.damage["Lightning"], Some([6.0, 179.0]));
            assert_eq!(result.damage["Fire"], None);
            assert_eq!(result.damage["Chaos"], None);
            assert_eq!(result.attack_speed_increased, 49.0);
            assert_eq!(result.attack_rate, 2.09);
            assert_eq!(result.critical_chance, 5.0);
            assert!(
                result
                    .remaining
                    .iter()
                    .any(|m| m.name == "CritMultiplier" && m.numeric_value == Some(16.0)),
                "critical damage bonus is not critical chance"
            );
            assert!(
                result
                    .remaining
                    .iter()
                    .any(|m| m.name == "PhysicalDamageLifeLeech"),
                "leech is outside the numeric-prefix extraction"
            );
            snapshots.push((quality, result));
        }
        if let Some(previous) = &baseline {
            assert_eq!(previous, &snapshots);
        } else {
            baseline = Some(snapshots.clone());
        }
        if !repeat {
            println!("{}",serde_json::to_string_pretty(&json!({"case":"original-02-item26-local","upstream_revision":REVISION,"source_pins_lf_sha256":PINS,"raw_xml_sha256":HASH_02,"selected_slot":"Weapon 1 Swap","input_boundary":"test-side XML selection, metadata stripping, source-loaded base and parsed complete lines; bonded line excluded explicitly; no ParseRaw/rune reconciliation or whole-build conversion","lines":evidence,"quality_vectors":snapshots,"coverage":"weapon numeric prefix only; later WeaponData, local-to-global conversions, bonded mechanics and selected action are not evaluated"})).unwrap());
        }
    }
}

#[test]
#[ignore = "optional pinned PoB source oracle; requires vendor submodule"]
fn local_predicate_does_not_consume_global_attack_damage_and_suppresses_incomplete_pairs() {
    for repeat in [false, true] {
        let oracle = ItemOracle::new(repeat);
        let raw = vec![
            "Adds 3 to 7 Physical Damage".into(),
            "Adds 3 to 7 Physical Damage to Attacks".into(),
        ];
        let (rows, evidence) = oracle.lines(&raw);
        assert!(evidence.iter().all(|e| e.unparsed.is_none()));
        let (value, again, remaining) = oracle.local(&rows, "PhysicalMin", "BASE", 0);
        assert_eq!(value, 3.0);
        assert_eq!(again, 0.0);
        assert!(
            remaining
                .iter()
                .any(|r| r.name == "PhysicalMin" && r.keyword_flags != 0)
        );
        let empty = oracle.lua.create_table().unwrap();
        let base = oracle.base("Grand Spear");
        let mut snapshots = vec![];
        for (minimum, maximum, rate, critical, quality, expected_pair) in [
            (0.0, 7.0, 1.4, 5.0, 20.0, None),
            (-1.0, 7.0, 1.4, 5.0, 0.0, None),
            (0.49, 0.5, 0.015, 5.005, 0.0, None),
            (0.49, 0.5, 0.015, 5.005, 20.0, Some([1.0, 1.0])),
        ] {
            let modified = copy(&oracle, &base);
            let weapon: Table = modified.get("weapon").unwrap();
            weapon.set("PhysicalMin", minimum).unwrap();
            weapon.set("PhysicalMax", maximum).unwrap();
            weapon.set("AttackRateBase", rate).unwrap();
            weapon.set("CritChanceBase", critical).unwrap();
            let result = oracle.assemble(&modified, "injected raw boundary", quality, &empty);
            assert_eq!(result.damage["Physical"], expected_pair);
            assert!(result.attack_rate.is_finite() && result.critical_chance.is_finite());
            if rate == 0.015 {
                assert_eq!(result.attack_rate, 0.02);
                assert_eq!(result.critical_chance, 5.01);
            }
            snapshots.push(result);
        }
        if !repeat {
            println!("{}",serde_json::to_string_pretty(&json!({"case":"source-local-predicate-boundaries","input_boundary":"explicitly injected numerical base fields; parsed local/global lines; not a raw item conversion","vectors":snapshots,"local_global_lines":evidence})).unwrap());
        }
    }
}

#[test]
#[ignore = "optional pinned PoB source oracle; requires vendor submodule"]
fn original_staff_range_and_grant_match_saved_generated_skill_without_retargeting_sniper() {
    assert_eq!(sha256(ORIGINAL_05.as_bytes()), HASH_05);
    let doc = Document::parse(ORIGINAL_05).unwrap();
    assert_eq!(
        selected_slot(&doc, "Weapon 1").attribute("itemId"),
        Some("28")
    );
    assert_eq!(
        element(&doc, "Skills").attribute("activeSkillSet"),
        Some("4")
    );
    assert_eq!(
        element(&doc, "Build").attribute("mainSocketGroup"),
        Some("3")
    );
    let set = identified(&doc, "SkillSet", "4");
    let groups = set
        .children()
        .filter(|n| n.has_tag_name("Skill"))
        .collect::<Vec<_>>();
    let summoner = groups[2];
    assert!(summoner.attribute("source").is_none());
    let sniper = summoner.children().find(|n| n.has_tag_name("Gem")).unwrap();
    assert_eq!(
        sniper.attribute("skillId"),
        Some("SummonSkeletalSnipersPlayer")
    );
    assert_eq!(
        sniper.attribute("skillMinion"),
        Some("RaisedSkeletonSniper")
    );
    assert_eq!(sniper.attribute("skillMinionSkill"), Some("1"));
    let generated = groups
        .iter()
        .find(|n| n.attribute("source") == Some("Item:28:New Item, Ashen Staff"))
        .unwrap();
    assert_eq!(generated.attribute("slot"), Some("Weapon 1"));
    let saved = generated
        .children()
        .find(|n| n.has_tag_name("Gem"))
        .unwrap();
    let item = identified(&doc, "Item", "28");
    let raw = item_lines(item);
    let lines = modifier_lines(&raw);
    assert_eq!(lines.len(), 2);
    assert_eq!(&raw[2], "Ashen Staff");
    assert_eq!(quality(&raw), 20.0);
    let authored = &lines[0];
    let (tag, line) = authored.split_once('}').unwrap();
    let range: f64 = tag.strip_prefix("{range:").unwrap().parse().unwrap();
    assert_eq!(range, 0.5);
    assert_eq!(
        item.children()
            .find(|n| n.has_tag_name("ModRange") && n.attribute("id") == Some("1"))
            .unwrap()
            .attribute("range"),
        Some("0.5")
    );
    let mut baseline = None;
    for repeat in [false, true] {
        let oracle = ItemOracle::new(repeat);
        let base = oracle.base(&raw[2]);
        assert_eq!(base.get::<String>("implicit").unwrap(), line);
        assert!(base.get::<Option<Table>>("weapon").unwrap().is_none());
        let mut vectors = vec![];
        for (range, expected) in [(0.0, 1), (range, 11), (1.0, 20)] {
            let formatted = oracle.range(line, range);
            let (rows, extra) = oracle.parse(&formatted);
            assert!(extra.is_none());
            let rows = rows.unwrap();
            assert_eq!(rows.raw_len(), 1);
            let record: Table = rows.get(1).unwrap();
            assert_eq!(record.get::<String>("name").unwrap(), "ExtraSkill");
            assert_eq!(record.get::<String>("type").unwrap(), "LIST");
            let value: Table = record.get("value").unwrap();
            assert_eq!(value.get::<String>("skillId").unwrap(), "FireboltPlayer");
            assert_eq!(value.get::<u32>("level").unwrap(), expected);
            let grants = oracle.grant(&base, generated.attribute("source").unwrap(), &rows);
            assert_eq!(grants.len(), 1);
            assert_eq!(grants[0].level, expected);
            assert_eq!(grants[0].skill_id, "FireboltPlayer");
            assert_eq!(grants[0].source, generated.attribute("source").unwrap());
            for _ in 0..if repeat { 32 } else { 0 } {
                assert_eq!(oracle.range(line, range), formatted);
                assert_eq!(
                    oracle.grant(&base, generated.attribute("source").unwrap(), &rows),
                    grants
                );
            }
            if range == 0.5 {
                assert_eq!(
                    saved.attribute("level").unwrap().parse::<u32>().unwrap(),
                    grants[0].level
                );
                assert_eq!(
                    saved.attribute("skillId"),
                    Some(grants[0].skill_id.as_str())
                );
            }
            vectors.push((range, formatted, grants));
        }
        let (spell, extra) = oracle.parse(&lines[1]);
        assert!(extra.is_none());
        let spell = modifiers(&spell.unwrap());
        assert_eq!(spell.len(), 1);
        assert_eq!(spell[0].name, "Damage");
        assert_eq!(spell[0].numeric_value, Some(128.0));
        assert_ne!(spell[0].flags, 0);
        if let Some(previous) = &baseline {
            assert_eq!(previous, &vectors);
        } else {
            baseline = Some(vectors.clone());
        }
        if !repeat {
            println!("{}",serde_json::to_string_pretty(&json!({"case":"original-05-item28-firebolt","upstream_revision":REVISION,"source_pins_lf_sha256":PINS,"raw_xml_sha256":HASH_05,"authored_implicit":authored,"range_vectors":vectors,"saved_generated_group":{"source":generated.attribute("source"),"slot":generated.attribute("slot"),"skill":saved.attribute("skillId"),"level":saved.attribute("level")},"unresolved_spell_damage":spell,"input_boundary":"test-side XML selection and range-tag decoding; source Gems/skill join; actual ItemTools applyRange, ModParser and Item.grantedSkills span; supplied receiving-source string, no CalcSetup or owned-provider resolution","preserved_manual_summoner":{"skill":sniper.attribute("skillId"),"minion":sniper.attribute("skillMinion"),"output_index":sniper.attribute("skillMinionSkill")},"coverage":"grant identity/level component only; no Firebolt damage or Sniper whole-build parity"})).unwrap());
        }
    }
}

#[test]
#[ignore = "optional pinned PoB source oracle; requires vendor submodule"]
fn owned_rules_match_source_weapon_components_with_candidate_dependent_facts() {
    use owned_item_rule_fixture::{boolean_fact, fixture, key, number_fact};
    use poe_optimizer_core::owned_build::ParameterValue;
    use poe_optimizer_engine::owned_rules::EffectDisposition;
    assert_eq!(sha256(ORIGINAL_02.as_bytes()), HASH_02);
    let doc = Document::parse(ORIGINAL_02).unwrap();
    assert_eq!(
        selected_slot(&doc, "Weapon 1 Swap").attribute("itemId"),
        Some("26")
    );
    let raw = item_lines(identified(&doc, "Item", "26"));
    let oracle = ItemOracle::new(false);
    let base = oracle.base(&raw[2]);
    let native = fixture();
    let mut scratch = native.compiled.new_scratch();
    let mut ledger = vec![];
    // The contribution facts below are explicitly supplied local-only reductions.
    // Their values are authored in this item (18+101 physical, flat elemental,
    // speed49); they are never calculated from the oracle's output endpoints.
    // Alternate quality, bonded modifiers and modifier applicability resolution
    // are outside this component fixture. Source ModParser checks source lines.
    for (quality, speed, physical_min) in [
        (0.0, 49.0, None),
        (20.0, 49.0, None),
        (20.0, 73.0, None),
        (0.0, 49.0, None),
        (20.0, 49.0, Some(0.0)),
    ] {
        let candidate_base = copy(&oracle, &base);
        let weapon: Table = candidate_base.get("weapon").unwrap();
        if let Some(minimum) = physical_min {
            weapon.set("PhysicalMin", minimum).unwrap();
        }
        let mut input_lines = modifier_lines(&raw).to_vec();
        let line = input_lines
            .iter_mut()
            .find(|line| line.as_str() == "49% increased Attack Speed")
            .unwrap();
        *line = format!("{speed}% increased Attack Speed");
        let (rows, source_lines) = oracle.lines(&input_lines);
        assert!(
            source_lines
                .iter()
                .filter(|line| !line.excluded_bonded)
                .all(|line| line.unparsed.is_none())
        );
        let physical_parts: Vec<_> = source_lines
            .iter()
            .flat_map(|line| &line.rows)
            .filter(|row| row.name == "PhysicalDamage" && row.kind == "INC")
            .map(|row| row.numeric_value.unwrap())
            .collect();
        assert_eq!(physical_parts, vec![18.0, 101.0]);
        let reference = oracle.assemble(&candidate_base, &raw[1], quality, &rows);
        let mut facts = vec![number_fact("quality", quality, "percent")];
        for (channel, added, increase) in [
            ("Physical", Some([0.0, 0.0]), 18.0 + 101.0),
            ("Lightning", Some([6.0, 179.0]), 0.0),
            ("Cold", Some([49.0, 74.0]), 0.0),
            ("Fire", None, 0.0),
            ("Chaos", None, 0.0),
        ] {
            // Explicit absence of this channel in the reviewed base+local facts
            // does not become numeric zero output. Its numeric facts are omitted.
            facts.push(boolean_fact(&format!("{channel}.present"), added.is_some()));
            if let Some(added) = added {
                facts.push(number_fact(
                    &format!("{channel}.increase"),
                    increase,
                    "percent",
                ));
                for (position, endpoint) in ["Min", "Max"].into_iter().enumerate() {
                    // PoB's base table omits elemental fields. The additive zero
                    // here is a supplied empty base contribution, not a missing
                    // evaluator fact default. The channel exists through flats.
                    let value = weapon
                        .get::<Option<f64>>(format!("{channel}{endpoint}"))
                        .unwrap()
                        .unwrap_or(0.0);
                    facts.push(number_fact(
                        &format!("{channel}.{endpoint}.base"),
                        value,
                        "damage",
                    ));
                    facts.push(number_fact(
                        &format!("{channel}.{endpoint}.flat"),
                        added[position],
                        "damage",
                    ));
                }
            }
        }
        for (name, unit, base_field, increase) in [
            ("rate", "rate-unit", "AttackRateBase", speed),
            ("crit", "percent", "CritChanceBase", 0.0),
        ] {
            facts.push(number_fact(
                &format!("{name}.base"),
                weapon.get(base_field).unwrap(),
                unit,
            ));
            if name == "crit" {
                facts.push(number_fact("crit.flat", 0.0, unit));
            }
            facts.push(number_fact(
                &format!("{name}.increase"),
                increase,
                "percent",
            ));
        }
        let actual = native
            .compiled
            .evaluate(
                &native.weapon_owner,
                &key("weapon"),
                &facts,
                &native.schema,
                &mut scratch,
            )
            .unwrap();
        assert_eq!(actual.effects.len(), 12);
        let by_id = actual
            .effects
            .iter()
            .map(|e| (e.id.as_str(), &e.disposition))
            .collect::<std::collections::BTreeMap<_, _>>();
        for channel in owned_item_rule_fixture::CHANNELS {
            for (position, endpoint) in ["Min", "Max"].into_iter().enumerate() {
                let name = format!("{channel}.{endpoint}").to_ascii_lowercase();
                match reference.damage[channel] {
                    Some(pair) => match by_id[name.as_str()] {
                        EffectDisposition::Applied {
                            value: ParameterValue::Quantity(q),
                        } => assert_eq!(q.value(), pair[position], "{name}, quality{quality}"),
                        disposition => {
                            panic!("{name} expected concrete source endpoint, got {disposition:?}")
                        }
                    },
                    None => assert_eq!(by_id[name.as_str()], &EffectDisposition::Inactive),
                }
            }
        }
        for (name, expected) in [
            ("rate", reference.attack_rate),
            ("crit", reference.critical_chance),
        ] {
            let EffectDisposition::Applied {
                value: ParameterValue::Quantity(value),
            } = by_id[name]
            else {
                panic!("expected {name}")
            };
            // Decimal-rate source rounding multiplies by 100; the owned quantum
            // form divides by .01. Permit only floating representation noise.
            assert!(
                (value.value() - expected).abs() <= 1e-12,
                "{name}: {} vs {expected}",
                value.value()
            );
        }
        // A demanded missing input stays unresolved; the same scratch recovers
        // on the next complete candidate and never fills it with zero.
        let without_flat: Vec<_> = facts
            .iter()
            .filter(|f| f.read != key("Physical.Min.flat"))
            .cloned()
            .collect();
        let missing = native
            .compiled
            .evaluate(
                &native.weapon_owner,
                &key("weapon"),
                &without_flat,
                &native.schema,
                &mut scratch,
            )
            .unwrap();
        assert!(missing.effects.iter().filter(|e| e.id.as_str().starts_with("physical.")).all(|e| matches!(&e.disposition, EffectDisposition::Unresolved { input } if input == &key("Physical.Min.flat"))));
        let again = native
            .compiled
            .evaluate(
                &native.weapon_owner,
                &key("weapon"),
                &facts,
                &native.schema,
                &mut scratch,
            )
            .unwrap();
        assert_eq!(again, actual);
        ledger.push(
            json!({"quality":quality,"supplied_speed":speed,"reference":reference,"owned":actual}),
        );
    }
    assert_eq!(ledger[0], ledger[3]);
    assert_ne!(
        ledger[1]["reference"]["attack_rate"],
        ledger[2]["reference"]["attack_rate"]
    );
    println!("{}", serde_json::to_string_pretty(&json!({"case":"owned-rule-versus-original02-local-components", "upstream_revision":REVISION,"source_pins_lf_sha256":PINS,"raw_xml_sha256":HASH_02,"vectors":ledger,"coverage":"source-loaded base plus explicitly supplied local reductions; no native source conversion, incoming-effect closure, provider binding, alternate quality or whole-build parity"})).unwrap());
}

#[test]
#[ignore = "optional pinned PoB source oracle; requires vendor submodule"]
fn owned_integer_projection_matches_source_grant_with_supplied_decoded_levels() {
    use owned_item_rule_fixture::{fixture, key, level_fact};
    use poe_optimizer_core::owned_build::ParameterValue;
    use poe_optimizer_engine::owned_rules::EffectDisposition;
    assert_eq!(sha256(ORIGINAL_05.as_bytes()), HASH_05);
    let doc = Document::parse(ORIGINAL_05).unwrap();
    assert_eq!(
        selected_slot(&doc, "Weapon 1").attribute("itemId"),
        Some("28")
    );
    let raw = item_lines(identified(&doc, "Item", "28"));
    let line = modifier_lines(&raw)[0].split_once('}').unwrap().1;
    let oracle = ItemOracle::new(false);
    let base = oracle.base(&raw[2]);
    assert_eq!(base.get::<String>("implicit").unwrap(), line);
    let native = fixture();
    let mut scratch = native.compiled.new_scratch();
    let mut ledger = vec![];
    // These are caller-supplied decoded integer inputs. Native range decoding
    // is tested separately in Import; Engine has no Import dependency and this
    // projection does not pretend to implement interpolation or provider binding.
    for (fraction, decoded) in [(0.0, 1), (0.5, 11), (1.0, 20), (0.25, 6)] {
        let text = oracle.range(line, fraction);
        let (rows, extra) = oracle.parse(&text);
        assert!(extra.is_none());
        let expected = oracle.grant(&base, "Item:28:New Item, Ashen Staff", &rows.unwrap());
        assert_eq!(expected.len(), 1);
        let result = native
            .compiled
            .evaluate(
                &native.staff_owner,
                &key("grant"),
                &[level_fact(decoded)],
                &native.schema,
                &mut scratch,
            )
            .unwrap();
        assert_eq!(result.effects.len(), 1);
        let EffectDisposition::Applied {
            value: ParameterValue::Integer(level),
        } = &result.effects[0].disposition
        else {
            panic!("applied integer projection")
        };
        assert_eq!(level.get(), i64::from(expected[0].level));
        ledger.push(json!({"source_fraction":fraction,"supplied_decoded_integer":decoded,"reference":expected,"owned":result}));
    }
    let missing = native
        .compiled
        .evaluate(
            &native.staff_owner,
            &key("grant"),
            &[],
            &native.schema,
            &mut scratch,
        )
        .unwrap();
    assert_eq!(
        missing.effects[0].disposition,
        EffectDisposition::Unresolved {
            input: key("decoded-level")
        }
    );
    let unsupported = native
        .compiled
        .evaluate(
            &native.staff_owner,
            &key("grant"),
            &[level_fact(21)],
            &native.schema,
            &mut scratch,
        )
        .unwrap();
    assert_eq!(
        unsupported.effects[0].disposition,
        EffectDisposition::UnsupportedValue {
            value: level_fact(21).value
        }
    );
    println!("{}", serde_json::to_string_pretty(&json!({"case":"owned-integer-projection-versus-original05-grant", "upstream_revision":REVISION,"source_pins_lf_sha256":PINS,"raw_xml_sha256":HASH_05,"vectors":ledger,"coverage":"supplied decoded integer -> exact declared skill parameter only; native interpolation and concrete skill/provider resolution are not exercised here"})).unwrap());
}
