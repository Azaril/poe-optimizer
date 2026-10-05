//! Bounded actual-parser applicability evidence; never a native mechanic or waiver.
use super::*;

const TEST: &str = "generated_extra_skill_stats_source::actual_custom_mod_skill_name_controls";
const CHILD: &str = "POE_GENERATED_EXTRA_STATS_CHILD";
const OUTPUT: &str = "POE_GENERATED_EXTRA_STATS_OUT";
const OBSERVER: &str = include_str!("generated_extra_skill_stats_source.lua");
const TITLE: &str = "ExtraSkillStat witness";
const PURIFYING: &str =
    "Consecrated Ground from Purifying Flame applies 17% increased Damage taken to Enemies";
const LIGHTNING: &str = "19% increased Lightning Trap Lightning Ailment Effect";
const PURIFYING_STAT: &str = "consecrated_ground_enemy_damage_taken_+%";
const LIGHTNING_STAT: &str = "shock_effect_+%";
const ITEM_EFFECT: &str = "BloodbarrierPlayer";
const ITEM_CACHED: &str = "Inflict Corrupted Blood for 5 seconds on Block, dealing 50% of your maximum Life as Physical damage per second";
const ITEM_UNCACHED: &str = "Inflict Corrupted Blood for 7.125 seconds on Block, dealing 53% of your maximum Life as Physical damage per second";
const ITEM_STATS: [&str; 3] = [
    "unique_blood_barrier_applies_x_stacks_of_corrupted_blood_on_block",
    "base_skill_effect_duration",
    "base_physical_damage_%_of_maximum_life_to_deal_per_minute",
];
const EFFECTS: [&str; 5] = [
    "SummonSandDjinnPlayer",
    "CommandSandDjinnKnifeThrowPlayer",
    "SummonWaterDjinnPlayer",
    "CommandWaterDjinnBubblePlayer",
    FIREBOLT,
];
const COMMANDS: [&str; 2] = [
    "CommandSandDjinnKnifeThrowPlayer",
    "CommandWaterDjinnBubblePlayer",
];

fn observe(lua: &Lua, phase: &str, item: &Json) -> Result<Json, RuntimeError> {
    let observer: Function = lua
        .load(OBSERVER)
        .set_name("@actual-extra-stat-source-observer")
        .eval()?;
    let value: Value = observer.call((
        lua.to_value(&EFFECTS)?,
        lua.to_value(&[PURIFYING_STAT, LIGHTNING_STAT])?,
        lua.to_value(&["Purifying Flame", "Lightning Trap", "Firebolt"])?,
        lua.to_value(&[PURIFYING, LIGHTNING])?,
        phase,
        lua.to_value(item)?,
    ))?;
    Ok(lua.from_value(value)?)
}
fn item_control(xml: &str, line: &str) -> (String, Json, std::ops::Range<usize>) {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc.descendants().find(|n| n.has_tag_name("Items")).unwrap();
    let active = items.attribute("activeItemSet").unwrap();
    let set = items
        .children()
        .find(|n| n.has_tag_name("ItemSet") && n.attribute("id") == Some(active))
        .unwrap();
    let slot = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some("Boots"))
        .unwrap();
    let id = slot.attribute("itemId").unwrap();
    assert_ne!(id, "0");
    assert_eq!(
        set.children()
            .filter(|n| n.has_tag_name("Slot") && n.attribute("itemId") == Some(id))
            .count(),
        1
    );
    let matches: Vec<_> = items
        .children()
        .filter(|n| n.has_tag_name("Item") && n.attribute("id") == Some(id))
        .collect();
    assert_eq!(matches.len(), 1);
    let item = matches[0];
    let text = item
        .children()
        .find(|n| n.is_text() && !n.text().unwrap().trim().is_empty())
        .unwrap();
    assert!(!text.text().unwrap().contains("Inflict Corrupted Blood"));
    let at = text.range().end;
    let inserted = format!("\n{line}\n");
    let mut result = xml.to_owned();
    result.insert_str(at, &inserted);
    let mut ids = EFFECTS.to_vec();
    ids.push(ITEM_EFFECT);
    let probe = json!({"item_id":id.parse::<u64>().unwrap(),"slot":"Boots","effect":ITEM_EFFECT,
        "effects":ids,"stat_keys":ITEM_STATS,"control_lines":[ITEM_CACHED,ITEM_UNCACHED],
        "scope":"diagnostic item-text mutation; no obtainable-item or game-valid-build claim"});
    (result, probe, at..at + inserted.len())
}
fn custom(xml: &str, line: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let config = doc
        .descendants()
        .find(|n| n.has_tag_name("Config"))
        .unwrap();
    let active = config.attribute("activeConfigSet").unwrap();
    let set = config
        .children()
        .find(|n| n.has_tag_name("ConfigSet") && n.attribute("id") == Some(active))
        .unwrap();
    assert!(
        !set.children()
            .any(|n| n.has_tag_name("CustomModifierBlock"))
    );
    let insertion = set.range().end - "</ConfigSet>".len();
    assert_eq!(&xml[insertion..set.range().end], "</ConfigSet>");
    let mut result = xml.to_owned();
    result.insert_str(
        insertion,
        &format!(
            "<CustomModifierBlock title=\"{TITLE}\" enabled=\"true\">{}</CustomModifierBlock>",
            escape(line)
        ),
    );
    result
}
fn second_switch(xml: &str, value: &str) -> String {
    let mut result = xml.to_owned();
    for source in ["Tree:13289", "Tree:32705"] {
        let doc = roxmltree::Document::parse(&result).unwrap();
        let skills = doc
            .descendants()
            .find(|n| n.has_tag_name("Skills"))
            .unwrap();
        let active = skills.attribute("activeSkillSet").unwrap();
        let set = skills
            .children()
            .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some(active))
            .unwrap();
        let matches: Vec<_> = set
            .children()
            .filter(|n| n.has_tag_name("Skill") && n.attribute("source") == Some(source))
            .collect();
        assert_eq!(matches.len(), 1);
        let gem = matches[0]
            .children()
            .find(|n| n.has_tag_name("Gem"))
            .unwrap();
        result = change_attributes(&result, gem, &[("enableGlobal2", Some(value))]);
    }
    result
}

