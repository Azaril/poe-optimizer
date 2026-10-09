//! Original constructor census for the exact enemy flat-physical channels.
//! No source methods are replaced and no native calculation is authored here.
use super::*;
#[path = "../../../tests/support/owned_enemy_flat_damage_evidence.rs"]
mod evidence;
const ENEMY_PACKET: &str = "data/owned/poe2/3887ae68/enemy-flat-attack-damage/source-evidence.json";
const CHANNELS: [&str; 2] = ["SelfPhysicalMin", "SelfPhysicalMax"];

fn fingerprint(value: &Json) -> String {
    hash(&serde_json::to_vec(value).unwrap())
}

// Record all computed modifier-name arguments, rather than searching only for
// the target strings. Original constructors still evaluate every data value.
fn name_ledger(lua: &Lua, sources: &BTreeMap<String, String>) -> Result<Json> {
    let mut dynamic = Vec::new();
    let mut self_literals = Vec::new();
    let mut writes = Vec::new();
    for (path, text) in sources {
        if !(path.starts_with("src/Modules/")
            || path.starts_with("src/Classes/")
            || path.starts_with("src/Data/"))
        {
            continue;
        }
        let code = tokens(text)?;
        for (i, token) in code.iter().enumerate() {
            if token.quoted && token.text.contains("Self") {
                let value: String = lua.load(format!("return {}", token.text)).eval()?;
                if value == "Self" || CHANNELS.contains(&value.as_str()) {
                    self_literals.push(json!({"path":path,"line":token.line,"value":value}));
                }
            }
            if !token.quoted
                && [
                    "mod",
                    "flag",
                    "NewMod",
                    "MergeNewMod",
                    "ReplaceMod",
                    "createMod",
                    "mod_createMod",
                ]
                .contains(&token.text)
                && code.get(i + 1).is_some_and(|t| t.text == "(")
            {
                let mut depth = 0usize;
                let mut end = None;
                for (j, t) in code.iter().enumerate().skip(i + 2) {
                    if !t.quoted {
                        if depth == 0 && [",", ")"].contains(&t.text) {
                            end = Some(j);
                            break;
                        }
                        if ["(", "{", "["].contains(&t.text) {
                            depth += 1;
                        }
                        if [")", "}", "]"].contains(&t.text) {
                            depth = depth
                                .checked_sub(1)
                                .ok_or_else(|| error("unbalanced factory argument"))?;
                        }
                    }
                }
                let end = end.ok_or_else(|| error("unterminated modifier factory"))?;
                let argument = &code[i + 2..end];
                if !(argument.len() == 1 && argument[0].quoted) {
                    dynamic.push(json!({"path":path,"line":token.line,"factory":token.text,"argument":argument.iter().map(|t|t.text).collect::<Vec<_>>()}));
                }
            }
            // Includes record constructors and assignments. The full source file
            // authenticates multi-line bodies; the row is a review navigation aid.
            if !token.quoted
                && token.text == "name"
                && code.get(i + 1).is_some_and(|t| t.text == "=")
                && !code.get(i + 2).is_some_and(|t| t.text == "=")
                && (path.starts_with("src/Modules/") || path.starts_with("src/Classes/"))
            {
                let line = text.lines().nth(token.line - 1).unwrap();
                writes.push(json!({"path":path,"line":token.line,"text":line.trim()}));
            }
        }
    }
    Ok(json!({"dynamic_factories":dynamic,"self_literals":self_literals,"name_writes":writes}))
}

