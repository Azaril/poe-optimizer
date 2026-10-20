//! Finite review of authenticated offline sources; this is not a Lua interpreter.
use super::{GEMS, bounded_read, data, hash, read, root};
use flate2::read::GzDecoder;
use poe_optimizer_data::skill_identities::{SkillIdentityCatalog, SkillIdentityData};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    sync::OnceLock,
};

const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const DESCRIPTION_PATH: &str = "src/Data/StatDescriptions/gem_stat_descriptions.lua";
const RECIPE_HASH: &str = "ebe5ed14555ef4200f843dbf2972e3d3bd5fe82b4d3c4b558e07f187d0c1681f";
const CATALOG_HASH: &str = "b9064ec447ccf3c3fc88244655a36bbb4453e50e16ffa64eda9880f756aed10e";
const MANIFEST_HASH: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";

struct Facts {
    proof: Value,
    catalog: Value,
    recipes: Value,
    sources: BTreeMap<String, String>,
}
fn string(value: &Value) -> &str {
    value.as_str().unwrap()
}
fn exact_keys(value: &Value, expected: &[&str]) {
    let actual: BTreeSet<_> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(actual, expected.iter().copied().collect());
}
fn pinned_json(pin: &Value, path: &str, expected: &str) -> Value {
    assert_eq!(pin["path"], path);
    assert_eq!(pin["sha256"], expected);
    let bytes = bounded_read(root().join(path), 4 * 1024 * 1024);
    assert_eq!(pin["bytes"], bytes.len());
    assert_eq!(hash(&bytes), expected);
    serde_json::from_slice(&bytes).unwrap()
}
fn facts() -> &'static Facts {
    static FACTS: OnceLock<Facts> = OnceLock::new();
    FACTS.get_or_init(|| {
        let proof = read("evidence.json");
        assert_eq!(proof["source_revision"], REVISION);
        let manifest = pinned_json(
            &proof["source_manifest"],
            "crates/poe-optimizer-pob/data/pob-source-manifest.json",
            MANIFEST_HASH,
        );
        assert_eq!(manifest["upstream_revision"], REVISION);
        let catalog = pinned_json(
            &proof["catalog"],
            "data/owned/poe2/3887ae68/import/skill-identities.json",
            CATALOG_HASH,
        );
        let typed: SkillIdentityData = serde_json::from_value(catalog.clone()).unwrap();
        SkillIdentityCatalog::new(typed).unwrap();
        let recipes = pinned_json(
            &proof["recipes"],
            "data/owned/poe2/3887ae68/self-flat-attack-damage/selected-domain.json",
            RECIPE_HASH,
        );
        assert_eq!(recipes["source_revision"], REVISION);
        let mut sources = BTreeMap::new();
        let mut total = 0;
        for pin in proof["sources"].as_array().unwrap() {
            let path = string(&pin["path"]);
            let archive = format!("sources/{}.gz", path.replace('/', "_"));
            assert_eq!(pin["archive"], archive);
            let bytes = bounded_read(data().join(archive), 2 * 1024 * 1024);
            assert_eq!(pin["archive_bytes"], bytes.len());
            assert_eq!(pin["archive_sha256"], hash(&bytes));
            let mut source = Vec::new();
            GzDecoder::new(bytes.as_slice())
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut source)
                .unwrap();
            assert!(source.len() <= 8 * 1024 * 1024);
            total += source.len();
            assert!(total <= 8 * 1024 * 1024);
            assert_eq!(pin["bytes"], source.len());
            assert_eq!(pin["sha256"], hash(&source));
            assert!(!source.contains(&b'\r'));
            let matches: Vec<_> = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == path)
                .collect();
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0]["bytes"], pin["bytes"]);
            assert_eq!(matches[0]["sha256"], pin["sha256"]);
            assert!(
                sources
                    .insert(path.to_owned(), String::from_utf8(source).unwrap())
                    .is_none()
            );
        }
        assert_eq!(sources.len(), 15);
        for boundary in proof["boundaries"].as_array().unwrap() {
            let body = span(&sources, boundary);
            assert!(!body.is_empty());
        }
        let minion_boundary: Vec<_> = proof["boundaries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x["path"] == "src/Modules/CalcActiveSkill.lua" && x["first_line"] == 895)
            .collect();
        assert_eq!(minion_boundary.len(), 1);
        let body = span(&sources, minion_boundary[0]);
        assert!(body.contains("activeGrantedEffect.minionList"));
        assert!(body.contains(
            "skillEffect.grantedEffect.support and skillEffect.grantedEffect.addMinionList"
        ));
        assert!(body.contains("t_insert(minionList, minionType)"));
        let setup_gem: Vec<_> = proof["boundaries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x["path"] == "src/Modules/Data.lua" && x["first_line"] == 979)
            .collect();
        assert_eq!(setup_gem.len(), 1);
        assert_eq!(setup_gem[0]["last_line"], 1068);
        let constructor = span(&sources, setup_gem[0]);
        assert!(constructor.contains("while gem[\"additionalGrantedEffectId\"..i] do"));
        assert!(constructor.contains("if gem.name:sub(1, 7) == \"Barbs I\" then"));
        assert!(constructor.contains("if gem.grantedEffectDisplayOrder then"));
        let setup = &sources["src/Modules/CalcSetup.lua"];
        assert!(setup.contains("if not grantedEffect or not grantedEffect.support then"));
        assert!(
            setup.contains("if not grantedEffect.support and not grantedEffect.hideFromSideBar")
        );
        assert!(sources[DESCRIPTION_PATH].contains("parent=\"stat_descriptions\""));
        Facts {
            proof,
            catalog,
            recipes,
            sources,
        }
    })
}

