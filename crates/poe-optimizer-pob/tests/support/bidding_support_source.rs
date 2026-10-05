//! Supplemental delivery evidence; the historical preparation report is unchanged.
use super::*;
use std::ops::Range;

const TEST_NAME: &str =
    "bidding_support::complete_bidding_support_delivery_uses_exact_minion_recipients";
const MODE: &str = "POE_BIDDING_SUPPORT_DELIVERY_CHILD";
const DELIVERY: &str = include_str!("bidding_support_delivery.lua");
const II: &str = "SupportBiddingPlayerTwo";
const III: &str = "SupportBiddingPlayerThree";

#[test]
#[ignore = "requires complete pinned PoB runtime; Bidding numerical source delivery, both JIT modes"]
fn complete_bidding_support_delivery_uses_exact_minion_recipients() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join(
        std::env::var_os("POE_BIDDING_SUPPORT_OUT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("runs/owned-bidding-support-source-01")),
    );
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(MODE) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    assert!(
        std::env::var_os(CHILD).is_none(),
        "unset the historical preparation child selector"
    );
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST_NAME, "--ignored", "--nocapture"])
            .env(MODE, mode)
            .env_remove(CHILD)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!(
                    "Bidding source deadline: {}\n{}",
                    path.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "Bidding source delivery JIT parity",
    );
}