#[test]
#[ignore = "requires complete pinned PoB; bounded parser/applicability evidence only"]
fn actual_custom_mod_skill_name_controls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join(
        std::env::var_os(OUTPUT)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("runs/owned-generated-extra-skill-stats-source-03")),
    );
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    assert!(
        !out.exists(),
        "choose a fresh {OUTPUT}; immutable evidence exists: {}",
        out.display()
    );
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "extra-stat source deadline: {}\n{}",
                    path.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        fs::read(out.join("source-jit-off.json")).unwrap(),
        fs::read(out.join("source-jit-on.json")).unwrap(),
        "JIT modes must agree"
    );
}
fn run_child(root: &Path, out: &Path, enabled: bool) {
    let input = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&input).unwrap();
    let purifying = custom(&original, PURIFYING);
    let (cached, probe, _) = item_control(&original, ITEM_CACHED);
    let (uncached, other_probe, insertion) = item_control(&original, ITEM_UNCACHED);
    assert_eq!(probe, other_probe);
    let mut removed = uncached.clone();
    removed.replace_range(insertion, "");
    assert_eq!(
        removed, original,
        "only the controlled supplier line is removed"
    );
    let cases = [
        ("original-05", original.clone()),
        ("purifying-original-switches", purifying.clone()),
        ("purifying-global2-true", second_switch(&purifying, "true")),
        (
            "purifying-global2-false",
            second_switch(&purifying, "false"),
        ),
        ("lightning-original-switches", custom(&original, LIGHTNING)),
        ("item-cached-skill-id", cached),
        ("item-uncached-skill-id", uncached.clone()),
        ("item-removed-supplier", removed),
        ("repeat-item-uncached-skill-id", uncached),
        ("repeat-original-05", original.clone()),
    ];
    let mut results = vec![];
    for (name, xml) in cases {
        let cold = std::cell::RefCell::new(None);
        let data_ready = |lua: &Lua| -> Result<(), RuntimeError> {
            let first = observe(lua, "before_build", &probe)?;
            assert!(
                first == observe(lua, "before_build", &probe)?,
                "cold observer must be read-only"
            );
            *cold.borrow_mut() = Some(first);
            Ok(())
        };
        let stage = |lua: &Lua| -> Result<Json, RuntimeError> {
            let base = super::observe_stage(lua)?;
            lua.globals().set("extraStatBase", lua.to_value(&base)?)?;
            let first = observe(lua, "stage", &probe)?;
            assert!(
                first == observe(lua, "stage", &probe)?,
                "stage observer must be read-only"
            );
            assert!(
                base == super::observe_stage(lua)?,
                "observer changed exact source objects"
            );
            let mut result = base;
            result["extra_skill_stats"] = first;
            Ok(result)
        };
        let mut case = observe_case_with_stage_and_data_hook(
            root,
            name,
            &xml,
            enabled,
            &stage,
            Some(&data_ready),
        );
        case["before_build"] = cold.into_inner().unwrap();
        results.push(case);
    }
    let files = [
        "src/Modules/Data.lua",
        "src/Data/Gems.lua",
        "src/Data/Skills/other.lua",
        "src/Data/Skills/act_int.lua",
        "src/Data/SkillStatMap.lua",
        "src/Modules/ModParser.lua",
        "src/Data/ModCache.lua",
        "src/Modules/ModTools.lua",
        "src/Classes/ConfigTab.lua",
        "src/Classes/SkillsTab.lua",
        "src/Classes/Item.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModList.lua",
        "src/Classes/ModDB.lua",
        "src/Modules/CalcTools.lua",
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcSetup.lua",
        "src/HeadlessWrapper.lua",
        "src/Modules/Main.lua",
    ];
    let report = json!({"schema_version":3,"source_revision":pinned::UPSTREAM_REVISION,
        "evidence_view":"raw_source_observation",
        "determinism_contract":{
            "semantic_view":"extra_skill_stats_distinct_skill_data_keys_v1",
            "projected_leaf":"/cases/*/states/{fresh,rebuilt_once,rebuilt_twice}/extra_skill_stats/item_transport/modes/{MAIN,CALCS}/receivers/*/emitted_skill_data",
            "source_order":"CalcActiveSkill.lua:87 pairs(stats), mergeLevelMod:22-48, ModList.lua:29-30 append",
            "consumer":"CalcActiveSkill.lua:896-900, CalcSetup.lua:2220-2224 and CalcOffence.lua:738-743 process each distinct SkillData.value.key independently; admitted records have no merge field",
            "admission":"exactly one complete duration, PhysicalDot with PercentStat(Life,1), and debuff record for the controlled item receiver; unknown or duplicate keys refused",
            "raw_order_deterministic":false,"semantic_repeat_and_jit_bytes_exact":true,
            "all_other_fields_and_arrays_exact":true,"numeric_tolerance":0,
            "arithmetic_reordered":false,"native_exception":false},
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":digest(OBSERVER.as_bytes()),
        "original_sha256":digest(original.as_bytes()),"native_inventory_authority":false,
        "all_producer_scope_proved":false,"whole_build_parity":false,"business_wrappers":false,
        "purpose":"finite actual-parser SkillName and item SkillId producer/transport controls; no all-producer closure, native mechanic or reference exception",
        "item_probe":probe,"current_original_field_closure":false,"obtainable_item_claim":false,
        "gameplay_formula_authority":false,"on_block_or_stack_mechanics_proved":false,
        "unmapped_item_stat":ITEM_STATS[0],"unmapped_stat_disposition":"delivered payload with no observed local/global mapping; not certified inert",
        "effect_ids":EFFECTS,"control_lines":[PURIFYING,LIGHTNING],
        "files":files.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),"cases":results});
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(bytes.len() <= 64 * 1024 * 1024);
    fs::write(
        out.join(format!(
            "source-jit-{}.raw.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    assert_eq!(fs::read_to_string(input).unwrap(), original);
    check(&report);
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
    check_replays(&semantic);
}
fn effect<'a>(metadata: &'a Json, id: &str) -> &'a Json {
    let matches: Vec<_> = rows(metadata)
        .iter()
        .filter(|row| row["id"] == id)
        .collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}