fn span(sources: &BTreeMap<String, String>, value: &Value) -> String {
    let source = &sources[string(&value["path"])];
    let first = value
        .get("first_line")
        .unwrap_or(&value["line"])
        .as_u64()
        .unwrap() as usize;
    let last = value
        .get("last_line")
        .unwrap_or(&value["end_line"])
        .as_u64()
        .unwrap() as usize;
    assert!(first > 0 && first <= last && last - first < 1000);
    let lines: Vec<_> = source.split_inclusive('\n').collect();
    assert!(last <= lines.len());
    let body = lines[first - 1..last].concat();
    assert_eq!(hash(body.as_bytes()), string(&value["sha256"]));
    body
}
fn only<'a>(rows: &'a Value, field: &str, expected: &Value) -> &'a Value {
    let found: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| &row[field] == expected)
        .collect();
    assert_eq!(found.len(), 1, "exact identity/reference is required");
    found[0]
}
fn check_identity(family: &Value, catalog: &Value) {
    let gem = only(&catalog["gems"], "key", &family["gem_identity"]["key"]);
    assert_eq!(gem, &family["gem_identity"]);
    let skill = only(&catalog["skills"], "id", &gem["primary_effect_id"]);
    assert_eq!(skill, &family["skill_identity"]);
    assert_eq!(skill["support"], true);
    assert!(skill["from_tree"].is_null());
    for name in [
        "declared_additional_effects",
        "declared_additional_stat_sets",
        "constructed_additional_effects",
        "additional_effects",
    ] {
        assert_eq!(gem[name], json!([]));
    }
    assert_eq!(gem["effect_list"], json!([skill["id"]]));
    assert!(gem["display_order"].is_null());
    assert!(
        !catalog["missing_references"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["gem_key"] == gem["key"])
    );
    let gem_declaration = only(
        &catalog["gem_declarations"],
        "index",
        &gem["winning_declaration"],
    );
    let skill_declaration = only(
        &catalog["skill_declarations"],
        "index",
        &skill["winning_declaration"],
    );
    assert_eq!(gem_declaration, &family["gem_declaration"]);
    assert_eq!(skill_declaration, &family["skill_declaration"]);
    assert_eq!(gem_declaration["key"], gem["key"]);
    assert_eq!(
        gem_declaration["identity"]["primary_effect_id"],
        skill["id"]
    );
    assert_eq!(skill_declaration["id"], skill["id"]);
    check_support_role(gem, skill);
    let mapping = &family["mapping"];
    assert_eq!(mapping["source"]["kind"], "definition");
    assert_eq!(mapping["source"]["value"]["kind"], "gem");
    let external = &mapping["source"]["value"]["value"];
    assert_eq!(
        external["game_id"],
        json!({"kind":"text","value":gem["game_id"]})
    );
    assert_eq!(
        external["variant_id"],
        json!({"kind":"text","value":gem["variant_id"]})
    );
    assert_eq!(mapping["outcome"]["kind"], "mapped");
    assert_eq!(
        mapping["outcome"]["value"]["basis"],
        json!({"kind":"exact"})
    );
    let target = &mapping["outcome"]["value"]["target"];
    assert_eq!(target["kind"], "definition");
    assert_eq!(target["value"]["kind"], "gem");
    assert_eq!(target["value"]["value"]["key"], family["gem"]);
}

