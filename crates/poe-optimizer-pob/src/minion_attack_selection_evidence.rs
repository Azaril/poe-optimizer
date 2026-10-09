//! Test-only acquisition of a pinned source-selection proof. No runtime rules.
use super::*;
use serde_json::{Value as Json, json};
use std::{path::PathBuf, sync::OnceLock};

#[path = "../../../tests/support/owned_minion_attack_selection_evidence.rs"]
mod retained;

const SUMMON: &str = "SummonSkeletalSnipersPlayer";
const BASIC: &str = "MinionMeleeBow";
const PROFILE: &str = "RaisedSkeletonSniper";
const PACKET: &str = "data/owned/poe2/3887ae68/minion-attack-selection/source-evidence.json";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sources() -> &'static BTreeMap<String, String> {
    static SOURCES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        let upstream = root().join("vendor/path-of-building-poe2");
        crate::source::verify(&upstream).unwrap();
        let manifest: Json =
            serde_json::from_str(include_str!("../data/pob-source-manifest.json")).unwrap();
        manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|file| {
                let path = file["path"].as_str().unwrap();
                path.ends_with(".lua").then(|| {
                    let text = std::fs::read_to_string(upstream.join(path))
                        .unwrap()
                        .replace("\r\n", "\n");
                    assert_eq!(hash(text.as_bytes()), file["sha256"]);
                    (path.to_owned(), text)
                })
            })
            .collect()
    })
}

fn field(table: &Table, name: &str) -> Result<Json> {
    plain(table.raw_get(name)?, 0)
}

fn plain(value: Value, depth: usize) -> Result<Json> {
    if depth > 32 {
        return Err(error("selection metadata depth bound"));
    }
    Ok(match value {
        Value::Nil => Json::Null,
        Value::Boolean(value) => json!(value),
        Value::Integer(value) => json!(value),
        Value::Number(value) if value.is_finite() => json!(value),
        Value::String(value) => json!(value.to_str()?.as_ref()),
        Value::Table(table) => {
            if table.metatable().is_some() {
                return Err(error(
                    "selection metadata has unreviewed metatable behavior",
                ));
            }
            let mut rows = BTreeMap::new();
            for pair in table.pairs::<Value, Value>() {
                let (key, value) = pair?;
                if rows
                    .insert(key_text(key)?, plain(value, depth + 1)?)
                    .is_some()
                {
                    return Err(error("ambiguous selection metadata key projection"));
                }
            }
            json!(rows)
        }
        _ => return Err(error("selection metadata is not plain finite data")),
    })
}

fn key_text(key: Value) -> Result<String> {
    match key {
        Value::String(value) => Ok(value.to_str()?.to_owned()),
        Value::Integer(value) => Ok(format!("[{value}]")),
        Value::Number(value) if value.is_finite() && value.fract() == 0.0 => {
            Ok(format!("[{value:.0}]"))
        }
        _ => Err(error("unsupported raw catalog key")),
    }
}

fn callback_span(sources: &BTreeMap<String, String>, function: &Function) -> Result<Json> {
    let info = function.info();
    if info.what != "Lua" {
        return Err(error("catalog callback must be original Lua"));
    }
    let path = info
        .source
        .as_deref()
        .and_then(|s| s.strip_prefix('@'))
        .ok_or_else(|| error("callback source path missing"))?;
    let first = info
        .line_defined
        .ok_or_else(|| error("callback first line missing"))?;
    let last = info
        .last_line_defined
        .ok_or_else(|| error("callback last line missing"))?;
    let text = source(sources, path)?;
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    if first == 0 || last < first || last > lines.len() {
        return Err(error("callback span outside authenticated file"));
    }
    Ok(
        json!({"path":path,"line":first,"end_line":last,"sha256":hash(lines[first-1..last].concat().as_bytes())}),
    )
}

#[derive(Default)]
struct Inventory {
    allowed_stat_map_meta: Option<usize>,
    seen: BTreeMap<usize, String>,
    rows: usize,
    modifiers: Vec<Json>,
    callbacks: Vec<Json>,
    aliases: Vec<Json>,
    metatables: Vec<Json>,
    source_metadata: Vec<Json>,
}