fn child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read(dir.join(format!("build-{i:02}.xml"))).unwrap())
        .collect();
    let mut cases = vec![];
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(digest(bytes), index["builds"][i]["xml_sha256"]);
        cases.push(observe_with_extra(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            None,
            Some(DELIVERY),
        ));
    }
    let original = std::str::from_utf8(&originals[4]).unwrap();
    let original_one = std::str::from_utf8(&originals[0]).unwrap();
    for (name, xml, control) in controls(original, original_one) {
        roxmltree::Document::parse(&xml).unwrap();
        assert_ne!(xml, original);
        cases.push(observe_with_extra(
            root,
            &name,
            &xml,
            enabled,
            Some(control),
            Some(DELIVERY),
        ));
    }
    cases.push(observe_with_extra(
        root,
        "repeat-original-05",
        original,
        enabled,
        None,
        Some(DELIVERY),
    ));
    assert_eq!(cases.len(), 22);
    let mut files = FILES.to_vec();
    files.extend([
        "src/Data/SkillStatMap.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/ModList.lua",
    ]);
    let report = json!({
        "source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4",
        "manifest_sha256":pinned::manifest_sha256(),
        "files":files.into_iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "business_wrappers":false,"source_tables_mutated":false,"source_cfg_modified":false,
        "native_build_parity":false,"native_inventory_authority":false,"canonical_parity_lifecycle_selected":false,
        "evidence_view":"raw_source_observation",
        "determinism_contract":{
            "deterministic_view":"bidding_distinct_channel_projection_v1",
            "raw_parent_outer_cross_channel_order":"unspecified",
            "source_reason":"CalcActiveSkill.mergeStatSet iterates pairs(stats); MinionModifier transfers exact nested objects into separate named ModDB channels",
            "projected_fields":["delivery.contexts[].parent_minion_modifiers","delivery.contexts[].children[].queries.producer_joins[].parent_outer_indices"],
            "same_channel_arithmetic_order":"preserved",
            "all_other_fields":"exact"
        },
        "lifecycle_stages":STAGES,"cases":cases
    });
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(
        bytes.len() <= 64 * 1024 * 1024,
        "Bidding evidence is {} bytes",
        bytes.len()
    );
    fs::write(
        out.join(format!(
            "source-jit-{}.raw.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    check_delivery(&report);
    let semantic = semantic_report(report);
    let bytes = serde_json::to_vec(&semantic).unwrap();
    assert!(bytes.len() <= 64 * 1024 * 1024);
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    let cases = rows(&semantic["cases"]);
    assert_exact(
        &cases[4]["states"],
        &cases.last().unwrap()["states"],
        "fresh independent Original05 semantic replay",
    );
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            fs::read(dir.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            *bytes
        );
    }
}

fn assert_exact(left: &Json, right: &Json, label: &str) {
    if let Some(difference) = json_evidence::first_difference(left, right, "$") {
        panic!("{label}: {difference}; full raw and semantic evidence remains on disk");
    }
}

// mlua serializes an unmarked empty Lua table as {}, not []. Accept precisely
// that source representation without rewriting it or accepting object-shaped rows.
fn source_rows_mut(value: &mut Json) -> &mut [Json] {
    match value {
        Json::Array(rows) => rows,
        Json::Object(fields) if fields.is_empty() => &mut [],
        _ => panic!("source rows must be an array or an empty Lua table"),
    }
}

/// Only the cross-channel ordering of the two reviewed parent LIST records is
/// incidental: the pinned merger traverses `pairs(stats)`, while their nested
/// records are consumed by separate named numerical channels. Keep the untouched
/// raw report and preserve every other field, especially child arithmetic order.
fn semantic_report(mut report: Json) -> Json {
    assert_eq!(report["evidence_view"], "raw_source_observation");
    for case in source_rows_mut(&mut report["cases"]) {
        for stage in STAGES {
            for parent in source_rows_mut(&mut case["states"][stage]["delivery"]["contexts"]) {
                project_parent_channels(parent);
            }
        }
    }
    report["evidence_view"] = json!("bidding_distinct_channel_projection_v1");
    report
}

fn project_parent_channels(parent: &mut Json) {
    let outer = parent
        .as_object_mut()
        .unwrap()
        .remove("parent_minion_modifiers")
        .unwrap();
    let mut channels = serde_json::Map::new();
    let mut addresses = Vec::new();
    for raw in rows(&outer) {
        assert_eq!(raw["exact_base_skill_store"], true);
        assert!(raw["ancestor_depth"].as_u64().is_some());
        let inner = &raw["record"]["value"]["mod"];
        let channel = match (inner["name"].as_str(), inner["type"].as_str()) {
            (Some("Damage"), Some("MORE")) => "Damage/MORE",
            (Some("CooldownRecovery"), Some("INC")) => "CooldownRecovery/INC",
            _ => panic!("unreviewed Bidding parent channel cannot be projected"),
        };
        // The raw inventory proof permits one retained family position, with
        // exactly one record per channel. Never sort duplicate arithmetic terms.
        assert!(
            !channels.contains_key(channel),
            "duplicate Bidding parent channel"
        );
        let mut record = raw.clone();
        assert!(
            record
                .as_object_mut()
                .unwrap()
                .remove("position")
                .unwrap()
                .as_u64()
                .is_some()
        );
        channels.insert(channel.into(), json!([record]));
        addresses.push(json!({"channel":channel,"channel_index":1}));
    }
    for child in source_rows_mut(&mut parent["children"]) {
        let queries = &mut child["queries"];
        let raw_len = rows(&queries["raw_bidding_records"]).len();
        let joins = source_rows_mut(&mut queries["producer_joins"]);
        assert_eq!(joins.len(), raw_len);
        for (index, join) in joins.iter_mut().enumerate() {
            assert_eq!(join["raw_index"], index + 1);
            assert_eq!(join["exact_inner_object"], true);
            let indices = join
                .as_object_mut()
                .unwrap()
                .remove("parent_outer_indices")
                .unwrap();
            let indices = rows(&indices);
            assert_eq!(indices.len(), 1);
            let raw_index = usize::try_from(indices[0].as_u64().unwrap()).unwrap();
            let address = addresses.get(raw_index.checked_sub(1).unwrap()).unwrap();
            join["parent_outer_channels"] = json!([address]);
        }
    }
    parent["parent_minion_modifiers_by_channel"] = Json::Object(channels);
}

fn group_range(xml: &str, effect: &str) -> (Range<usize>, usize) {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = doc
        .descendants()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some("4"))
        .unwrap();
    let groups: Vec<_> = set.children().filter(|n| n.has_tag_name("Skill")).collect();
    let matches: Vec<_> = groups
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            n.attribute("source").is_none()
                && n.children()
                    .any(|c| c.has_tag_name("Gem") && c.attribute("skillId") == Some(effect))
        })
        .collect();
    assert_eq!(matches.len(), 1);
    (matches[0].1.range(), matches[0].0 + 1)
}
fn set_attr(text: &str, name: &str, value: &str) -> String {
    let node = roxmltree::Document::parse(text).unwrap();
    let node = node.root_element();
    let mut out = text.to_owned();
    if let Some(a) = node.attributes().find(|a| a.name() == name) {
        out.replace_range(a.range(), &format!("{name}=\"{value}\""));
    } else {
        let at = text.find('>').unwrap();
        let at = if text.as_bytes()[at - 1] == b'/' {
            at - 1
        } else {
            at
        };
        out.insert_str(at, &format!(" {name}=\"{value}\""));
    }
    out
}
fn edit_group(xml: &str, effect: &str, edit: impl FnOnce(&str) -> String) -> String {
    let (range, _) = group_range(xml, effect);
    let next = edit(&xml[range.clone()]);
    let mut out = xml.to_owned();
    out.replace_range(range, &next);
    out
}
fn edit_gem(group: &str, effect: &str, edit: impl FnOnce(&str) -> String) -> String {
    let doc = roxmltree::Document::parse(group).unwrap();
    let gems: Vec<_> = doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(effect))
        .collect();
    assert_eq!(gems.len(), 1);
    let range = gems[0].range();
    let mut out = group.to_owned();
    out.replace_range(range.clone(), &edit(&group[range]));
    out
}
fn focus(xml: &str, effect: &str, child: usize, calcs_set: usize) -> String {
    let (_, index) = group_range(xml, effect);
    let mut next = edit_group(xml, effect, |g| {
        let g = set_attr(
            &set_attr(g, "mainActiveSkill", "1"),
            "mainActiveSkillCalcs",
            "1",
        );
        edit_gem(&g, effect, |gem| {
            let mut gem = set_attr(
                &set_attr(gem, "skillMinionSkill", &child.to_string()),
                "skillMinionSkillCalcs",
                &child.to_string(),
            );
            assert!(gem.trim_end().ends_with("/>"));
            let at = gem.rfind("/>").unwrap();
            gem.replace_range(at..at+2,&format!("><MinionSkillIndexLookup grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"{child}\" statSetIndex=\"1\"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"{child}\" statSetIndex=\"{calcs_set}\"/></MinionSkillIndexLookupCalcs></Gem>"));
            gem
        })
    });
    let doc = roxmltree::Document::parse(&next).unwrap();
    let node = doc.descendants().find(|n| n.has_tag_name("Build")).unwrap();
    let range = node.range();
    let replacement = set_attr(&next[range.clone()], "mainSocketGroup", &index.to_string());
    next.replace_range(range, &replacement);
    let doc = roxmltree::Document::parse(&next).unwrap();
    let node = doc
        .descendants()
        .find(|n| n.has_tag_name("Calcs"))
        .unwrap()
        .children()
        .find(|n| n.has_tag_name("Input") && n.attribute("name") == Some("skill_number"))
        .unwrap();
    let range = node.range();
    let replacement = set_attr(&next[range.clone()], "number", &index.to_string());
    next.replace_range(range, &replacement);
    next
}
fn third_template(original_one: &str) -> String {
    let doc = roxmltree::Document::parse(original_one).unwrap();
    let gem = doc
        .descendants()
        .find(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(III))
        .unwrap();
    original_one[gem.range()].to_owned()
}
fn controls(original: &str, original_one: &str) -> Vec<(String, String, Json)> {
    let mut out = vec![];
    for (family, effect, children) in [("sand", SAND, 3), ("water", WATER, 5)] {
        for child in 1..=children {
            let calcs_set = if family == "sand" && child <= 2 { 2 } else { 1 };
            out.push((format!("focus-{family}-child-{child}"),focus(original,effect,child,calcs_set),json!({"kind":"focus","family":family,"effect":effect,"child":child,"main_set":1,"calcs_set":calcs_set})));
        }
        let focused = focus(original, effect, 1, if family == "sand" { 2 } else { 1 });
        let (disabled, _) = disable_support(&focused, effect, II);
        out.push((
            format!("disable-{family}-bidding"),
            disabled,
            json!({"kind":"disable","family":family,"effect":effect,"child":1}),
        ));
        let third = third_template(original_one);
        let replacement = edit_group(&focused, effect, |g| edit_gem(g, II, |_| third.clone()));
        out.push((
            format!("replace-{family}-bidding-iii"),
            replacement,
            json!({"kind":"replace","family":family,"effect":effect,"child":1,"winner":III}),
        ));
    }
    let focused = focus(original, SAND, 1, 2);
    let third = third_template(original_one);
    for append in [false, true] {
        let changed = edit_group(&focused, SAND, |g| {
            edit_gem(g, II, |old| {
                if append {
                    format!("{old}{third}")
                } else {
                    format!("{third}{old}")
                }
            })
        });
        out.push((format!("same-family-{}",if append{"iii-last"}else{"ii-last"}),changed,json!({"kind":"family_duplicate","effect":SAND,"child":1,"winner":if append{III}else{II}})));
    }
    for quality in [0, 15] {
        let changed = edit_group(&focused, SAND, |g| {
            edit_gem(g, II, |old| {
                format!("{old}{}", set_attr(old, "quality", &quality.to_string()))
            })
        });
        out.push((format!("same-definition-quality-{quality}"),changed,json!({"kind":"definition_duplicate","effect":SAND,"child":1,"winner":II,"quality":quality})));
    }
    assert_eq!(out.len(), 16);
    out
}