fn check_support_role(gem: &Value, skill: &Value) {
    assert_eq!(skill["support"], true);
    assert_eq!(gem["effect_list"], json!([skill["id"]]));
    assert!(!string(&gem["name"]).starts_with("Barbs I"));
    assert!(!string(&gem["key"]).contains("Vaal"));
    for field in [
        "declared_additional_effects",
        "declared_additional_stat_sets",
        "constructed_additional_effects",
        "additional_effects",
    ] {
        assert_eq!(gem[field], json!([]));
    }
}

// These line conventions belong to the exact pinned generated source bodies.
// Rejecting other syntax is deliberate: this checker does not interpret Lua.
fn field_names(body: &str, indent: usize) -> Vec<String> {
    let prefix = "\t".repeat(indent);
    body.lines()
        .filter_map(|line| {
            let line = line.strip_prefix(&prefix)?;
            let (key, _) = line.split_once(" =")?;
            (!key.is_empty() && key.bytes().all(|x| x.is_ascii_alphabetic() || x == b'_'))
                .then(|| key.to_owned())
        })
        .collect()
}
fn quoted(body: &str) -> impl Iterator<Item = &str> {
    body.split('"').skip(1).step_by(2)
}
fn check_primary_body(family: &Value, body: &str) {
    let id = string(&family["skill_identity"]["id"]);
    assert!(body.starts_with(&format!("skills[\"{id}\"] = {{\n")));
    assert!(body.ends_with("\n}\n"));
    assert!(body.contains("\tsupport = true,\n"));
    for forbidden in [
        "minionList",
        "addMinionList",
        "grantsThornsDamage",
        "qualityMods",
        "levelMods",
        "function",
        "ExtraSkill",
        "ExtraSupport",
        "ExtraMinionSkill",
    ] {
        assert!(
            !body.contains(forbidden),
            "unreviewed source factory/callback: {forbidden}"
        );
    }
    let root_fields = field_names(body, 1);
    assert_eq!(json!(root_fields), family["root_fields"]);
    let permitted = [
        "name",
        "description",
        "color",
        "support",
        "requireSkillTypes",
        "addSkillTypes",
        "excludeSkillTypes",
        "gemFamily",
        "ignoreMinionTypes",
        "levels",
        "statSets",
    ];
    assert!(
        root_fields
            .iter()
            .all(|field| permitted.contains(&field.as_str()))
    );
    assert_eq!(
        root_fields.iter().collect::<BTreeSet<_>>().len(),
        root_fields.len()
    );
    let mut fields = field_names(body, 3);
    assert_eq!(fields.iter().collect::<BTreeSet<_>>().len(), fields.len());
    fields.sort();
    assert_eq!(json!(fields), family["stat_set_fields"]);
    assert!(fields.iter().all(|field| {
        [
            "baseEffectiveness",
            "baseFlags",
            "constantStats",
            "incrementalEffectiveness",
            "label",
            "levels",
            "statDescriptionScope",
            "statMap",
            "stats",
        ]
        .contains(&field.as_str())
    }));
    assert_eq!(
        body.matches("statDescriptionScope = \"gem_stat_descriptions\"")
            .count(),
        1
    );
    assert_eq!(body.matches("\tstatSets = {\n\t\t[1] = {").count(), 1);
    let source_stats: BTreeSet<_> = quoted(body)
        .filter(|text| {
            text.contains('_')
                && *text != "gem_stat_descriptions"
                && text
                    .bytes()
                    .all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || b"_+%".contains(&x))
        })
        .collect();
    let rows = family["recipe"]["resolved"].as_array().unwrap();
    let retained: BTreeSet<_> = rows
        .iter()
        .map(|row| {
            assert_eq!(row["set"], 1);
            string(&row["stat"])
        })
        .collect();
    assert_eq!(rows.len(), retained.len());
    assert_eq!(
        source_stats, retained,
        "all source stat inputs must be accounted"
    );
}