impl Inventory {
    fn visit(
        &mut self,
        sources: &BTreeMap<String, String>,
        path: &str,
        value: Value,
        depth: usize,
    ) -> Result<()> {
        self.rows += 1;
        if self.rows > 2_000_000 || depth > 64 {
            return Err(error("catalog inventory resource bound"));
        }
        match value {
            Value::Table(table) => {
                if let Some(meta) = table.metatable()
                    && (Some(meta.to_pointer() as usize) != self.allowed_stat_map_meta
                        || !path.ends_with("/statMap"))
                {
                    return Err(error("unreviewed catalog metatable behavior"));
                }
                let pointer = table.to_pointer() as usize;
                if let Some(previous) = self.seen.get(&pointer) {
                    self.aliases.push(json!({"path":path,"target":previous}));
                    return Ok(());
                }
                self.seen.insert(pointer, path.to_owned());
                if table
                    .raw_get::<Value>("name")?
                    .as_string()
                    .is_some_and(|name| name.as_bytes().as_ref() == b"SkillData")
                {
                    if table.raw_get::<String>("type")? != "LIST" {
                        return Err(error("SkillData modifier is not LIST"));
                    }
                    let value: Table = table.raw_get("value")?;
                    let key = required_text(&value, "key")?;
                    self.modifiers.push(json!({"path":path,"key":key}));
                }
                let mut rows = BTreeMap::new();
                for pair in table.clone().pairs::<Value, Value>() {
                    let (key, value) = pair?;
                    if rows.insert(key_text(key)?, value).is_some() {
                        return Err(error("ambiguous raw catalog key projection"));
                    }
                }
                for (key, value) in rows {
                    if matches!(
                        key.as_str(),
                        "minionUses" | "minionHasItemSet" | "addFlags" | "skillFlag"
                    ) {
                        self.source_metadata.push(
                            json!({"path":format!("{path}/{key}"),"value":plain(value.clone(),0)?}),
                        );
                    }
                    self.visit(sources, &format!("{path}/{key}"), value, depth + 1)?;
                }
                if let Some(meta) = table.metatable() {
                    self.metatables
                        .push(json!({"path":path,"metatable":format!("{path}/<metatable>")}));
                    self.visit(
                        sources,
                        &format!("{path}/<metatable>"),
                        Value::Table(meta),
                        depth + 1,
                    )?;
                }
            }
            Value::Function(function) => self
                .callbacks
                .push(json!({"path":path,"source":callback_span(sources,&function)?})),
            Value::Nil
            | Value::Boolean(_)
            | Value::Integer(_)
            | Value::Number(_)
            | Value::String(_) => {}
            _ => return Err(error("unsupported catalog inventory value")),
        }
        Ok(())
    }
}

fn metadata(table: &Table, fields: &[&str]) -> Result<Json> {
    let fields: BTreeMap<_, _> = fields
        .iter()
        .map(|name| Ok((*name, field(table, name)?)))
        .collect::<Result<_>>()?;
    Ok(json!(fields))
}

fn direct_callbacks(sources: &BTreeMap<String, String>, table: &Table) -> Result<Json> {
    let mut callbacks = BTreeMap::new();
    for pair in table.clone().pairs::<Value, Value>() {
        let (key, value) = pair?;
        if let Value::Function(function) = value {
            callbacks.insert(key_text(key)?, callback_span(sources, &function)?);
        }
    }
    for name in [
        "initialFunc",
        "preSkillTypeFunc",
        "preDamageFunc",
        "preDotFunc",
    ] {
        match table.raw_get::<Value>(name)? {
            Value::Nil | Value::Function(_) => {}
            _ => {
                return Err(error(
                    "selected execution callback is not a function or absent",
                ));
            }
        }
    }
    Ok(json!(callbacks))
}