fn actions<'a>(state: &'a Json, mode: &str, id: &str) -> Vec<&'a Json> {
    rows(&state["modes"][mode]["actions"])
        .iter()
        .filter(|row| row["effect"] == id)
        .collect()
}
fn check_lookups(state: &Json) {
    let lookups = rows(&state["lookups"]);
    assert_eq!(lookups.len(), 3);
    for lookup in lookups {
        let name = lookup["name"].as_str().unwrap();
        assert!(matches!(
            name,
            "Purifying Flame" | "Lightning Trap" | "Firebolt"
        ));
        assert_eq!(lookup["game_id"]["present"], name == "Firebolt");
    }
}
fn check_control_cache(state: &Json, selected: Option<&str>) {
    assert_eq!(state["parser_original"], true);
    assert_eq!(state["parser_public_cache_identity"], true);
    let entries = rows(&state["control_parser_cache"]);
    assert_eq!(entries.len(), 2);
    for (entry, line) in entries.iter().zip([PURIFYING, LIGHTNING]) {
        assert_eq!(entry["line"], line);
        let expected = if selected == Some(line) {
            let (stat, value, skill) = if line == PURIFYING {
                (PURIFYING_STAT, 17, "Purifying Flame")
            } else {
                (LIGHTNING_STAT, 19, "Lightning Trap")
            };
            json!({"present":true,"value":[[{
                "name":"ExtraSkillStat","type":"LIST","flags":0,"keywordFlags":0,
                "value":{"key":stat,"value":value},
                "1":{"type":"SkillName","skillName":skill,"includeTransfigured":true}
            }]]})
        } else {
            json!({"present":false})
        };
        assert_eq!(entry["entry"], expected, "exact control cache for {line}");
    }
}
fn expected_sources(id: &str) -> Vec<(u64, u64, Option<&'static str>)> {
    // The unchanged original contains both generated and authored Djinn copies.
    // Keep their exact source occurrences; a definition-only count conflates them.
    match id {
        "SummonSandDjinnPlayer" | "CommandSandDjinnKnifeThrowPlayer" => {
            vec![(208, 209, Some("Tree:13289")), (215, 216, None)]
        }
        "SummonWaterDjinnPlayer" | "CommandWaterDjinnBubblePlayer" => {
            vec![(226, 227, Some("Tree:32705")), (228, 229, None)]
        }
        FIREBOLT => vec![(243, 244, Some("Item:28:New Item, Ashen Staff"))],
        _ => panic!("unexpected effect"),
    }
}
fn item_values(name: &str) -> Option<(u64, u64)> {
    match name {
        "item-cached-skill-id" => Some((5000, 50)),
        "item-uncached-skill-id" | "repeat-item-uncached-skill-id" => Some((7125, 53)),
        _ => None,
    }
}
fn item_mods(duration: u64, damage: u64, source: Option<&str>) -> Vec<Json> {
    let mut result = vec![
        json!({"name":"ExtraSkill","type":"LIST","flags":0,"keywordFlags":0,
        "value":{"skillId":ITEM_EFFECT,"level":1,"noSupports":true,"name":"Bloodbarrier"}}),
    ];
    for (key, value) in ITEM_STATS.into_iter().zip([1, duration, damage]) {
        result.push(
            json!({"name":"ExtraSkillStat","type":"LIST","flags":0,"keywordFlags":0,
            "value":{"key":key,"value":value},"1":{"type":"SkillId","skillId":ITEM_EFFECT}}),
        );
    }
    if let Some(source) = source {
        for record in &mut result {
            record["source"] = json!(source);
        }
    }
    result
}
fn slotted_item_mods(duration: u64, damage: u64, source: &str, slot: &Json) -> Vec<Json> {
    // Item.lua:2439 (BuildModListForSlotNum) copies base records, then adds sourceSlot.
    // Keep that exact transport metadata distinct from the parser/base records.
    let mut result = item_mods(duration, damage, Some(source));
    for record in &mut result {
        record["sourceSlot"] = slot.clone();
    }
    result
}
fn check_item_cache(state: &Json, uncached: bool) {
    assert_eq!(state["item_methods_original"], true);
    let entries = rows(&state["item_parser_cache"]);
    assert_eq!(entries.len(), 2);
    for (index, (entry, line)) in entries.iter().zip([ITEM_CACHED, ITEM_UNCACHED]).enumerate() {
        assert_eq!(entry["line"], line);
        let expected = if index == 0 || uncached {
            let (duration, damage) = if index == 0 { (5000, 50) } else { (7125, 53) };
            json!({"present":true,"value":[item_mods(duration,damage,None)]})
        } else {
            json!({"present":false})
        };
        assert_eq!(entry["entry"], expected, "exact hit/miss item parser cache");
    }
}
fn item_records(chain: &Json, source: &str) -> Vec<Json> {
    let mut result = vec![];
    let nodes = rows(chain);
    assert!(nodes.len() <= 32);
    for (index, node) in nodes.iter().enumerate() {
        assert_eq!(node["depth"], index);
        assert_eq!(node["has_parent"], index + 1 < nodes.len());
        if index + 1 < nodes.len() {
            assert_eq!(node["parent_kind"], "store");
        } else {
            assert!(matches!(
                node["parent_kind"].as_str(),
                Some("absent" | "false_sentinel")
            ));
        }
        if node["parent_is_player_mod_db"] == true {
            assert_eq!(nodes[index + 1]["is_player_mod_db"], true);
        }
        for row in rows(&node["records"]) {
            let record = &row["record"];
            if record["source"] == source {
                result.push(record.clone());
            }
        }
    }
    // ModDB stores names in separate buckets; source sequence order is not an
    // inter-bucket semantic. Keep the complete exact records, multiplicity and
    // their original store positions in the report, sorting only this comparison.
    result.sort_by_key(|v| serde_json::to_string(v).unwrap());
    result
}
fn check_item(c: &Json, probe: &Json) {
    let name = c["name"].as_str().unwrap();
    let amounts = item_values(name);
    check_item_cache(&c["before_build"], false);
    let cold = effect(&c["before_build"]["item_metadata"], ITEM_EFFECT);
    assert_eq!(cold["has_global_effect"]["present"], false);
    for stage in STAGES {
        let state = &c["states"][stage]["extra_skill_stats"];
        check_item_cache(state, amounts.is_some_and(|(_, damage)| damage == 53));
        let item = &state["item_transport"];
        assert_eq!(item["item_id"], probe["item_id"]);
        assert_eq!(item["slot"], probe["slot"]);
        let source = item["mod_source"].as_str().unwrap();
        assert!(source.starts_with(&format!("Item:{}:", probe["item_id"])));
        let mut base_expected = amounts
            .map(|(duration, damage)| item_mods(duration, damage, Some(source)))
            .unwrap_or_default();
        base_expected.sort_by_key(|v| serde_json::to_string(v).unwrap());
        let mut expected = amounts
            .map(|(duration, damage)| slotted_item_mods(duration, damage, source, &probe["slot"]))
            .unwrap_or_default();
        expected.sort_by_key(|v| serde_json::to_string(v).unwrap());
        for (field, records) in [("item_base", &base_expected), ("item_active", &expected)] {
            assert_eq!(
                &item_records(&item[field], source),
                records,
                "{name}/{stage}/{field}"
            );
        }
        assert_eq!(rows(&item["item_base"])[0]["is_item_base"], true);
        assert_eq!(rows(&item["item_active"])[0]["is_item_active"], true);
        assert_eq!(rows(&item["item_base"])[0]["parent_kind"], "false_sentinel");
        assert_eq!(rows(&item["item_active"])[0]["parent_kind"], "absent");
        let lines = rows(&item["lines"]);
        assert_eq!(lines.len(), usize::from(amounts.is_some()));
        if let Some((duration, damage)) = amounts {
            assert_eq!(
                lines[0]["line"],
                if damage == 50 {
                    ITEM_CACHED
                } else {
                    ITEM_UNCACHED
                }
            );
            assert_eq!(lines[0]["extra"]["present"], false);
            assert_eq!(lines[0]["disabled"]["present"], false);
            assert_eq!(
                lines[0]["records"],
                json!(item_mods(duration, damage, Some(source)))
            );
        }
        assert_eq!(
            rows(&item["item_grants"]).len(),
            usize::from(amounts.is_some())
        );
        for mode in ["MAIN", "CALCS"] {
            let transport = &item["modes"][mode];
            assert_eq!(transport["equipped_item_exact"], true);
            for field in ["item_store", "player_store"] {
                assert_eq!(
                    item_records(&transport[field], source),
                    expected,
                    "{name}/{stage}/{mode}/{field}"
                );
            }
            assert_eq!(rows(&transport["item_store"])[0]["is_item_mod_db"], true);
            assert_eq!(
                rows(&transport["item_store"])[0]["parent_kind"],
                "false_sentinel"
            );
            assert_eq!(
                rows(&transport["player_store"])[0]["is_player_mod_db"],
                true
            );
            let grants = rows(&transport["grants"]);
            let receivers = rows(&transport["receivers"]);
            assert_eq!(grants.len(), usize::from(amounts.is_some()));
            assert_eq!(receivers.len(), usize::from(amounts.is_some()));
            if let Some((duration, damage)) = amounts {
                let grant = &grants[0];
                assert_eq!(grant["source_item_exact"], true);
                assert_eq!(grant["source_node_absent"], true);
                assert_eq!(grant["fields"]["skillId"], ITEM_EFFECT);
                assert_eq!(grant["fields"]["source"], source);
                assert_eq!(grant["fields"]["slotName"], probe["slot"]);
                assert_eq!(rows(&grant["groups"]).len(), 1);
                let group = &rows(&grant["groups"])[0];
                assert_eq!(group["source_item_exact"], true);
                assert_eq!(group["source_node_absent"], true);
                assert_eq!(group["saved_group_present"], false);
                assert_eq!(group["saved_source_ordinal"], json!({"present":false}));
                let receiver = &receivers[0];
                for field in [
                    "cfg_effect_exact",
                    "catalogue_effect_exact",
                    "actor_exact",
                    "group_source_item_exact",
                    "source_instance_exact",
                ] {
                    assert_eq!(receiver[field], true, "{name}/{stage}/{mode}/{field}");
                }
                assert_eq!(receiver["effect"], ITEM_EFFECT);
                assert_eq!(receiver["group_fields"], group["group_fields"]);
                assert_eq!(receiver["gem_fields"], group["gem_fields"]);
                assert_eq!(receiver["gem_fields"]["enableGlobal1"], true);
                assert_eq!(item_records(&receiver["store_chain"], source), expected);
                assert_eq!(
                    rows(&receiver["store_chain"])
                        .iter()
                        .filter(|n| n["is_player_mod_db"] == true)
                        .count(),
                    1
                );
                let expected_payloads: Vec<_> = ITEM_STATS
                    .into_iter()
                    .zip([1, duration, damage])
                    .map(|(key, value)| json!({"key":key,"value":value}))
                    .collect();
                assert_eq!(receiver["extra_stats"], json!(expected_payloads));
                // These existing mapped SkillData records prove the original
                // merge consumed the payloads independently of our List query.
                keyed_skill_data(&receiver["emitted_skill_data"], duration, damage);
            }
        }
        let metadata = effect(&state["item_metadata"], ITEM_EFFECT);
        for part in rows(&metadata["parts"]) {
            let stack = &rows(&part["maps"])[0];
            assert_eq!(stack["stat"], ITEM_STATS[0]);
            assert_eq!(stack["local_entry"]["present"], false);
            assert_eq!(stack["global_entry"]["present"], false);
        }
        if amounts.is_some() {
            let set = &rows(&metadata["parts"])[1];
            assert_eq!(rows(&set["maps"])[1]["local_entry"]["present"], true);
            assert_eq!(rows(&set["maps"])[2]["local_entry"]["present"], true);
        }
    }
}
fn keyed_skill_data(raw: &Json, duration: u64, damage: u64) -> Json {
    let records = raw.as_array().expect("exact emitted SkillData sequence");
    assert_eq!(records.len(), 3, "exact controlled mapped and base records");
    let mut keyed = serde_json::Map::new();
    for record in records {
        let key = record["value"]["key"].as_str().unwrap();
        let value = match key {
            "duration" if duration.is_multiple_of(1000) => json!(duration / 1000),
            "duration" => json!(duration as f64 / 1000.0),
            "PhysicalDot" => json!(damage),
            "debuff" => json!(true),
            _ => panic!("unreviewed emitted SkillData key: {key}"),
        };
        let mut expected = json!({"name":"SkillData","type":"LIST","flags":0,"keywordFlags":0,
            "source":format!("Skill:{ITEM_EFFECT}"),"value":{"key":key,"value":value}});
        if key == "PhysicalDot" {
            // Local Bloodbarrier map; not the different global PhysicalDegen map.
            expected["1"] = json!({"type":"PercentStat","stat":"Life","percent":1});
        }
        assert_eq!(record, &expected, "complete emitted {key} record");
        assert!(
            keyed.insert(key.to_owned(), record.clone()).is_none(),
            "duplicate emitted key"
        );
    }
    assert!(
        keyed.contains_key("duration")
            && keyed.contains_key("PhysicalDot")
            && keyed.contains_key("debuff")
    );
    Json::Object(keyed)
}
fn semantic_report(mut report: Json) -> Json {
    assert_eq!(report["evidence_view"], "raw_source_observation");
    for case in report["cases"].as_array_mut().unwrap() {
        let Some((duration, damage)) = item_values(case["name"].as_str().unwrap()) else {
            continue;
        };
        for stage in STAGES {
            for mode in ["MAIN", "CALCS"] {
                let receivers = case["states"][stage]["extra_skill_stats"]["item_transport"]["modes"][mode]["receivers"]
                    .as_array_mut().unwrap();
                assert_eq!(receivers.len(), 1);
                let receiver = &mut receivers[0];
                assert_eq!(receiver["effect"], ITEM_EFFECT);
                receiver["emitted_skill_data"] =
                    keyed_skill_data(&receiver["emitted_skill_data"], duration, damage);
            }
        }
    }
    report["evidence_view"] = json!("extra_skill_stats_distinct_skill_data_keys_v1");
    report
}
fn check_replays(report: &Json) {
    assert!(case(report, "original-05")["states"] == case(report, "repeat-original-05")["states"]);
    assert!(
        case(report, "original-05")["states"] == case(report, "item-removed-supplier")["states"]
    );
    assert_eq!(
        case(report, "original-05")["xml_sha256"],
        case(report, "item-removed-supplier")["xml_sha256"]
    );
    assert!(
        case(report, "item-uncached-skill-id")["states"]
            == case(report, "repeat-item-uncached-skill-id")["states"]
    );
}
fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 10);
    for c in rows(&report["cases"]) {
        let name = c["name"].as_str().unwrap();
        let is_purifying = name.starts_with("purifying");
        let is_lightning = name.starts_with("lightning");
        let control = is_purifying || is_lightning;
        let item_active = item_values(name).is_some();
        check_item(c, &report["item_probe"]);
        check_lookups(&c["before_build"]);
        // This hook runs after full Main/cache initialization, before XML load.
        // The two literal lines must take the real parser miss path.
        check_control_cache(&c["before_build"], None);
        for id in EFFECTS {
            let cold = effect(&c["before_build"]["metadata"], id);
            assert_eq!(cold["has_global_effect"]["present"], false);
            assert_eq!(
                cold["game_id"]["present"],
                !COMMANDS.contains(&id),
                "cold name resolution {id}"
            );
        }
        for stage in STAGES {
            let state = &c["states"][stage]["extra_skill_stats"];
            assert_eq!(state["native_inventory_authority"], false);
            assert_eq!(state["extra_stat_scope_proved"], false);
            assert_eq!(state["outputs_preserved"], true);
            check_lookups(state);
            check_control_cache(
                state,
                if is_purifying {
                    Some(PURIFYING)
                } else if is_lightning {
                    Some(LIGHTNING)
                } else {
                    None
                },
            );
            for id in EFFECTS {
                let row = effect(&state["metadata"], id);
                let command = COMMANDS.contains(&id);
                assert_eq!(row["game_id"]["present"], !command);
                assert_eq!(
                    row["has_global_effect"]["present"], false,
                    "{name}/{stage}/{id}"
                );
                for part in rows(&row["parts"]) {
                    for map in rows(&part["maps"]) {
                        assert_eq!(
                            map["local_entry"]["present"], false,
                            "excluded custom stat must not initialize lazy map: {name}/{stage}/{id}"
                        );
                    }
                }
            }
            for mode in ["MAIN", "CALCS"] {
                let mods = rows(&state["modes"][mode]["player_records"]);
                assert_eq!(
                    mods.len(),
                    if item_active { 3 } else { usize::from(control) },
                    "actual parser-produced player records {name}/{stage}/{mode}"
                );
                if control {
                    let (stat, amount, skill) = if is_purifying {
                        (PURIFYING_STAT, 17, "Purifying Flame")
                    } else {
                        (LIGHTNING_STAT, 19, "Lightning Trap")
                    };
                    assert_eq!(mods[0]["name"], "ExtraSkillStat");
                    assert_eq!(mods[0]["type"], "LIST");
                    assert_eq!(mods[0]["source"], format!("Custom:{TITLE}"));
                    assert_eq!(mods[0]["value"], json!({"key":stat,"value":amount}));
                    assert_eq!(
                        mods[0]["1"],
                        json!({"type":"SkillName","skillName":skill,"includeTransfigured":true})
                    );
                    assert_eq!(mods[0]["flags"], 0);
                    assert_eq!(mods[0]["keywordFlags"], 0);
                }
                for id in EFFECTS {
                    let observed = actions(state, mode, id);
                    let command = COMMANDS.contains(&id);
                    let actual_sources: Vec<_> = observed
                        .iter()
                        .map(|action| {
                            (
                                action["source_ordinal"].as_u64().unwrap(),
                                action["gem_source_ordinal"].as_u64().unwrap(),
                                action["source"].as_str(),
                            )
                        })
                        .collect();
                    assert_eq!(
                        actual_sources,
                        expected_sources(id),
                        "{name}/{stage}/{mode}/{id}"
                    );
                    for action in observed {
                        assert_eq!(action["cfg_effect_exact"], true);
                        let item_source = state["item_transport"]["mod_source"].as_str().unwrap();
                        let mut supplied = item_values(name)
                            .map(|(d, p)| {
                                slotted_item_mods(d, p, item_source, &report["item_probe"]["slot"])
                            })
                            .unwrap_or_default();
                        supplied.sort_by_key(|v| serde_json::to_string(v).unwrap());
                        assert_eq!(item_records(&action["store_chain"], item_source), supplied);
                        assert_eq!(
                            rows(&action["store_chain"])
                                .iter()
                                .filter(|n| n["is_player_mod_db"] == true)
                                .count(),
                            1
                        );
                        assert_eq!(action["cfg"]["game_id"]["present"], !command);
                        if command {
                            assert_eq!(action["cfg"]["include_transfigured_match_game_id"], "");
                        } else {
                            assert_eq!(
                                action["cfg"]["include_transfigured_match_game_id"],
                                action["cfg"]["game_id"]["value"]
                            );
                        }
                        let stats = rows(&action["extra_stats"]);
                        assert!(
                            stats.is_empty(),
                            "actual source excludes unrelated SkillName: {name}/{stage}/{mode}/{id}"
                        );
                    }
                }
            }
            for group in rows(&state["groups"]) {
                for row in rows(&group["effects"]) {
                    if COMMANDS.iter().any(|id| row["id"] == *id) {
                        assert_eq!(
                            row["global_value"],
                            json!({"present":true,"value":group["source"].is_null() || name=="purifying-global2-true"})
                        );
                    }
                }
            }
            let caches = rows(&state["parser_cache"]);
            assert_eq!(caches.len(), usize::from(control));
            if control {
                assert_eq!(
                    caches[0]["line"],
                    if is_purifying { PURIFYING } else { LIGHTNING }
                );
                assert_eq!(caches[0]["entry"]["present"], true);
            }
        }
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    fn records() -> Json {
        json!([
            {"name":"SkillData","type":"LIST","flags":0,"keywordFlags":0,
                "source":"Skill:BloodbarrierPlayer","value":{"key":"duration","value":7.125}},
            {"name":"SkillData","type":"LIST","flags":0,"keywordFlags":0,
                "source":"Skill:BloodbarrierPlayer","value":{"key":"PhysicalDot","value":53},
                "1":{"type":"PercentStat","stat":"Life","percent":1}},
            {"name":"SkillData","type":"LIST","flags":0,"keywordFlags":0,
                "source":"Skill:BloodbarrierPlayer","value":{"key":"debuff","value":true}}
        ])
    }

    fn report() -> Json {
        let mut states = serde_json::Map::new();
        for stage in STAGES {
            let mut modes = serde_json::Map::new();
            for mode in ["MAIN", "CALCS"] {
                modes.insert(
                    mode.into(),
                    json!({"receivers":[{
                        "effect":ITEM_EFFECT,"emitted_skill_data":records(),
                        "cfg":{"flags":1},"skill_data":{"duration":7.125,"PhysicalDot":0},
                        "store_chain":[{"depth":0},{"depth":1}],"arithmetic":[53,1]
                    }]}),
                );
            }
            states.insert(
                stage.into(),
                json!({"outputs":{"DPS":123},
                "extra_skill_stats":{"item_transport":{"modes":modes}}}),
            );
        }
        json!({"evidence_view":"raw_source_observation","untouched":[2,1],
            "cases":[{"name":"item-uncached-skill-id","states":states}]})
    }

    const RECEIVER: &str =
        "/cases/0/states/fresh/extra_skill_stats/item_transport/modes/MAIN/receivers/0";

    #[test]
    fn distinct_emitted_keys_preserve_all_other_order_and_values() {
        let original = report();
        let mut swapped = original.clone();
        swapped
            .pointer_mut(&format!("{RECEIVER}/emitted_skill_data"))
            .unwrap()
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        assert_ne!(original, swapped);
        let expected = semantic_report(original.clone());
        assert_eq!(
            serde_json::to_vec(&expected).unwrap(),
            serde_json::to_vec(&semantic_report(swapped)).unwrap()
        );
        for (suffix, value) in [
            ("/skill_data/duration", json!(7.126)),
            ("/cfg/flags", json!(2)),
            ("/store_chain", json!([{"depth":1},{"depth":0}])),
            ("/arithmetic", json!([1, 53])),
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(&format!("{RECEIVER}{suffix}")).unwrap() = value;
            assert_ne!(expected, semantic_report(changed), "{suffix}");
        }
        assert_eq!(expected["untouched"], original["untouched"]);
        assert_eq!(
            expected["cases"][0]["states"]["fresh"]["outputs"],
            original["cases"][0]["states"]["fresh"]["outputs"]
        );
    }

    #[test]
    fn unknown_duplicate_or_changed_emitted_records_are_refused() {
        for (pointer, value) in [
            ("/0/value/key", json!("unknown")),
            ("/0/value/value", json!(7.126)),
            ("/1/1/percent", json!(2)),
            ("/1/source", json!("Skill:other")),
        ] {
            let mut changed = records();
            *changed.pointer_mut(pointer).unwrap() = value;
            assert!(
                std::panic::catch_unwind(|| keyed_skill_data(&changed, 7125, 53)).is_err(),
                "{pointer}"
            );
        }
        let mut duplicate = records();
        duplicate[2] = duplicate[0].clone();
        assert!(std::panic::catch_unwind(|| keyed_skill_data(&duplicate, 7125, 53)).is_err());
        let mut merge = records();
        merge[0]["value"]["merge"] = json!("MAX");
        assert!(std::panic::catch_unwind(|| keyed_skill_data(&merge, 7125, 53)).is_err());
        let mut untagged = records();
        untagged[1].as_object_mut().unwrap().remove("1");
        assert!(std::panic::catch_unwind(|| keyed_skill_data(&untagged, 7125, 53)).is_err());
        for malformed in [json!({}), json!([]), json!(null)] {
            assert!(std::panic::catch_unwind(|| keyed_skill_data(&malformed, 7125, 53)).is_err());
        }
    }
}