fn check_modifier(value: &Value, skill: &str) {
    let fields = value.as_object().unwrap();
    assert!(
        fields.keys().all(|x| [
            "flags",
            "keywordFlags",
            "name",
            "source",
            "type",
            "value",
            "[1]",
        ]
        .contains(&x.as_str())),
        "unreviewed modifier fields"
    );
    assert_eq!(value["source"], format!("Skill:{skill}"));
    assert!(value["flags"].as_u64().is_some() && value["keywordFlags"].as_u64().is_some());
    let pair = (string(&value["name"]), string(&value["type"]));
    if pair == ("MinionModifier", "LIST") {
        exact_keys(&value["value"], &["mod"]);
        check_modifier(&value["value"]["mod"], skill);
    } else {
        assert!(
            [
                ("CooldownRecovery", "INC"),
                ("Damage", "MORE"),
                ("AdditionalCooldownUses", "BASE"),
                ("Duration", "MORE"),
                ("AreaOfEffect", "INC"),
                ("Speed", "INC"),
                ("Speed", "MORE"),
                ("DetonationTime", "MORE"),
                ("RepeatCount", "BASE"),
                ("RepeatAreaOfEffect", "INC"),
            ]
            .contains(&pair),
            "unreviewed modifier is not evidence of structural absence"
        );
        assert!(value.get("value").is_none_or(Value::is_number));
    }
    if let Some(tag) = value.get("[1]") {
        match string(&tag["type"]) {
            "Condition" => {
                exact_keys(tag, &["type", "var"]);
                assert_eq!(tag["var"], "CommandableSkill");
            }
            "Multiplier" => {
                exact_keys(tag, &["type", "var", "actor"]);
                assert_eq!(tag["var"], "PersistentMinionTypes");
                assert_eq!(tag["actor"], "parent");
            }
            "SkillType" => {
                exact_keys(tag, &["type", "skillType"]);
                assert!(tag["skillType"].as_u64().is_some());
            }
            _ => panic!("unreviewed modifier tag"),
        }
    }
}
fn check_records(records: &Value, skill: &str) {
    exact_keys(records, &["aliases", "callbacks", "value"]);
    assert_eq!(records["aliases"], json!([]));
    assert_eq!(records["callbacks"], json!([]));
    if !records["value"].is_null() {
        exact_keys(&records["value"], &["[1]"]);
        check_modifier(&records["value"]["[1]"], skill);
    }
}
fn check_recipe(family: &Value) {
    let recipe = &family["recipe"];
    exact_keys(
        recipe,
        &[
            "base_mods",
            "callbacks",
            "id",
            "metadata",
            "resolved",
            "stat_set_base_mods",
        ],
    );
    let skill = string(&family["skill_identity"]["id"]);
    assert_eq!(recipe["id"], skill);
    assert_eq!(recipe["callbacks"], json!({}));
    assert_eq!(recipe["metadata"]["support"], true);
    assert!(recipe["metadata"]["minionList"].is_null());
    check_records(&recipe["base_mods"], skill);
    assert!(recipe["base_mods"]["value"].is_null());
    let sets = recipe["stat_set_base_mods"].as_array().unwrap();
    assert_eq!(sets.len(), 1);
    exact_keys(&sets[0], &["set", "records"]);
    assert_eq!(sets[0]["set"], 1);
    check_records(&sets[0]["records"], skill);
    assert!(sets[0]["records"]["value"].is_null());
    for row in recipe["resolved"].as_array().unwrap() {
        exact_keys(row, &["set", "stat", "mapping"]);
        check_records(&row["mapping"], skill);
    }
}

fn check_descriptions(family: &Value, sources: &BTreeMap<String, String>) {
    let missing: BTreeSet<_> = family["recipe"]["resolved"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["mapping"]["value"].is_null())
        .map(|row| string(&row["stat"]))
        .collect();
    let rows = family["null_semantics"].as_array().unwrap();
    assert_eq!(
        rows.len(),
        missing.len(),
        "null mapping absence is not semantic evidence"
    );
    let mut reviewed = BTreeSet::new();
    for row in rows {
        let stat = string(&row["stat"]);
        assert!(missing.contains(stat) && reviewed.insert(stat));
        assert_eq!(row["set"], 1);
        assert_eq!(row["scope"], "gem_stat_descriptions");
        assert_eq!(row["classification"], "existing_recipient_numeric_or_usage");
        let description = &row["description"];
        assert_eq!(description["applicable_scope"], true);
        assert_eq!(description["source"]["path"], DESCRIPTION_PATH);
        let body = span(sources, &description["source"]);
        assert_eq!(description["source"]["body"], body);
        let texts: Vec<_> = body
            .lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("text=\"")
                    .and_then(|line| line.strip_suffix('"'))
            })
            .collect();
        assert_eq!(json!(texts), description["text"]);
        assert_eq!(texts.len(), 2);
        let expected_prefix = match stat {
            "support_ground_effect_area_of_effect_+%_final_per_second"
            | "support_ground_effect_area_of_effect_+%_final_per_second_max" => {
                "Ground Surfaces created by Supported Skills "
            }
            "support_last_gasp_duration_ms" => "Minions from Supported Skills die ",
            "support_increased_area_damage_+%_final" => "Supported Skills deal ",
            _ => panic!("unreviewed null mapping"),
        };
        assert!(texts.iter().all(|text| text.starts_with(expected_prefix)));
        let stats_body = body.split_once("\n\t\tstats={\n").unwrap().1;
        assert!(quoted(stats_body).any(|source_stat| source_stat == stat));
        // This proves recipient ownership only, not a formula or native rule.
        assert!(!string(&row["interpretation"]).is_empty());
    }
    assert_eq!(reviewed, missing);
}