fn source_values(queries: &Json, field: &str) -> Vec<(String, f64)> {
    rows(&queries[field])
        .iter()
        .filter_map(|r| {
            r["source_effect"]
                .as_str()
                .map(|s| (s.to_owned(), r["value"].as_f64().unwrap()))
        })
        .collect()
}
fn check_delivery(report: &Json) {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 22);
    let mut witnessed = std::collections::BTreeSet::new();
    for case in cases {
        for stage in STAGES {
            let states = &case["states"][stage];
            let delivery = &states["delivery"];
            assert_eq!(delivery["original_methods_preserved"], true);
            assert_eq!(delivery["jit_mode_preserved"], true);
            assert_eq!(delivery["source_cfg_modified"], false);
            assert_eq!(delivery["source_tables_mutated"], false);
            assert_eq!(delivery["business_wrappers"], false);
            assert_eq!(rows(&delivery["definitions"]).len(), 2);
            for parent in rows(&delivery["contexts"]) {
                let accepted: Vec<_> = rows(&parent["candidates"])
                    .iter()
                    .filter(|c| c["accepted"] == true)
                    .collect();
                assert!(accepted.len() <= 1);
                let outer = rows(&parent["parent_minion_modifiers"]);
                let chain = rows(&parent["parent_mod_store_chain"]);
                assert_eq!(
                    chain
                        .iter()
                        .filter(|s| s["exact_base_skill_store"] == true)
                        .count(),
                    1,
                    "exact source-owned base list must survive in parent ancestry"
                );
                assert_eq!(
                    outer.len(),
                    accepted
                        .first()
                        .map_or(0, |c| if c["effect"] == II { 2 } else { 1 }),
                    "complete Bidding parent record inventory: {}",
                    case["name"]
                );
                for record in outer {
                    assert_eq!(record["exact_base_skill_store"], true);
                    let depth = record["ancestor_depth"].as_u64().unwrap() as usize;
                    assert!(depth < chain.len());
                    assert_eq!(chain[depth]["ancestor_depth"], depth);
                    assert_eq!(chain[depth]["exact_base_skill_store"], true);
                    assert_eq!(record["record"]["name"], "MinionModifier");
                    assert_eq!(record["record"]["type"], "LIST");
                    let inner = &record["record"]["value"]["mod"];
                    assert_eq!(inner["flags"], 0);
                    assert_eq!(inner["keywordFlags"], 0);
                    let tags = rows(&inner["positions"]);
                    assert_eq!(tags.len(), 1);
                    assert_eq!(tags[0]["index"], 1);
                    assert_eq!(tags[0]["value"]["type"], "Condition");
                    assert_eq!(tags[0]["value"]["var"], "CommandableSkill");
                    let expected = if record["source_effect"] == II {
                        30
                    } else {
                        80
                    };
                    assert_eq!(inner["value"], expected);
                    if inner["name"] == "Damage" {
                        assert_eq!(record["source_effect"], II);
                        assert_eq!(inner["type"], "MORE");
                    } else {
                        assert_eq!(inner["name"], "CooldownRecovery");
                        assert_eq!(inner["type"], "INC");
                    }
                }
                let player = &parent["player_queries"];
                assert!(
                    source_values(player, "cooldown_records").is_empty(),
                    "Bidding leaked into player cooldown: {}",
                    case["name"]
                );
                assert!(
                    source_values(player, "damage_records").is_empty(),
                    "Bidding leaked into player damage: {}",
                    case["name"]
                );
                for child in rows(&parent["children"]) {
                    let q = &child["queries"];
                    assert_eq!(q["original_cfg_unchanged"], true);
                    assert_eq!(q["original_store_unchanged"], true);
                    assert_eq!(
                        rows(&q["raw_bidding_records"]).len(),
                        rows(&q["producer_joins"]).len()
                    );
                    for (index, join) in rows(&q["producer_joins"]).iter().enumerate() {
                        assert_eq!(join["raw_index"], index + 1);
                        assert_eq!(join["exact_inner_object"], true);
                        let indices = rows(&join["parent_outer_indices"]);
                        assert_eq!(indices.len(), 1);
                        let parent_index = usize::try_from(indices[0].as_u64().unwrap()).unwrap();
                        assert!(parent_index > 0 && parent_index <= outer.len());
                    }
                    if parent["selected"] != true || child["selected"] != true {
                        continue;
                    }
                    let candidates: Vec<_> = rows(&parent["candidates"])
                        .iter()
                        .filter(|c| c["accepted"] == true)
                        .collect();
                    assert!(
                        candidates.len() <= 1,
                        "family must have one retained support"
                    );
                    let eligible = q["commandable"].as_bool().unwrap();
                    let expected: Vec<_> = candidates
                        .iter()
                        .filter(|_| eligible)
                        .map(|c| {
                            (
                                c["effect"].as_str().unwrap().to_owned(),
                                if c["effect"] == II { 30.0 } else { 80.0 },
                            )
                        })
                        .collect();
                    assert_eq!(
                        source_values(q, "cooldown_records"),
                        expected,
                        "{}/{stage}/{}/{}",
                        case["name"],
                        parent["mode"],
                        child["effect"]
                    );
                    let damage: Vec<_> = candidates
                        .iter()
                        .filter(|c| eligible && c["effect"] == II)
                        .map(|_| (II.to_owned(), 30.0))
                        .collect();
                    assert_eq!(
                        source_values(q, "damage_records"),
                        damage,
                        "{}/{stage}/{}",
                        case["name"],
                        child["effect"]
                    );
                    witnessed.insert((
                        child["effect"].as_str().unwrap().to_owned(),
                        child["stat_set_index"].as_u64().unwrap(),
                    ));
                    if child["effect"] == "PassiveTriggeredManaWaveWaterDjinn" {
                        assert!(!eligible);
                    }
                }
            }
            let control = &case["control"];
            if !control.is_null() {
                for mode in ["MAIN", "CALCS"] {
                    let parents: Vec<_> = rows(&delivery["contexts"])
                        .iter()
                        .filter(|p| {
                            p["mode"] == mode
                                && p["effect"] == control["effect"]
                                && p.get("group_source").is_none()
                        })
                        .collect();
                    assert_eq!(parents.len(), 1);
                    let parent = parents[0];
                    assert_eq!(parent["selected"], true);
                    let chosen: Vec<_> = rows(&parent["children"])
                        .iter()
                        .filter(|c| c["selected"] == true)
                        .collect();
                    assert_eq!(chosen.len(), 1);
                    assert_eq!(chosen[0]["index"], control["child"]);
                    let candidates = rows(&parent["candidates"]);
                    if control["kind"] == "disable" {
                        assert!(candidates.is_empty());
                    }
                    if let Some(winner) = control["winner"].as_str() {
                        assert_eq!(candidates.len(), 1);
                        assert_eq!(candidates[0]["effect"], winner);
                    }
                    if control["kind"] == "definition_duplicate" {
                        assert_eq!(candidates[0]["quality"], control["quality"]);
                    }
                    if control["kind"] == "focus" {
                        assert_eq!(
                            chosen[0]["stat_set_index"],
                            control[if mode == "MAIN" {
                                "main_set"
                            } else {
                                "calcs_set"
                            }]
                        );
                    }
                }
            }
        }
    }
    assert_eq!(
        witnessed.len(),
        10,
        "every constructed Djinn child stat set must execute through genuine selection"
    );
    for case in cases.iter().take(5) {
        assert!(case["control"].is_null());
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    fn parent() -> Json {
        let record = |name: &str, kind: &str, position: u64| {
            json!({
                "position":position,"ancestor_depth":1,"exact_base_skill_store":true,"source_effect":II,
                "record":{"name":"MinionModifier","type":"LIST","source":"Skill:SupportBiddingPlayerTwo",
                    "value":{"mod":{"name":name,"type":kind,"value":30,"source":"Skill:SupportBiddingPlayerTwo",
                        "positions":[{"index":1,"value":{"type":"Condition","var":"CommandableSkill"}}]}}}
            })
        };
        json!({
            "effect":SAND,"source":{"position":1,"raw_quality":0},
            "parent_minion_modifiers":[record("Damage","MORE",10),record("CooldownRecovery","INC",11)],
            "children":[{"queries":{
                "cfg":{"flags":1},"cooldown":30,"damage_factor":1.3,
                "raw_bidding_records":[{"name":"Damage","position":1},{"name":"CooldownRecovery","position":1}],
                "damage_records":[{"value":30,"source":"first"},{"value":10,"source":"second"}],
                "producer_joins":[
                    {"raw_index":1,"exact_inner_object":true,"parent_outer_indices":[1]},
                    {"raw_index":2,"exact_inner_object":true,"parent_outer_indices":[2]}]
            }}]
        })
    }

    fn projected(mut value: Json) -> Json {
        project_parent_channels(&mut value);
        value
    }

    #[test]
    fn empty_source_tables_preserve_their_original_encoding() {
        for empty in [json!({}), json!([])] {
            // Original02/03/04 have no Djinn contexts; the source emits {}.
            let mut states = serde_json::Map::new();
            for stage in STAGES {
                states.insert(stage.into(), json!({"delivery":{"contexts":empty}}));
            }
            let report = json!({"evidence_view":"raw_source_observation","cases":[{
                "name":"original-02","states":states
            }]});
            let result = semantic_report(report.clone());
            assert_eq!(result["cases"], report["cases"]);

            let mut no_children = parent();
            no_children["children"] = empty.clone();
            assert_eq!(projected(no_children)["children"], empty);

            let mut no_delivery = parent();
            no_delivery["children"][0]["queries"]["raw_bidding_records"] = empty.clone();
            no_delivery["children"][0]["queries"]["producer_joins"] = empty.clone();
            let result = projected(no_delivery);
            assert_eq!(
                result["children"][0]["queries"]["raw_bidding_records"],
                empty
            );
            assert_eq!(result["children"][0]["queries"]["producer_joins"], empty);
        }
        for mut invalid in [json!({"0":{}}), json!(null), json!(false)] {
            assert!(
                std::panic::catch_unwind(move || {
                    source_rows_mut(&mut invalid);
                })
                .is_err()
            );
        }
    }

    #[test]
    fn only_distinct_parent_channel_interleaving_is_incidental() {
        let original = parent();
        let mut swapped = original.clone();
        swapped["parent_minion_modifiers"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        swapped["parent_minion_modifiers"][0]["position"] = json!(10);
        swapped["parent_minion_modifiers"][1]["position"] = json!(11);
        swapped["children"][0]["queries"]["producer_joins"][0]["parent_outer_indices"] = json!([2]);
        swapped["children"][0]["queries"]["producer_joins"][1]["parent_outer_indices"] = json!([1]);
        assert_ne!(original, swapped);
        assert_eq!(
            serde_json::to_vec(&projected(original)).unwrap(),
            serde_json::to_vec(&projected(swapped)).unwrap()
        );
    }

    #[test]
    fn values_identity_ancestry_cfg_and_child_arithmetic_order_remain_exact() {
        let original = parent();
        let baseline = projected(original.clone());
        for (pointer, value) in [
            (
                "/parent_minion_modifiers/0/record/value/mod/value",
                json!(31),
            ),
            ("/parent_minion_modifiers/0/source_effect", json!(III)),
            ("/parent_minion_modifiers/0/ancestor_depth", json!(2)),
            ("/source/position", json!(2)),
            ("/children/0/queries/cfg/flags", json!(2)),
            ("/children/0/queries/cooldown", json!(31)),
            (
                "/children/0/queries/raw_bidding_records/0/position",
                json!(2),
            ),
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(pointer).unwrap() = value;
            assert_ne!(baseline, projected(changed), "{pointer}");
        }
        let mut changed = original;
        changed["children"][0]["queries"]["damage_records"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        assert_ne!(
            baseline,
            projected(changed),
            "same-channel arithmetic order"
        );
    }

    #[test]
    fn unknown_duplicate_channels_and_unverified_joins_are_rejected() {
        for (pointer, value) in [
            (
                "/parent_minion_modifiers/0/record/value/mod/name",
                json!("Unknown"),
            ),
            (
                "/children/0/queries/producer_joins/0/exact_inner_object",
                json!(false),
            ),
            (
                "/children/0/queries/producer_joins/0/parent_outer_indices",
                json!([3]),
            ),
            (
                "/children/0/queries/producer_joins/0/parent_outer_indices",
                json!([1, 2]),
            ),
        ] {
            let mut changed = parent();
            *changed.pointer_mut(pointer).unwrap() = value;
            assert!(
                std::panic::catch_unwind(|| projected(changed)).is_err(),
                "{pointer}"
            );
        }
        let mut duplicate = parent();
        duplicate["parent_minion_modifiers"][1] = duplicate["parent_minion_modifiers"][0].clone();
        assert!(std::panic::catch_unwind(|| projected(duplicate)).is_err());
    }
}
