//! Pinned coefficient supplier closure; original construction, no copied evaluator.
use super::*;

const GAME_STAT: &str = "active_skill_has_%_standard_scaling_attack_damage";
const DIRECTORY: &str = "data/owned/poe2/3887ae68/intrinsic-attack-coefficient";
const SELECTION_SHA: &str = "a2daf1e5c6c058ae2ce32f434feee325d06344a96a5ccbf29ed7320df2c63ec3";

pub(super) fn extra_stat_factories(lua: &Lua, sources: &BTreeMap<String, String>) -> Result<Json> {
    let mut out = Vec::new();
    for (path, text) in sources {
        if !path.starts_with("src/") {
            continue;
        }
        let code = tokens(text)?;
        for (index, token) in code.iter().enumerate() {
            if !token.quoted
                || !matches!(token.text, "\"ExtraSkillStat\"" | "'ExtraSkillStat'")
                || !code
                    .get(index + 2)
                    .is_some_and(|t| matches!(t.text, "\"LIST\"" | "'LIST'"))
                || !code.get(index + 4).is_some_and(|t| t.text == "{")
            {
                continue;
            }
            let mut depth = 1;
            let mut key = None;
            for position in index + 5..code.len() {
                if !code[position].quoted {
                    match code[position].text {
                        "{" => depth += 1,
                        "}" => depth -= 1,
                        _ => {}
                    }
                }
                if depth == 0 {
                    break;
                }
                if depth == 1
                    && code[position].text == "key"
                    && code.get(position + 1).is_some_and(|t| t.text == "=")
                {
                    let literal = code
                        .get(position + 2)
                        .filter(|t| t.quoted)
                        .ok_or_else(|| error("computed ExtraSkillStat key requires review"))?;
                    if !code
                        .get(position + 3)
                        .is_some_and(|t| t.text == "," || t.text == "}")
                    {
                        return Err(error("computed ExtraSkillStat key requires review"));
                    }
                    key = Some(
                        lua.load(format!("return {}", literal.text))
                            .eval::<String>()?,
                    );
                }
            }
            out.push(json!({"path":path,"line":token.line,"key":key.ok_or_else(||error("ExtraSkillStat factory lacks literal key"))?}));
        }
    }
    Ok(json!(out))
}

fn capture_coefficient(sources: &BTreeMap<String, String>) -> Result<Json> {
    let (lua, result, _) = construct_catalog(sources)?;
    let data: Table = result.raw_get("data")?;
    let skills: Table = data.raw_get("skills")?;
    let (mut inventory, minions) = catalog_inventory(&lua, sources, &data, &result)?;
    // The visited inventory stores addresses for alias detection. This Minions
    // root is constructed separately from data and must stay alive while the
    // cache allocates new objects. Exercise that lifetime with a full collection.
    lua.gc_collect()?;
    // This is original immutable cache construction, not execution of parser
    // callbacks with guessed arguments. The fallback factory domain is below.
    let cache: Table = lua
        .load(source(sources, "src/Data/ModCache.lua")?)
        .set_name("@src/Data/ModCache.lua")
        .eval()?;
    inventory.visit(sources, "modCache", Value::Table(cache), 0)?;
    drop(minions);
    let mut coefficient_modifiers = Vec::new();
    let mut extra_stats = Vec::new();
    for (path, record) in &inventory.all_modifiers {
        let name: String = record.raw_get("name")?;
        if name == "SkillData" {
            let value: Table = record.raw_get("value")?;
            if value.raw_get::<String>("key")? == "baseMultiplier" {
                coefficient_modifiers
                    .push(json!({"path":path,"record":plain(Value::Table(record.clone()),0)?}));
            }
        } else if name == "ExtraSkillStat" {
            extra_stats.push(json!({"path":path,"record":plain(Value::Table(record.clone()),0)?}));
        }
    }
    let basic: Table = skills.raw_get(BASIC)?;
    let sets: Table = basic.raw_get("statSets")?;
    let mut selected_sets = BTreeMap::new();
    for pair in sets.pairs::<u32, Table>() {
        let (index, set) = pair?;
        let mut row = metadata(&set, &["levels", "stats", "constantStats", "baseMods"])?;
        row["direct_callbacks"] = direct_callbacks(sources, &set)?;
        selected_sets.insert(index, row);
    }
    let mut selected = metadata(
        &basic,
        &[
            "id",
            "levels",
            "qualityStats",
            "altQualityStats",
            "baseMods",
        ],
    )?;
    selected["stat_sets"] = json!(selected_sets);
    selected["direct_callbacks"] = direct_callbacks(sources, &basic)?;
    let static_inputs = static_inventory(&lua, sources)?;
    let coefficient_factories: Vec<_> = static_inputs["factory_keys"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["key"] == "baseMultiplier")
        .cloned()
        .collect();
    let coefficient_aliases: Vec<_> = inventory
        .aliases
        .iter()
        .filter(|row| {
            row["target"]
                .as_str()
                .unwrap()
                .starts_with(&format!("skillStatMap/{GAME_STAT}"))
        })
        .cloned()
        .collect();
    let files: BTreeMap<_, _> = [
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcOffence.lua",
        "src/Modules/CalcTools.lua",
        "src/Modules/Data.lua",
        "src/Modules/ModParser.lua",
        "src/Modules/ModTools.lua",
        "src/Modules/ConfigOptions.lua",
        "src/Classes/ConfigTab.lua",
        "src/Data/ModCache.lua",
        "src/Data/SkillStatMap.lua",
        "src/Data/Skills/minion.lua",
        "src/Data/Skills/sup_str.lua",
        "src/Data/Skills/sup_dex.lua",
    ]
    .into_iter()
    .map(|path| Ok((path, hash(source(sources, path)?.as_bytes()))))
    .collect::<Result<_>>()?;
    Ok(json!({
        "schema_version":1,"kind":"pinned-basic-attack-final-coefficient-domain",
        "upstream_revision":crate::source::UPSTREAM_REVISION,"source_manifest_sha256":crate::source::manifest_sha256(),
        "selection_proof":{"path":PACKET,"sha256":SELECTION_SHA},"files":files,
        "scope":{"exact_skill":"MinionMeleeBow","exact_profile":"RaisedSkeletonSniper","coefficient":1.0,"native_numerical_coverage":false,"normal_import_only":true,"lua_order_as_native_policy":false},
        "selected":selected,"coefficient_modifiers":coefficient_modifiers,"coefficient_factories":coefficient_factories,"coefficient_aliases":coefficient_aliases,
        "stat_consumers":stat_consumers(&skills,&BTreeSet::from([GAME_STAT.to_owned()]))?,
        "cached_extra_stats":extra_stats,"extra_stat_factories":extra_stat_factories(&lua,sources)?,
        "catalog_census":{"raw_tables":inventory.seen.len(),"raw_rows":inventory.rows,"modifiers":inventory.modifier_count},
        "selected_pass_corroboration":{"path":"data/owned/poe2/3887ae68/minion-preconversion-source/report.json","sha256":"7a723a4eece7b297737006b08d1dc61ec0047d13cfd2b9fbfeaee94eb52397cb","role":"corroboration-not-supplier-completeness"}
    }))
}