// This is an acquisition ledger, not a Lua interpreter. The original pinned
// producers/transport and their complete source domains are reviewed separately.
// A new dynamic construction spelling is deliberately not inferred as harmless.
fn static_inventory(lua: &Lua, sources: &BTreeMap<String, String>) -> Result<Json> {
    let mut references = Vec::new();
    let mut factory_keys = Vec::new();
    let mut dynamic_writes = Vec::new();
    for (path, text) in sources {
        if !(path.starts_with("src/Modules/")
            || path.starts_with("src/Classes/")
            || path.starts_with("src/Data/"))
        {
            continue;
        }
        let code = tokens(text)?;
        for (i, token) in code.iter().enumerate() {
            let decoded = if token.quoted {
                // Only candidate factory/reference literals require evaluation;
                // all other strings remain inert, including generated prose.
                if token.text.contains("SkillData")
                    || token.text.contains("WeaponData")
                    || token.text.contains("minionUseBowAndQuiver")
                {
                    Some(
                        lua.load(format!("return {}", token.text))
                            .eval::<String>()?,
                    )
                } else {
                    None
                }
            } else {
                Some(token.text.to_owned())
            };
            if decoded.as_deref().is_some_and(|name| {
                matches!(name, "SkillData" | "WeaponData" | "minionUseBowAndQuiver")
            }) {
                references.push(
                    json!({"path":path,"line":token.line,"name":decoded,"quoted":token.quoted}),
                );
            }
            if !token.quoted
                && token.text == "skill"
                && code.get(i + 1).is_some_and(|t| t.text == "(")
                && (path.starts_with("src/Data/Skills/") || path == "src/Data/SkillStatMap.lua")
            {
                let key = code
                    .get(i + 2)
                    .filter(|t| t.quoted)
                    .ok_or_else(|| error("dynamic SkillData helper key requires source review"))?;
                if !code.get(i + 3).is_some_and(|t| t.text == ",") {
                    return Err(error(
                        "computed SkillData helper key requires source review",
                    ));
                }
                let key: String = lua.load(format!("return {}", key.text)).eval()?;
                factory_keys
                    .push(json!({"path":path,"line":token.line,"key":key,"kind":"skill-helper"}));
            }
            if decoded.as_deref() == Some("SkillData")
                && token.quoted
                && code
                    .get(i + 2)
                    .is_some_and(|t| t.quoted && (t.text == "\"LIST\"" || t.text == "'LIST'"))
                && code.get(i + 4).is_some_and(|t| t.text == "{")
            {
                let mut depth = 1;
                let mut key = None;
                for j in i + 5..code.len() {
                    if !code[j].quoted {
                        match code[j].text {
                            "{" => depth += 1,
                            "}" => depth -= 1,
                            _ => {}
                        }
                    }
                    if depth == 0 {
                        break;
                    }
                    if depth == 1
                        && code[j].text == "key"
                        && code.get(j + 1).is_some_and(|t| t.text == "=")
                    {
                        let argument = code
                            .get(j + 2)
                            .ok_or_else(|| error("missing SkillData key"))?;
                        if argument.quoted {
                            if !code
                                .get(j + 3)
                                .is_some_and(|t| t.text == "," || t.text == "}")
                            {
                                return Err(error(
                                    "computed SkillData record key requires source review",
                                ));
                            }
                            key = Some(
                                lua.load(format!("return {}", argument.text))
                                    .eval::<String>()?,
                            );
                        } else if path == DATA && token.line == 70 && argument.text == "dataKey" {
                            key = Some("<original makeSkillDataMod parameter>".into());
                        } else {
                            return Err(error(
                                "dynamic SkillData record key requires source review",
                            ));
                        }
                    }
                }
                // The one generic constructor receives only literal helper keys;
                // its original body is authenticated, not rewritten here.
                factory_keys.push(json!({"path":path,"line":token.line,"key":key.ok_or_else(||error("SkillData LIST record has no literal key"))?,"kind":"list-record"}));
            }
            if !token.quoted
                && token.text == "skillData"
                && code.get(i + 1).is_some_and(|t| t.text == "[")
            {
                let Some(end) = (i + 2..code.len()).find(|j| code[*j].text == "]") else {
                    return Err(error("unterminated skillData index"));
                };
                if code.get(end + 1).is_some_and(|t| t.text == "=")
                    && !code.get(end + 2).is_some_and(|t| t.text == "=")
                {
                    let allowed = match path.as_str() {
                        "src/Modules/CalcActiveSkill.lua" => [897, 900].contains(&token.line),
                        "src/Modules/CalcSetup.lua" => [2221, 2224].contains(&token.line),
                        "src/Modules/CalcOffence.lua" => {
                            [740, 742, 2413, 2414, 2420, 2421].contains(&token.line)
                        }
                        "src/Modules/CalcDefence.lua" => {
                            [242, 243, 246, 247, 297, 314, 315].contains(&token.line)
                        }
                        "src/Data/Skills/other.lua" => (21156..=21182).contains(&token.line),
                        _ => false,
                    };
                    if !allowed {
                        return Err(error(format!(
                            "unreviewed dynamic SkillData writer {path}:{}",
                            token.line
                        )));
                    }
                    dynamic_writes.push(json!({"path":path,"line":token.line,"key_tokens":code[i+2..end].iter().map(|t|t.text).collect::<Vec<_>>()}));
                }
            }
        }
    }
    Ok(json!({"references":references,"factory_keys":factory_keys,"dynamic_writes":dynamic_writes}))
}