fn capture_enemy(sources: &BTreeMap<String, String>) -> Result<Json> {
    let (lua, result, module_order) = construct_catalog(sources)?;
    let data: Table = result.get("data")?;
    let (mut inventory, minions) = catalog_inventory(&lua, sources, &data, &result)?;
    let base = json!({"raw_tables":inventory.seen.len(),"raw_rows":inventory.rows,
        "aliases_sha256":fingerprint(&json!(inventory.aliases)),"callbacks_sha256":fingerprint(&json!(inventory.callbacks)),
        "metatables_sha256":fingerprint(&json!(inventory.metatables))});
    // Keep every visited root alive while pointer identities are in `seen`.
    // A forced collection makes lifetime mistakes observable before new tables.
    lua.gc_collect()?;
    let cache: Table = lua
        .load(source(sources, "src/Data/ModCache.lua")?)
        .set_name("@src/Data/ModCache.lua")
        .eval()?;
    inventory.visit(sources, "modCache", Value::Table(cache.clone()), 0)?;
    let cache_census = json!({"raw_tables":inventory.seen.len(),"raw_rows":inventory.rows,"modifiers":inventory.modifier_count,
        "aliases_sha256":fingerprint(&json!(inventory.aliases))});
    let boss: Table = lua
        .load(source(sources, "src/Data/BossSkills.lua")?)
        .set_name("@src/Data/BossSkills.lua")
        .eval()?;
    let maps: Table = lua
        .load(source(sources, "src/Data/ModMap.lua")?)
        .set_name("@src/Data/ModMap.lua")
        .eval()?;
    inventory.visit(sources, "bossSkills", Value::Table(boss.clone()), 0)?;
    inventory.visit(sources, "mapMods", Value::Table(maps.clone()), 0)?;
    let mut boss_additional = Vec::new();
    let bosses: Table = boss.raw_get("bossSkills")?;
    let mut ordered = BTreeMap::new();
    for pair in bosses.pairs::<String, Table>() {
        let (id, row) = pair?;
        ordered.insert(id, row);
    }
    for (id, row) in ordered {
        if let Some(additional) = row.raw_get::<Option<Table>>("additionalStats")? {
            for mode in ["base", "uber"] {
                if let Some(fields) = additional.raw_get::<Option<Table>>(mode)? {
                    let value = plain(Value::Table(fields), 0)?;
                    boss_additional.push(json!({"boss":id,"mode":mode,"stats":value}));
                }
            }
        }
    }
    let mut targets = Vec::new();
    for (path, table) in &inventory.all_modifiers {
        if CHANNELS.contains(&required_text(table, "name")?.as_str()) {
            targets.push(json!({"path":path,"record":plain(Value::Table(table.clone()),0)?}));
        }
    }
    let self_names: BTreeMap<_, _> = inventory
        .modifier_name_counts
        .iter()
        .filter(|(name, _)| name.starts_with("Self"))
        .collect();
    let selection = capture(sources)?;
    let ledger = name_ledger(&lua, sources)?;
    let profile: Table = minions.raw_get(PROFILE)?;
    let files: BTreeMap<_, _> = sources
        .iter()
        .map(|(path, text)| (path, hash(text.as_bytes())))
        .collect();
    let actual = json!({"schema_version":1,"kind":"pinned-constructed-enemy-flat-physical-domain",
        "upstream_revision":crate::source::UPSTREAM_REVISION,"source_manifest_sha256":crate::source::manifest_sha256(),
        "scope":{"channels":CHANNELS,"exact_intrinsic_basic_source":true,"hostile_minions_admitted":false,
            "imported_party_admitted":false,"requires_accounted_inputs":true,"native_runtime_or_coverage_change":false},
        "dependencies":evidence::dependencies(),"source_files_sha256":fingerprint(&json!(files)),"module_order":module_order,
        "catalog":base,"cache_census":cache_census,
        "expanded_census":{"raw_tables":inventory.seen.len(),"raw_rows":inventory.rows,"modifiers":inventory.modifier_count,
            "callbacks":inventory.callbacks,"self_modifier_names":self_names},
        "target_modifiers":targets,"boss_additional_stats":boss_additional,
        "selected":selection["catalog"]["selected"],"full_profile":plain(Value::Table(profile),0)?,
        "name_ledger":ledger,"name_ledger_sha256":fingerprint(&ledger),
        "corroboration":selection["selected_pass_corroboration"]});
    drop((minions, cache, boss, maps));
    Ok(actual)
}