fn check_family(family: &Value, facts: &Facts) {
    check_identity(family, &facts.catalog);
    let gem_body = span(&facts.sources, &family["gem_declaration"]["source"]);
    assert!(gem_body.starts_with(&format!(
        "\t[\"{}\"] = {{\n",
        string(&family["gem_identity"]["key"])
    )));
    assert!(gem_body.contains(&format!(
        "grantedEffectId = \"{}\"",
        string(&family["skill_identity"]["id"])
    )));
    assert!(gem_body.contains(&format!(
        "gameId = \"{}\"",
        string(&family["gem_identity"]["game_id"])
    )));
    assert!(gem_body.contains(&format!(
        "variantId = \"{}\"",
        string(&family["gem_identity"]["variant_id"])
    )));
    assert!(gem_body.contains(&format!(
        "name = \"{}\"",
        string(&family["gem_identity"]["name"])
    )));
    for extra in [
        "additionalGrantedEffectId",
        "additionalStatSet",
        "secondaryGrantedEffectId",
        "grantedEffectDisplayOrder",
    ] {
        assert!(!gem_body.contains(extra));
    }
    let body = span(&facts.sources, &family["skill_declaration"]["source"]);
    check_primary_body(family, &body);
    check_recipe(family);
    assert_eq!(
        &family["recipe"],
        only(
            &facts.recipes["skills"],
            "id",
            &family["skill_identity"]["id"]
        ),
        "the complete retained recipe is preserved"
    );
    let mapped: BTreeSet<_> = family["recipe"]["resolved"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| !row["mapping"]["value"].is_null())
        .map(|row| string(&row["stat"]))
        .collect();
    let mut sources_seen = BTreeSet::new();
    for row in family["mapping_sources"].as_array().unwrap() {
        let stat = string(&row["stat"]);
        assert!(mapped.contains(stat) && sources_seen.insert(stat));
        assert_eq!(row["set"], 1);
        assert!(
            row["path"] == "src/Data/SkillStatMap.lua"
                || row["path"] == family["skill_declaration"]["source"]["path"]
        );
        let body = span(&facts.sources, row);
        assert!(body.trim_start().starts_with(&format!("[\"{stat}\"] = {{")));
        for forbidden in [
            "function",
            "ExtraSkill",
            "ExtraMinionSkill",
            "ExtraSupport",
            "SkillData",
        ] {
            assert!(!body.contains(forbidden));
        }
        assert!(body.contains("mod("));
    }
    assert_eq!(sources_seen, mapped);
    check_descriptions(family, &facts.sources);
}

pub(super) fn check() {
    let facts = facts();
    let families = facts.proof["families"].as_array().unwrap();
    assert_eq!(families.len(), GEMS.len());
    let mut resolved = 0;
    let mut nulls = 0;
    for (family, gem) in families.iter().zip(GEMS) {
        assert_eq!(family["gem"], gem);
        assert_eq!(
            &family["mapping"],
            only(
                &read("dependencies.json")["mapping"],
                "source",
                &family["mapping"]["source"]
            )
        );
        check_family(family, facts);
        resolved += family["recipe"]["resolved"].as_array().unwrap().len();
        nulls += family["null_semantics"].as_array().unwrap().len();
    }
    assert_eq!((resolved, nulls), (18, 4));
}