fn capture(sources: &BTreeMap<String, String>) -> Result<Json> {
    let (lua, result, module_order) = construct_catalog(sources)?;
    let data: Table = result.get("data")?;
    let skills: Table = data.get("skills")?;
    let global_map: Table = data.get("skillStatMap")?;
    let minions: Function = lua
        .load(source(sources, "src/Data/Minions.lua")?)
        .set_name("@src/Data/Minions.lua")
        .eval()?;
    let minions: Table = minions.call((
        result.get::<Function>("mod")?,
        result.get::<Function>("flag")?,
    ))?;
    let summon: Table = skills.raw_get(SUMMON)?;
    let basic: Table = skills.raw_get(BASIC)?;
    let profile: Table = minions.raw_get(PROFILE)?;
    for table in [&summon, &basic, &profile] {
        if table.metatable().is_some() {
            return Err(error(
                "selected source root has unreviewed metatable behavior",
            ));
        }
    }
    let stat_map_meta: Table = data.raw_get("skillStatMapMeta")?;
    let mut inventory = Inventory {
        allowed_stat_map_meta: Some(stat_map_meta.to_pointer() as usize),
        ..Inventory::default()
    };
    inventory.visit(sources, "skills", Value::Table(skills.clone()), 0)?;
    inventory.visit(sources, "skillStatMap", Value::Table(global_map), 0)?;
    inventory.visit(sources, "minions", Value::Table(minions.clone()), 0)?;
    let stat_sets: Table = basic.raw_get("statSets")?;
    let mut flags = BTreeMap::new();
    for pair in stat_sets.pairs::<u32, Table>() {
        let (index, stat_set) = pair?;
        if stat_set.metatable().is_some() {
            return Err(error("selected stat set has unreviewed metatable behavior"));
        }
        let mut row = metadata(&stat_set, &["baseFlags", "parts", "skillFlags"])?;
        row["direct_callbacks"] = direct_callbacks(sources, &stat_set)?;
        flags.insert(index, row);
    }
    let files: BTreeMap<_, _> = sources
        .iter()
        .map(|(p, t)| (p, hash(t.as_bytes())))
        .collect();
    let selected = json!({
        "summon_id":SUMMON,"basic_id":BASIC,"profile_id":PROFILE,
        "selected_roots_have_no_metatable":true,
        "summon_callbacks":direct_callbacks(sources,&summon)?,
        "basic_callbacks":direct_callbacks(sources,&basic)?,
        "profile_callbacks":direct_callbacks(sources,&profile)?,
        "summon":metadata(&summon,&["minionList","minionUses","minionHasItemSet","parts","addFlags"] )?,
        "basic":metadata(&basic,&["parts","baseFlags","addFlags","weaponTypes"] )?,
        "basic_stat_sets":flags,
        "profile":metadata(&profile,&["skillList","weaponType1","weaponType2","hostile","modList","damage","damageSpread","attackTime","critChance","baseDamageIgnoresAttackSpeed"] )?
    });
    let static_inventory = static_inventory(&lua, sources)?;
    let evidence = json!({
        "schema_version":1,"kind":"pinned-constructed-minion-intrinsic-source-selection",
        "upstream_revision":crate::source::UPSTREAM_REVISION,"source_manifest_sha256":crate::source::manifest_sha256(),
        "scope":{"source_selection_only":true,"native_numerical_coverage":false,"all_actions":false,"normal_import_only":true},
        "files":files,"module_order":module_order,
        "catalog":{"skill_count":skills.clone().pairs::<Value,Value>().count(),"raw_tables":inventory.seen.len(),"raw_rows":inventory.rows,
            "skill_data_modifiers":inventory.modifiers,"callbacks":inventory.callbacks,"aliases":inventory.aliases,
            "metatables":inventory.metatables,"metatable_policy":"only-original-skillStatMapMeta-on-statMap","source_metadata":inventory.source_metadata,"selected":selected},
        "static_inventory":static_inventory,
        "selected_pass_corroboration":{"path":"data/owned/poe2/3887ae68/minion-preconversion-source/report.json","sha256":"7a723a4eece7b297737006b08d1dc61ec0047d13cfd2b9fbfeaee94eb52397cb","bytes":4041068,"role":"corroboration-not-supplier-completeness"}
    });
    Ok(evidence)
}

