//! Constructed flat-physical suppliers and their unresolved eligibility domains.
//! This inventory is deliberately nonempty and grants no native coverage.
use super::*;
#[path = "../../../tests/support/owned_self_flat_damage_evidence.rs"]
mod evidence;
const SELF_PACKET: &str = "data/owned/poe2/3887ae68/self-flat-attack-damage/source-evidence.json";

fn fingerprint(value: &Json) -> String {
    hash(&serde_json::to_vec(value).unwrap())
}

fn targets(sources: &BTreeMap<String, String>, value: Value) -> Result<Vec<Json>> {
    let mut census = Inventory::default();
    census.visit(sources, "mapping", value, 0)?;
    Ok(census.physical_flat_records)
}

fn raw_mappings(
    sources: &BTreeMap<String, String>,
    table: &Table,
    owner: &str,
    expected_owner: Option<&Table>,
    scope: &str,
    rows: &mut Vec<Json>,
    keys: &mut BTreeSet<String>,
) -> Result<()> {
    // Raw entries only; resolved consumer lookup below separately honors the
    // authenticated original global stat-map fallback and local overrides.
    for pair in table.clone().pairs::<String, Value>() {
        let (key, value) = pair?;
        if key == "_grantedEffect" {
            // Data.lua951 installs this owner back-reference, not a stat map.
            let Value::Table(owner_table) = value else {
                return Err(error("stat-map owner reference is not a table"));
            };
            if owner_table.raw_get::<String>("id")? != owner
                || expected_owner
                    .is_none_or(|expected| expected.to_pointer() != owner_table.to_pointer())
            {
                return Err(error("stat-map owner reference mismatch"));
            }
            continue;
        }
        let records = targets(sources, value)
            .map_err(|err| error(format!("mapping {owner}/{scope}/{key}: {err}")))?;
        if !records.is_empty() {
            keys.insert(key.clone());
            rows.push(json!({"owner":owner,"scope":scope,"stat":key,"records":records}));
        }
    }
    Ok(())
}

fn parser_controls(sources: &BTreeMap<String, String>) -> Result<Json> {
    let (lua, loaded, ()) =
        crate::unique_requirements_extract::host_with_observer(sources, |_| Ok(()))?;
    assert!(
        loaded
            .lock()
            .unwrap()
            .iter()
            .any(|p| p == "src/Modules/ModParser.lua")
    );
    let library: Table = lua.globals().get("modLib")?;
    let parse: Function = library.get("parseMod")?;
    let mut out = Vec::new();
    for line in evidence::CONTROL_LINES {
        let (records, remaining): (Value, Value) = parse.call(line)?;
        let flat = targets(sources, records.clone())?;
        out.push(json!({"line":line,"records":plain(records,0)?,"remaining":plain(remaining,0)?,"flat_records":flat}));
    }
    Ok(json!(out))
}