fn rejected(f: impl FnOnce() + std::panic::UnwindSafe) {
    assert!(std::panic::catch_unwind(f).is_err());
}
pub(super) fn identity_controls() {
    let facts = facts();
    let family = &facts.proof["families"][0];
    check_family(family, facts);
    for pointer in [
        "/gem_identity/primary_effect_id",
        "/gem_identity/variant_id",
        "/skill_identity/id",
        "/gem_declaration/identity/primary_effect_id",
    ] {
        let mut bad = family.clone();
        *bad.pointer_mut(pointer).unwrap() = json!("unreviewed-missing-reference");
        rejected(|| check_identity(&bad, &facts.catalog));
    }
    let mut bad = family.clone();
    bad["gem_identity"]["constructed_additional_effects"] =
        json!([{"index":1,"id":"unreviewed-child"}]);
    rejected(|| check_identity(&bad, &facts.catalog));
    let mut catalog = facts.catalog.clone();
    catalog["missing_references"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "gem_key":family["gem_identity"]["key"],"kind":"primary_effect","index":null,
            "effect_id":family["skill_identity"]["id"]
        }));
    rejected(|| check_identity(family, &catalog));
    let mut gem = family["gem_identity"].clone();
    gem["name"] = json!("Barbs II");
    rejected(|| check_support_role(&gem, &family["skill_identity"]));
    let mut active = family["skill_identity"].clone();
    active["support"] = json!(false);
    rejected(|| check_support_role(&family["gem_identity"], &active));
    let mut auxiliary = family["gem_identity"].clone();
    auxiliary["effect_list"]
        .as_array_mut()
        .unwrap()
        .push(json!("unreviewed-child"));
    rejected(|| check_support_role(&auxiliary, &family["skill_identity"]));
}
pub(super) fn factory_controls() {
    let facts = facts();
    let family = &facts.proof["families"][0];
    let body = span(&facts.sources, &family["skill_declaration"]["source"]);
    check_primary_body(family, &body);
    // Exercise semantic checks without relying on an outer stale hash rejection.
    for addition in [
        "\taddMinionList = { \"new-child\" },\n",
        "\tminionList = { \"new-child\" },\n",
        "\tbaseMods = { mod(\"ExtraSkill\", \"LIST\", {skillId=\"new-child\"}) },\n",
        "\tunknownFactory = { \"new-child\" },\n",
        "\tpreFunc = function() end,\n",
    ] {
        let changed = body.replacen(
            "\tsupport = true,\n",
            &format!("\tsupport = true,\n{addition}"),
            1,
        );
        rejected(|| check_primary_body(family, &changed));
    }
}
pub(super) fn mapping_controls() {
    let facts = facts();
    let family = &facts.proof["families"][0];
    let body = span(&facts.sources, &family["skill_declaration"]["source"]);
    let mut removed = family.clone();
    removed["recipe"]["resolved"].as_array_mut().unwrap().pop();
    rejected(|| check_primary_body(&removed, &body));
    for field in ["aliases", "callbacks"] {
        let mut changed = family.clone();
        changed["recipe"]["resolved"][0]["mapping"][field] = json!(["unreviewed"]);
        rejected(|| check_recipe(&changed));
    }
    let mut extra = family.clone();
    extra["recipe"]["resolved"][0]["mapping"]["value"]["[1]"]["value"]["mod"]["name"] =
        json!("ExtraMinionSkill");
    rejected(|| check_recipe(&extra));
    let mut removed_source = family.clone();
    removed_source["mapping_sources"]
        .as_array_mut()
        .unwrap()
        .pop();
    rejected(|| check_family(&removed_source, facts));
    let mut duplicate = family.clone();
    let row = duplicate["recipe"]["resolved"][0].clone();
    duplicate["recipe"]["resolved"]
        .as_array_mut()
        .unwrap()
        .push(row);
    rejected(|| check_primary_body(&duplicate, &body));
}
pub(super) fn description_controls() {
    let facts = facts();
    for index in [1, 3, 4] {
        let family = &facts.proof["families"][index];
        check_descriptions(family, &facts.sources);
        let mut removed = family.clone();
        removed["null_semantics"].as_array_mut().unwrap().pop();
        rejected(|| check_descriptions(&removed, &facts.sources));
        let mut wrong_scope = family.clone();
        wrong_scope["null_semantics"][0]["scope"] = json!("skill_stat_descriptions");
        rejected(|| check_descriptions(&wrong_scope, &facts.sources));
        let mut wrong_description = family.clone();
        wrong_description["null_semantics"][0]["description"]["source"]["path"] =
            json!("src/Data/StatDescriptions/stat_descriptions.lua");
        rejected(|| check_descriptions(&wrong_description, &facts.sources));
        let mut no_semantics = family.clone();
        no_semantics["null_semantics"][0]["description"]["text"] = json!([]);
        rejected(|| check_descriptions(&no_semantics, &facts.sources));
    }
}