#[test]
#[ignore = "authors the reviewed offline source-selection evidence"]
fn author_intrinsic_selection_evidence() {
    let evidence = capture(sources()).unwrap();
    retained::check(&evidence, true);
    let path = root().join(PACKET);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}

#[test]
fn constructed_intrinsic_selection_matches_retained_evidence() {
    let evidence = capture(sources()).unwrap();
    retained::check(&evidence, true);
    let expected: Json =
        serde_json::from_slice(&std::fs::read(root().join(PACKET)).unwrap()).unwrap();
    assert!(
        evidence == expected,
        "constructed selection proof differs from retained evidence"
    );
}

#[test]
fn source_selection_semantics_refuse_new_suppliers_metadata_and_callbacks() {
    let original = capture(sources()).unwrap();
    retained::check(&original, false);
    for (label, mutated) in retained::invalid_controls(&original) {
        assert!(
            std::panic::catch_unwind(|| retained::check(&mutated, false)).is_err(),
            "accepted {label}"
        );
    }
}

#[test]
fn real_constructor_refuses_source_affecting_mutations_with_updated_file_hashes() {
    for (needle, replacement) in [
        (
            "skills[\"SummonSkeletalSnipersPlayer\"] = {",
            "skills[\"SummonSkeletalSnipersPlayer\"] = { minionUses = {[\"Weapon 1\"]=true},",
        ),
        (
            "skills[\"SummonSkeletalSnipersPlayer\"] = {",
            "skills[\"SummonSkeletalSnipersPlayer\"] = { minionHasItemSet = true,",
        ),
        (
            "skills[\"SummonSkeletalSnipersPlayer\"] = {",
            "skills[\"SummonSkeletalSnipersPlayer\"] = { baseMods={skill(\"minionUseBowAndQuiver\",true)},",
        ),
    ] {
        let mut changed = sources().clone();
        let text = changed.get_mut("src/Data/Skills/act_int.lua").unwrap();
        assert!(text.contains(needle));
        *text = text.replacen(needle, replacement, 1);
        let evidence = capture(&changed).unwrap();
        assert!(std::panic::catch_unwind(|| retained::check_semantics(&evidence)).is_err());
    }
    let mut changed = sources().clone();
    let text = changed.get_mut("src/Data/Skills/minion.lua").unwrap();
    *text=text.replacen("skills[\"MinionMeleeBow\"] = {","skills[\"MinionMeleeBow\"] = { preDamageFunc=function(activeSkill) activeSkill.actor.weaponData1 = {} end,",1);
    let evidence = capture(&changed).unwrap();
    assert!(std::panic::catch_unwind(|| retained::check_semantics(&evidence)).is_err());
}