fn capture_self(sources: &BTreeMap<String, String>) -> Result<Json> {
    let (lua, result, module_order) = construct_catalog(sources)?;
    let data: Table = result.raw_get("data")?;
    let skills: Table = data.raw_get("skills")?;
    let global: Table = data.raw_get("skillStatMap")?;
    let (mut inventory, minions) = catalog_inventory(&lua, sources, &data, &result)?;
    let catalog = json!({"tables":inventory.seen.len(),"rows":inventory.rows,
        "aliases_sha256":fingerprint(&json!(inventory.aliases)),"callbacks_sha256":fingerprint(&json!(inventory.callbacks)),
        "metatables_sha256":fingerprint(&json!(inventory.metatables))});
    // Keep every traversed root live while the cache allocates objects. The
    // target rows themselves are already Rust JSON, not thousands of Lua refs.
    lua.gc_collect()?;
    let cache: Table = lua
        .load(source(sources, "src/Data/ModCache.lua")?)
        .set_name("@src/Data/ModCache.lua")
        .eval()?;
    inventory.visit(sources, "modCache", Value::Table(cache.clone()), 0)?;
    assert!(inventory.modifier_ancestors.is_empty());
    for row in &mut inventory.physical_flat_records {
        row["inner_filter_shape"] = json!(evidence::qualification(&row["record"]));
    }
    // Canonical rows are unique objects, not every delivery occurrence. Keep
    // relevant alias routes without expanding each shared stat-map subtree.
    let mut alias_groups: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for alias in &inventory.aliases {
        let target = alias["target"].as_str().unwrap();
        let paths: Vec<_> = inventory
            .physical_flat_records
            .iter()
            .filter_map(|r| {
                let path = r["path"].as_str().unwrap();
                (path == target || path.starts_with(&format!("{target}/"))).then(|| path.to_owned())
            })
            .collect();
        if !paths.is_empty() {
            alias_groups
                .entry(target.to_owned())
                .or_insert_with(|| (Vec::new(), paths))
                .0
                .push(alias["path"].as_str().unwrap().to_owned());
        }
    }
    let alias_routes:Vec<_>=alias_groups.into_iter().map(|(target,(aliases,paths))|json!({"target":target,"alias_paths":aliases,"canonical_flat_paths":paths})).collect();
    let mut mappings = Vec::new();
    let mut keys = BTreeSet::new();
    raw_mappings(
        sources,
        &global,
        "<shared>",
        None,
        "skillStatMap",
        &mut mappings,
        &mut keys,
    )?;
    let mut skill_rows = BTreeMap::new();
    for pair in skills.clone().pairs::<String, Table>() {
        let (id, skill) = pair?;
        skill_rows.insert(id, skill);
    }
    for (id, skill) in &skill_rows {
        if let Some(map) = skill.raw_get::<Option<Table>>("statMap")? {
            raw_mappings(
                sources,
                &map,
                id,
                Some(skill),
                "statMap",
                &mut mappings,
                &mut keys,
            )?;
        }
        let sets: Table = skill.raw_get("statSets")?;
        for pair in sets.pairs::<u32, Table>() {
            let (index, set) = pair?;
            if let Some(map) = set.raw_get::<Option<Table>>("statMap")? {
                raw_mappings(
                    sources,
                    &map,
                    id,
                    Some(skill),
                    &format!("statSets/{index}/statMap"),
                    &mut mappings,
                    &mut keys,
                )?;
            }
        }
    }
    mappings.sort_by_key(|row| serde_json::to_string(row).unwrap());
    let all_consumers = stat_consumers(&skills, &keys)?;
    let mut consumers = Vec::new();
    let mut owners = BTreeMap::new();
    for row in all_consumers.as_array().unwrap() {
        let id = row["skill_id"].as_str().unwrap();
        let skill = &skill_rows[id];
        let sets: Table = skill.raw_get("statSets")?;
        let fixed_set = row["scope"]
            .as_str()
            .unwrap()
            .strip_prefix("statSets/")
            .map(|s| s.split('/').next().unwrap().parse::<u32>().unwrap());
        for pair in sets.clone().pairs::<u32, Table>() {
            let (index, set) = pair?;
            if fixed_set.is_some_and(|selected| selected != index) {
                continue;
            }
            let map: Table = set.raw_get("statMap")?;
            let resolved: Value = map.get(row["stat"].as_str().unwrap())?;
            let resolved_targets = targets(sources, resolved)?;
            let mut consumer = row.clone();
            consumer["resolved_stat_set"] = json!(index);
            consumer["resolved_flat_records"] = json!(resolved_targets);
            consumers.push(consumer);
        }
        owners.insert(
            id.to_owned(),
            metadata(
                skill,
                &[
                    "name",
                    "support",
                    "hidden",
                    "skillTypes",
                    "requireSkillTypes",
                    "excludeSkillTypes",
                    "addSkillTypes",
                    "weaponTypes",
                ],
            )?,
        );
    }
    // Direct baseMods and nested records need owner metadata even when they
    // have no stat-input row. Cache records retain their exact cache key path.
    for row in &inventory.physical_flat_records {
        if let Some(id) = row["path"]
            .as_str()
            .unwrap()
            .strip_prefix("skills/")
            .and_then(|s| s.split('/').next())
        {
            let skill = &skill_rows[id];
            owners.insert(
                id.to_owned(),
                metadata(
                    skill,
                    &[
                        "name",
                        "support",
                        "hidden",
                        "skillTypes",
                        "requireSkillTypes",
                        "excludeSkillTypes",
                        "addSkillTypes",
                        "weaponTypes",
                    ],
                )?,
            );
        }
    }
    consumers.sort_by_key(|row| serde_json::to_string(row).unwrap());
    let extra_stats: Vec<_> = inventory
        .all_modifiers
        .iter()
        .filter(|(_, table)| {
            table.raw_get::<String>("name").ok().as_deref() == Some("ExtraSkillStat")
        })
        .map(|(path, table)| {
            Ok(json!({"path":path,"record":plain(Value::Table(table.clone()),0)?}))
        })
        .collect::<Result<_>>()?;
    let files: BTreeMap<_, _> = sources
        .iter()
        .map(|(path, text)| (path, hash(text.as_bytes())))
        .collect();
    let selected = capture(sources)?;
    let actual = json!({"schema_version":1,"kind":"pinned-constructed-self-flat-physical-supplier-census",
        "upstream_revision":crate::source::UPSTREAM_REVISION,"source_manifest_sha256":crate::source::manifest_sha256(),
        "scope":evidence::scope(),"dependencies":evidence::dependencies(),"source_files_sha256":fingerprint(&json!(files)),
        "module_order":module_order,"catalog":catalog,
        "cache_census":{"tables":inventory.seen.len(),"rows":inventory.rows,"modifiers":inventory.modifier_count,
            "aliases_sha256":fingerprint(&json!(inventory.aliases)),"name_counts":inventory.modifier_name_counts.iter().filter(|(n,_)|matches!(n.as_str(),"PhysicalMin"|"PhysicalMax")).collect::<BTreeMap<_,_>>()},
        "flat_records":inventory.physical_flat_records,"relevant_alias_routes":alias_routes,"mappings":mappings,"stat_consumers":consumers,"owners":owners,
        "extra_stats":extra_stats,"extra_stat_factories":coefficient::extra_stat_factories(&lua,sources)?,
        "selected":selected["catalog"]["selected"],"selected_pass_corroboration":selected["selected_pass_corroboration"],
        "parser_controls":parser_controls(sources)?,"observations":evidence::observations(),
        "dynamic_domains":evidence::dynamic_domains()});
    drop((cache, minions));
    Ok(actual)
}