fn check_semantics(evidence: &Json) {
    assert_eq!(
        evidence["kind"],
        "pinned-basic-attack-final-coefficient-domain"
    );
    assert_eq!(
        evidence["scope"],
        json!({"exact_skill":"MinionMeleeBow","exact_profile":"RaisedSkeletonSniper","coefficient":1.0,"native_numerical_coverage":false,"normal_import_only":true,"lua_order_as_native_policy":false})
    );
    let selected = &evidence["selected"];
    assert_eq!(selected["id"], BASIC);
    assert_eq!(selected["levels"], json!({"[1]":{"levelRequirement":0}}));
    assert_eq!(selected["qualityStats"], json!({}));
    assert_eq!(selected["altQualityStats"], json!({}));
    assert_eq!(selected["baseMods"], Json::Null);
    assert_eq!(selected["direct_callbacks"], json!({}));
    let set = &selected["stat_sets"]["1"];
    assert_eq!(selected["stat_sets"].as_object().unwrap().len(), 1);
    assert_eq!(set["levels"], json!({"[1]":{"actorLevel":1}}));
    assert_eq!(set["direct_callbacks"], json!({}));
    assert!(set["baseMods"].is_null() || set["baseMods"] == json!({}));
    assert_eq!(
        evidence["coefficient_modifiers"],
        json!([{"path":format!("skillStatMap/{GAME_STAT}/[1]"),"record":{"name":"SkillData","type":"LIST","value":{"key":"baseMultiplier"},"flags":0,"keywordFlags":0}}])
    );
    assert_eq!(
        evidence["coefficient_factories"],
        json!([{"path":"src/Data/SkillStatMap.lua","line":2177,"key":"baseMultiplier","kind":"skill-helper"}])
    );
    assert_eq!(evidence["coefficient_aliases"], json!([]));
    let expected: BTreeMap<_, _> = [
        ("ArmourExplosionPlayer", 30),
        ("TriggeredBrambleslamPlayer", 35),
        ("TriggeredCraterPlayer", 35),
        ("StompingGroundShockwavePlayer", 50),
        ("TriggeredVolcanicEruptionPlayer", 20),
        ("TriggeredCaltropsPlayer", 75),
    ]
    .into_iter()
    .collect();
    let consumers = evidence["stat_consumers"].as_array().unwrap();
    assert_eq!(consumers.len(), expected.len());
    assert_eq!(
        consumers
            .iter()
            .map(|row| row["skill_id"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        expected.keys().copied().collect()
    );
    for row in consumers {
        assert_eq!(row["support"], Json::Null);
        assert_eq!(row["scope"], "statSets/1/constantStats");
        assert_eq!(row["stat"], GAME_STAT);
        let value = expected[row["skill_id"].as_str().unwrap()];
        assert_eq!(row["row"], json!({"[1]":GAME_STAT,"[2]":value}));
    }
    for row in evidence["extra_stat_factories"].as_array().unwrap() {
        assert_ne!(row["key"], GAME_STAT);
    }
    assert_eq!(
        evidence["extra_stat_factories"].as_array().unwrap().len(),
        14
    );
    for row in evidence["cached_extra_stats"].as_array().unwrap() {
        assert_ne!(row["record"]["value"]["key"], GAME_STAT);
    }
    assert_eq!(evidence["cached_extra_stats"].as_array().unwrap().len(), 3);
}

fn check(evidence: &Json) {
    check_semantics(evidence);
    assert_eq!(evidence["schema_version"], 1);
    assert_eq!(
        evidence["selection_proof"],
        json!({"path":PACKET,"sha256":SELECTION_SHA})
    );
    let selection_bytes = std::fs::read(root().join(PACKET)).unwrap();
    assert_eq!(hash(&selection_bytes), SELECTION_SHA);
    let selection: Json = serde_json::from_slice(&selection_bytes).unwrap();
    retained::check(&selection, true);
    assert_eq!(
        evidence["source_manifest_sha256"],
        selection["source_manifest_sha256"]
    );
    assert_eq!(
        evidence["upstream_revision"],
        selection["upstream_revision"]
    );
    for (path, value) in evidence["files"].as_object().unwrap() {
        assert_eq!(value, &selection["files"][path]);
    }
    let pin = &evidence["selected_pass_corroboration"];
    let bytes = std::fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
    assert_eq!(hash(&bytes), pin["sha256"]);
}

#[test]
#[ignore = "author reviewed final coefficient source domain"]
fn author_intrinsic_attack_coefficient_evidence() {
    let evidence = capture_coefficient(sources()).unwrap();
    check(&evidence);
    let directory = root().join(DIRECTORY);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("source-evidence.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
}

#[test]
fn final_coefficient_domain_survives_gc_and_matches_original_construction() {
    let evidence = capture_coefficient(sources()).unwrap();
    check(&evidence);
    let retained: Json = serde_json::from_slice(
        &std::fs::read(root().join(DIRECTORY).join("source-evidence.json")).unwrap(),
    )
    .unwrap();
    assert!(
        evidence == retained,
        "coefficient source domain differs from retained evidence"
    );
}

#[test]
fn actual_new_coefficient_sources_refuse_after_rehashing() {
    for (path, needle, replacement) in [
        (
            "src/Data/Skills/minion.lua",
            "skills[\"MinionMeleeBow\"] = {",
            "skills[\"MinionMeleeBow\"] = { preDamageFunc=function(activeSkill) activeSkill.skillData.baseMultiplier=0 end,",
        ),
        (
            "src/Data/Skills/minion.lua",
            "skills[\"GasShotSkeletonSniperMinion\"] = {",
            "skills[\"MinionMeleeBow\"].levels[1].baseMultiplier=0\nskills[\"GasShotSkeletonSniperMinion\"] = {",
        ),
        (
            "src/Data/Skills/minion.lua",
            "skills[\"GasShotSkeletonSniperMinion\"] = {",
            "skills[\"MinionMeleeBow\"].statSets[1].levels[1].baseMultiplier=0\nskills[\"GasShotSkeletonSniperMinion\"] = {",
        ),
        (
            "src/Data/Skills/minion.lua",
            "skills[\"GasShotSkeletonSniperMinion\"] = {",
            "skills[\"MinionMeleeBow\"].qualityStats={{\"active_skill_has_%_standard_scaling_attack_damage\",0}}\nskills[\"GasShotSkeletonSniperMinion\"] = {",
        ),
        (
            "src/Data/Skills/minion.lua",
            "skills[\"GasShotSkeletonSniperMinion\"] = {",
            "skills[\"MinionMeleeBow\"].statSets[1].baseMods={skill(\"baseMultiplier\",0,{type=\"Condition\",var=\"DormantSource\"})}\nskills[\"GasShotSkeletonSniperMinion\"] = {",
        ),
        (
            "src/Data/Skills/sup_int.lua",
            "{ \"support_last_gasp_duration_ms\", 4000 }",
            "{ \"active_skill_has_%_standard_scaling_attack_damage\", 0 }",
        ),
        (
            "src/Modules/ModParser.lua",
            "key = \"unique_blood_barrier_applies_x_stacks_of_corrupted_blood_on_block\"",
            "key = \"active_skill_has_%_standard_scaling_attack_damage\"",
        ),
    ] {
        let mut changed = sources().clone();
        let text = changed.get_mut(path).unwrap();
        assert!(text.contains(needle));
        *text = text.replacen(needle, replacement, 1);
        let evidence = capture_coefficient(&changed).unwrap();
        assert!(
            std::panic::catch_unwind(|| check_semantics(&evidence)).is_err(),
            "admitted changed supplier in {path}"
        );
    }
}