#[test]
fn dynamic_factory_key_requires_new_source_review() {
    let mut changed = sources().clone();
    let text = changed.get_mut("src/Data/Skills/act_int.lua").unwrap();
    *text=text.replacen("skills[\"SummonSkeletalSnipersPlayer\"] = {","skills[\"SummonSkeletalSnipersPlayer\"] = { baseMods={skill(\"minion\" .. \"UseBowAndQuiver\",true)},",1);
    let result = capture(&changed);
    // A nonliteral expression must not pass simply because its first token is a string.
    assert!(
        result.is_err()
            || std::panic::catch_unwind(|| retained::check_semantics(&result.unwrap())).is_err()
    );
}

#[test]
fn mixed_key_spellings_cannot_hide_catalog_rows() {
    let lua = Lua::new();
    let table:Table=lua.load("return {[1]='ordinary', ['[1]']={name='SkillData',type='LIST',value={key='minionUseBowAndQuiver',value=true}}}").eval().unwrap();
    assert!(plain(Value::Table(table.clone()), 0).is_err());
    assert!(
        Inventory::default()
            .visit(&BTreeMap::new(), "collision", Value::Table(table), 0)
            .is_err()
    );
}

#[test]
fn selected_callback_alias_cannot_hide_execution_behavior() {
    let mut changed = sources().clone();
    let text = changed.get_mut("src/Data/Skills/minion.lua").unwrap();
    *text = text.replacen("skills[\"GasShotSkeletonSniperMinion\"] = {",
        "skills[\"AAASourceSelectionAlias\"] = skills[\"MinionMeleeBow\"]\nskills[\"MinionMeleeBow\"].preDamageFunc=function(activeSkill) activeSkill.actor.weaponData1 = {} end\nskills[\"GasShotSkeletonSniperMinion\"] = {",1);
    let evidence = capture(&changed).unwrap();
    assert!(
        evidence["catalog"]["callbacks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["path"]
                .as_str()
                .unwrap()
                .starts_with("skills/AAASourceSelectionAlias/"))
    );
    assert!(evidence["catalog"]["selected"]["basic_callbacks"]["preDamageFunc"].is_object());
    assert!(std::panic::catch_unwind(|| retained::check_semantics(&evidence)).is_err());
}

#[test]
fn metatables_cannot_supply_hidden_selection_fields_or_modifiers() {
    let mut changed = sources().clone();
    let text = changed.get_mut("src/Data/Skills/minion.lua").unwrap();
    *text=text.replacen("skills[\"GasShotSkeletonSniperMinion\"] = {",
        "setmetatable(skills[\"MinionMeleeBow\"], {__index={preDamageFunc=function(activeSkill) activeSkill.actor.weaponData1={} end}})\nskills[\"GasShotSkeletonSniperMinion\"] = {",1);
    let failure = capture(&changed).unwrap_err().to_string();
    assert!(
        failure.contains("selected source root has unreviewed metatable behavior"),
        "{failure}"
    );
    let lua = Lua::new();
    let table: Table = lua
        .load("return setmetatable({}, {__index={unarmed=true}})")
        .eval()
        .unwrap();
    assert!(plain(Value::Table(table), 0).is_err());
    let modifier:Table=lua.load("return {value={key='minionUseBowAndQuiver',value=true}, child=setmetatable({}, {__index={name='SkillData'}})}").eval().unwrap();
    let failure = Inventory::default()
        .visit(&BTreeMap::new(), "nested", Value::Table(modifier), 0)
        .unwrap_err()
        .to_string();
    assert!(
        failure.contains("unreviewed catalog metatable behavior"),
        "{failure}"
    );
}