#[test]
#[ignore = "authors nonempty original self-flat supplier/eligibility evidence"]
fn author_self_flat_supplier_census() {
    let actual = capture_self(sources()).unwrap();
    std::fs::write(
        root().join("runs/owned-self-flat-source-candidate.json"),
        serde_json::to_vec_pretty(&actual).unwrap(),
    )
    .unwrap();
    evidence::check(&actual, true);
    let path = root().join(SELF_PACKET);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&actual).unwrap()).unwrap();
}

#[test]
fn original_self_flat_census_and_controls_match_retained_evidence() {
    let actual = capture_self(sources()).unwrap();
    evidence::check(&actual, true);
    let retained: Json =
        serde_json::from_slice(&std::fs::read(root().join(SELF_PACKET)).unwrap()).unwrap();
    assert!(actual == retained, "self-flat supplier census changed");
}

#[test]
fn actual_flat_sources_wrappers_conditions_and_stat_consumers_require_review() {
    for body in [
        "baseMods={mod(\"PhysicalMin\",\"BASE\",0)},",
        "baseMods={mod(\"PhysicalMax\",\"BASE\",7)},",
        "baseMods={mod(\"MinionModifier\",\"LIST\",{mod=mod(\"PhysicalMin\",\"BASE\",0,0,0,{type=\"Condition\",var=\"FullLife\"})})},",
        "baseMods={mod(\"ExtraSkillMod\",\"LIST\",{mod=mod(\"Physical\"..\"Max\",\"BASE\",-2)})},",
        "baseMods=(function() local shared=mod(\"PhysicalMin\",\"BASE\",0); return {shared,mod(\"MinionModifier\",\"LIST\",{mod=shared})} end)(),",
        "qualityStats={{\"attack_minimum_added_physical_damage\",1}},",
        "qualityStats={{\"attack_maximum_added_physical_damage\",0}},",
    ] {
        let mut changed = sources().clone();
        let text = changed.get_mut("src/Data/Skills/minion.lua").unwrap();
        let needle = "skills[\"GasShotSkeletonSniperMinion\"] = {";
        assert_eq!(text.matches(needle).count(), 1);
        let (field, value) = body.trim_end_matches(',').split_once('=').unwrap();
        *text = text.replacen(
            needle,
            &format!("skills[\"MinionMeleeBow\"].{field}={value}\n{needle}"),
            1,
        );
        let actual = capture_self(&changed).unwrap();
        assert_ne!(
            actual["source_files_sha256"],
            evidence::source_files_digest()
        );
        if field == "baseMods" {
            assert!(actual["flat_records"].as_array().unwrap().iter().any(|r| {
                r["path"]
                    .as_str()
                    .unwrap()
                    .starts_with("skills/MinionMeleeBow/baseMods/")
            }));
            if body.contains("local shared=") {
                assert!(
                    actual["relevant_alias_routes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|route| {
                            route["target"] == "skills/MinionMeleeBow/baseMods/[1]"
                                && route["alias_paths"].as_array().unwrap().contains(&json!(
                                    "skills/MinionMeleeBow/baseMods/[2]/value/mod"
                                ))
                        })
                );
            }
        } else {
            assert!(
                actual["stat_consumers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["skill_id"] == BASIC
                        && !r["resolved_flat_records"].as_array().unwrap().is_empty())
            );
        }
        assert!(std::panic::catch_unwind(|| evidence::check_semantics(&actual)).is_err());
    }
}

#[test]
fn stat_map_metadata_requires_the_exact_original_owner_table() {
    let lua = Lua::new();
    let owner = lua.create_table().unwrap();
    owner.raw_set("id", BASIC).unwrap();
    let impostor = lua.create_table().unwrap();
    impostor.raw_set("id", BASIC).unwrap();
    let map = lua.create_table().unwrap();
    map.raw_set("_grantedEffect", impostor).unwrap();
    assert!(
        raw_mappings(
            sources(),
            &map,
            BASIC,
            Some(&owner),
            "statMap",
            &mut Vec::new(),
            &mut BTreeSet::new()
        )
        .is_err()
    );
    map.raw_set("_grantedEffect", owner.clone()).unwrap();
    assert!(
        raw_mappings(
            sources(),
            &map,
            BASIC,
            None,
            "skillStatMap",
            &mut Vec::new(),
            &mut BTreeSet::new()
        )
        .is_err()
    );
    raw_mappings(
        sources(),
        &map,
        BASIC,
        Some(&owner),
        "statMap",
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .unwrap();
}