#[test]
#[ignore = "authors the bounded original enemy-flat source census"]
fn author_enemy_flat_damage_domain() {
    let actual = capture_enemy(sources()).unwrap();
    // Bounded diagnostic retained before admission; never a published proof.
    let diagnostic = root().join("runs/owned-enemy-flat-source-candidate.json");
    std::fs::write(diagnostic, serde_json::to_vec_pretty(&actual).unwrap()).unwrap();
    evidence::check(&actual, true);
    let path = root().join(ENEMY_PACKET);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&actual).unwrap()).unwrap();
}
#[test]
fn original_enemy_flat_census_matches_retained_proof() {
    let actual = capture_enemy(sources()).unwrap();
    evidence::check(&actual, true);
    let retained: Json =
        serde_json::from_slice(&std::fs::read(root().join(ENEMY_PACKET)).unwrap()).unwrap();
    assert!(actual == retained, "original enemy-flat census changed");
}
#[test]
fn real_zero_nested_dynamic_and_boss_writers_are_seen_before_refusal() {
    for (path, needle, replacement) in [
        (
            "src/Data/Skills/minion.lua",
            "skills[\"GasShotSkeletonSniperMinion\"] = {",
            "skills[\"MinionMeleeBow\"].baseMods={mod(\"SelfPhysicalMin\",\"BASE\",0)}\nskills[\"GasShotSkeletonSniperMinion\"] = {",
        ),
        (
            "src/Data/Skills/minion.lua",
            "skills[\"GasShotSkeletonSniperMinion\"] = {",
            "skills[\"MinionMeleeBow\"].baseMods={mod(\"SelfPhysicalMax\",\"BASE\",7)}\nskills[\"GasShotSkeletonSniperMinion\"] = {",
        ),
        (
            "src/Data/Skills/minion.lua",
            "skills[\"GasShotSkeletonSniperMinion\"] = {",
            "skills[\"MinionMeleeBow\"].baseMods={mod(\"SelfPhysicalMin\",\"BASE\",0,0,0,{type=\"Condition\",var=\"Unknown\"})}\nskills[\"GasShotSkeletonSniperMinion\"] = {",
        ),
        (
            "src/Data/Skills/minion.lua",
            "skills[\"GasShotSkeletonSniperMinion\"] = {",
            "skills[\"MinionMeleeBow\"].baseMods={mod(\"EnemyModifier\",\"LIST\",{mod=mod(\"Self\"..\"Physical\"..\"Max\",\"BASE\",0)})}\nskills[\"GasShotSkeletonSniperMinion\"] = {",
        ),
        (
            "src/Data/Skills/minion.lua",
            "skills[\"GasShotSkeletonSniperMinion\"] = {",
            "skills[\"MinionMeleeBow\"].baseMods={}; for _, endpoint in ipairs({\"Min\",\"Max\"}) do table.insert(skills[\"MinionMeleeBow\"].baseMods,mod(\"SelfPhysical\"..endpoint,\"BASE\",0)) end\nskills[\"GasShotSkeletonSniperMinion\"] = {",
        ),
        (
            "src/Data/Minions.lua",
            "minions[\"RaisedSkeletonBrute\"] = {",
            "minions[\"RaisedSkeletonSniper\"].modList={mod(\"EnemyModifier\",\"LIST\",{mod=mod(\"SelfPhysicalMin\",\"BASE\",-3)})}\nminions[\"RaisedSkeletonBrute\"] = {",
        ),
        (
            "src/Data/BossSkills.lua",
            "[\"Atziri Flameblast\"] = {",
            "[\"Atziri Flameblast\"] = { additionalStats={base={SelfPhysicalMin=0}},",
        ),
    ] {
        let mut changed = sources().clone();
        let text = changed.get_mut(path).unwrap();
        assert_eq!(text.matches(needle).count(), 1);
        *text = text.replacen(needle, replacement, 1);
        let actual = capture_enemy(&changed).unwrap();
        assert_ne!(
            actual["source_files_sha256"],
            evidence::source_files_digest()
        );
        if path.ends_with("BossSkills.lua") {
            assert!(
                actual["boss_additional_stats"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["stats"].get("SelfPhysicalMin").is_some())
            );
        } else {
            assert!(
                !actual["target_modifiers"].as_array().unwrap().is_empty(),
                "actual original constructor must expose candidate"
            );
        }
        assert!(
            std::panic::catch_unwind(|| evidence::check_semantics(&actual)).is_err(),
            "accepted real changed source {path}"
        );
    }
}
