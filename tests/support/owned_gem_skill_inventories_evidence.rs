//! Finite, authenticated constructor review, not a source-language evaluator.
use super::{GEMS, bounded_read, exact, hash, read, rejected, root};
use flate2::read::GzDecoder;
use poe_optimizer_data::skill_identities::{SkillIdentityCatalog, SkillIdentityData};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Read, sync::OnceLock};

const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const CATALOG: &str = "b9064ec447ccf3c3fc88244655a36bbb4453e50e16ffa64eda9880f756aed10e";
const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
struct Facts {
    proof: Value,
    catalog: Value,
    sources: BTreeMap<String, String>,
}
fn string(value: &Value) -> &str {
    value.as_str().unwrap()
}
fn pinned_json(pin: &Value, path: &str, digest: &str) -> Value {
    assert_eq!(pin["path"], path);
    assert_eq!(pin["sha256"], digest);
    let bytes = bounded_read(root().join(path), 4 * 1024 * 1024);
    assert_eq!(pin["bytes"], bytes.len());
    assert_eq!(hash(&bytes), digest);
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
            MANIFEST,
        );
        assert_eq!(manifest["upstream_revision"], REVISION);
        let catalog = pinned_json(
            &proof["catalog"],
            "data/owned/poe2/3887ae68/import/skill-identities.json",
            CATALOG,
        );
        let typed: SkillIdentityData = serde_json::from_value(catalog.clone()).unwrap();
        SkillIdentityCatalog::new(typed).unwrap();
        let expected = [
            "src/Classes/SkillsTab.lua",
            "src/Data/Gems.lua",
            "src/Data/Skills/act_int.lua",
            "src/Data/Skills/sup_int.lua",
            "src/Modules/CalcActiveSkill.lua",
            "src/Modules/CalcSetup.lua",
            "src/Modules/Data.lua",
        ];
        let pins = proof["sources"].as_array().unwrap();
        assert_eq!(pins.len(), expected.len());
        let mut sources = BTreeMap::new();
        let mut total = 0;
        for (pin, path) in pins.iter().zip(expected) {
            assert_eq!(pin["path"], path);
            let archive = if path == "src/Data/Skills/act_int.lua" {
                "data/owned/poe2/3887ae68/self-flat-minion-store/act_int.lua.gz".to_owned()
            } else {
                format!(
                    "data/owned/poe2/3887ae68/support-structural-inventories/sources/{}.gz",
                    path.replace('/', "_")
                )
            };
            assert_eq!(pin["archive"], archive);
            let bytes = bounded_read(root().join(archive), 2 * 1024 * 1024);
            assert_eq!(pin["archive_bytes"], bytes.len());
            assert_eq!(pin["archive_sha256"], hash(&bytes));
            let mut source = Vec::new();
            GzDecoder::new(bytes.as_slice())
                .take(4 * 1024 * 1024 + 1)
                .read_to_end(&mut source)
                .unwrap();
            total += source.len();
            assert!(source.len() <= 4 * 1024 * 1024 && total <= 4 * 1024 * 1024);
            assert_eq!(pin["bytes"], source.len());
            assert_eq!(pin["sha256"], hash(&source));
            assert!(!source.contains(&b'\r'));
            let authority = exact(&manifest["files"], "/path", &pin["path"]);
            assert_eq!(authority["bytes"], pin["bytes"]);
            assert_eq!(authority["sha256"], pin["sha256"]);
            assert!(
                sources
                    .insert(path.to_owned(), String::from_utf8(source).unwrap())
                    .is_none()
            );
        }
        let f = Facts {
            proof,
            catalog,
            sources,
        };
        boundaries(&f);
        f
    })
}
fn span(f: &Facts, row: &Value) -> String {
    let first = row
        .get("first_line")
        .unwrap_or(&row["line"])
        .as_u64()
        .unwrap() as usize;
    let last = row
        .get("last_line")
        .unwrap_or(&row["end_line"])
        .as_u64()
        .unwrap() as usize;
    assert!(first > 0 && first <= last && last - first < 1000);
    let lines: Vec<_> = f.sources[string(&row["path"])]
        .split_inclusive('\n')
        .collect();
    assert!(last <= lines.len());
    let body = lines[first - 1..last].concat();
    assert_eq!(hash(body.as_bytes()), string(&row["sha256"]));
    body
}
fn boundaries(f: &Facts) {
    let expected = [
        ("src/Modules/Data.lua", 56, 98),
        ("src/Modules/Data.lua", 910, 966),
        ("src/Modules/Data.lua", 979, 1068),
        ("src/Classes/SkillsTab.lua", 1242, 1322),
        ("src/Modules/CalcSetup.lua", 630, 666),
        ("src/Modules/CalcSetup.lua", 1987, 2107),
        ("src/Modules/CalcActiveSkill.lua", 144, 237),
        ("src/Modules/CalcActiveSkill.lua", 895, 955),
        ("src/Modules/CalcActiveSkill.lua", 1116, 1184),
    ];
    let rows = f.proof["boundaries"].as_array().unwrap();
    assert_eq!(rows.len(), expected.len());
    for (row, (path, first, last)) in rows.iter().zip(expected) {
        assert_eq!(row["path"], path);
        assert_eq!(row["first_line"], first);
        assert_eq!(row["last_line"], last);
        let _ = span(f, row);
    }
    let constructor = span(f, &rows[2]);
    assert!(constructor.starts_with("local function setupGem(gem, gemId)"));
    for marker in [
        "gem.grantedEffect = data.skills[gem.grantedEffectId]",
        "while gem[\"additionalGrantedEffectId\"..i] do",
        "table.insert(gem.grantedEffectList, skill)",
        "if gem.name:sub(1, 7) == \"Barbs I\" then",
        "if gem.grantedEffectDisplayOrder then",
        "setupGem(gem, gemId)",
    ] {
        assert!(constructor.contains(marker));
    }
    assert!(!constructor.contains("additionalStatSet"));
    assert!(span(f, &rows[4]).contains("if not grantedEffect or not grantedEffect.support then"));
    let active = span(f, &rows[5]);
    assert!(active.contains("for index, grantedEffect in ipairs(grantedEffectList) do"));
    assert!(active.contains("if not grantedEffect.support and not grantedEffect.hideFromSideBar"));
    assert!(active.contains("gemInstance.statSet[grantedEffect.id] or 1"));
    assert!(active.contains("calcs.createActiveSkill(activeEffect, appliedSupportList"));
    // Downstream Actor factories have separate ownership and remain unclosed.
    assert!(span(f, &rows[7]).contains("skillEffect.grantedEffect.addMinionList"));
    assert!(span(f, &rows[8]).contains("\"ExtraMinionSkill\""));
}

/// Constructor membership before activation, UI visibility or support acceptance.
/// Missing references are rejected before any potential list can be certified.
fn effects<'a>(gem: &Value, catalog: &'a Value) -> Vec<&'a Value> {
    assert!(
        !string(&gem["name"]).starts_with("Barbs I"),
        "unreviewed constructor branch"
    );
    assert!(
        !string(&gem["key"]).contains("Vaal"),
        "unreviewed constructor branch"
    );
    assert!(gem["display_order"].is_null());
    assert!(
        !catalog["missing_references"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["gem_key"] == gem["key"])
    );
    let declared = gem["declared_additional_effects"].as_array().unwrap();
    assert_eq!(gem["constructed_additional_effects"], json!(declared));
    let mut ids = vec![gem["primary_effect_id"].clone()];
    for (index, row) in declared.iter().enumerate() {
        assert_eq!(row["index"], index + 1);
        assert!(!ids.contains(&row["id"]), "duplicate effect");
        ids.push(row["id"].clone());
    }
    assert_eq!(gem["additional_effects"], json!(ids[1..]));
    assert_eq!(gem["effect_list"], json!(ids));
    ids.iter()
        .map(|id| exact(&catalog["skills"], "/id", id))
        .collect()
}
fn identity(row: &Value, f: &Facts, prior: &Value) {
    let gem = exact(&f.catalog["gems"], "/key", &row["gem_identity"]["key"]);
    assert_eq!(gem, &row["gem_identity"]);
    let selected = effects(gem, &f.catalog);
    assert_eq!(
        selected.len(),
        1,
        "reviewed four singleton effect inventories"
    );
    let skill = selected[0];
    assert_eq!(skill, &row["skill_identity"]);
    assert!(skill["from_tree"].is_null());
    for (field, rows, index) in [
        (
            "gem_declaration",
            "gem_declarations",
            &gem["winning_declaration"],
        ),
        (
            "skill_declaration",
            "skill_declarations",
            &skill["winning_declaration"],
        ),
    ] {
        assert_eq!(exact(&f.catalog[rows], "/index", index), &row[field]);
    }
    assert_eq!(row["gem_declaration"]["key"], gem["key"]);
    assert_eq!(
        row["gem_declaration"]["identity"]["primary_effect_id"],
        skill["id"]
    );
    assert_eq!(row["skill_declaration"]["id"], skill["id"]);
    assert_eq!(
        row["gem_declaration"]["identity"]["additional_effects"],
        gem["declared_additional_effects"]
    );
    assert_eq!(
        row["gem_declaration"]["identity"]["additional_stat_sets"],
        gem["declared_additional_stat_sets"]
    );
    let mapping = &row["mapping"];
    assert_eq!(
        mapping["source"],
        json!({"kind":"definition","value":{"kind":"gem","value":{
            "game_id":{"kind":"text","value":gem["game_id"]},
            "variant_id":{"kind":"text","value":gem["variant_id"]}
        }}})
    );
    assert_eq!(
        mapping["outcome"]["value"]["basis"],
        json!({"kind":"exact"})
    );
    assert_eq!(mapping["outcome"]["kind"], "mapped");
    let old = exact(&prior["definitions"], "/value/id/key", &row["gem"]);
    assert_eq!(
        mapping["outcome"]["value"]["target"],
        json!({"kind":"definition","value":{"kind":"gem","value":old["value"]["id"]}})
    );
    let support = skill["support"] == true;
    let role = if support {
        "support_assignment"
    } else {
        "skill_use"
    };
    assert_eq!(row["role"], role);
    assert_eq!(old["value"]["schema"]["value"]["roles"], json!([role]));
    let sm = &row["skill_mapping"];
    assert_eq!(
        sm["source"],
        json!({"kind":"definition","value":{"kind":"skill","value":{
            "effect_id":{"kind":"text","value":skill["id"]}
        }}})
    );
    assert_eq!(sm["outcome"]["kind"], "mapped");
    assert_eq!(sm["outcome"]["value"]["basis"], json!({"kind":"exact"}));
    let target = &sm["outcome"]["value"]["target"];
    assert_eq!(target["kind"], "definition");
    assert_eq!(target["value"]["kind"], "skill");
    let members = if support {
        json!([])
    } else {
        json!([target["value"]["value"]])
    };
    assert_eq!(
        old["value"]["schema"]["value"]["skills"]["members"],
        members
    );
}

// These lexical conventions describe only the pinned generated declarations.
// Exact source hashes are the authority; this is not a generic Lua parser.
fn fields(body: &str, indent: usize) -> Vec<String> {
    let prefix = "\t".repeat(indent);
    body.lines()
        .filter_map(|line| {
            let line = line.strip_prefix(&prefix)?;
            let (key, _) = line.split_once(" =")?;
            (!key.is_empty() && key.bytes().all(|x| x.is_ascii_alphanumeric() || x == b'_'))
                .then(|| key.to_owned())
        })
        .collect()
}
fn shapes(row: &Value, gem_body: &str, skill_body: &str) {
    assert_eq!(json!(fields(gem_body, 2)), row["gem_fields"]);
    assert_eq!(json!(fields(skill_body, 1)), row["skill_fields"]);
    assert!(gem_body.starts_with(&format!(
        "\t[\"{}\"] = {{",
        string(&row["gem_identity"]["key"])
    )));
    assert!(skill_body.starts_with(&format!(
        "skills[\"{}\"] = {{",
        string(&row["skill_identity"]["id"])
    )));
    assert!(gem_body.contains(&format!(
        "\t\tgrantedEffectId = \"{}\",",
        string(&row["skill_identity"]["id"])
    )));
    assert_eq!(
        skill_body.contains("\tsupport = true,"),
        row["role"] == "support_assignment"
    );
    assert!(!gem_body.contains("additionalGrantedEffectId"));
    assert!(!gem_body.contains("grantedEffectDisplayOrder"));
    assert!(
        !skill_body.contains("\tinitialFunc =") && !skill_body.contains("\tpreSkillTypeFunc =")
    );
    aliases(row, gem_body, skill_body);
}
fn aliases(row: &Value, gem_body: &str, skill_body: &str) {
    let expected = if row["gem"] == GEMS[2] {
        json!([{"index":1,"id":"IceNovaPlayerOnFrostbolt"},{"index":2,"id":"IceNovaColdInfusedPlayer"}])
    } else {
        json!([])
    };
    assert_eq!(
        row["gem_identity"]["declared_additional_stat_sets"],
        expected
    );
    let declared = expected.as_array().unwrap();
    let alias_lines: Vec<_> = gem_body
        .lines()
        .filter(|l| l.starts_with("\t\tadditionalStatSet"))
        .collect();
    assert_eq!(alias_lines.len(), declared.len());
    for (line, alias) in alias_lines.iter().zip(declared) {
        assert_eq!(
            *line,
            format!(
                "\t\tadditionalStatSet{} = \"{}\",",
                alias["index"],
                string(&alias["id"])
            )
        );
    }
    let sets: Vec<_> = skill_body
        .lines()
        .filter_map(|line| line.strip_prefix("\t\t[")?.strip_suffix("] = {"))
        .collect();
    let labels: Vec<_> = skill_body
        .lines()
        .filter_map(|line| line.strip_prefix("\t\t\tlabel = \"")?.strip_suffix("\","))
        .collect();
    assert_eq!(labels.len(), if row["gem"] == GEMS[2] { 2 } else { 1 });
    assert_eq!(
        sets,
        (1..=labels.len())
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(json!(labels), row["stat_set_labels"]);
}
pub(super) fn check(prior: &Value) {
    let f = facts();
    let rows = f.proof["families"].as_array().unwrap();
    assert_eq!(rows.len(), GEMS.len());
    assert_eq!(f.proof["scope"], read("authoring.json")["scope"]);
    for (row, gem) in rows.iter().zip(GEMS) {
        assert_eq!(row["gem"], gem);
        identity(row, f, prior);
        let gem_body = span(f, &row["gem_declaration"]["source"]);
        let skill_body = span(f, &row["skill_declaration"]["source"]);
        shapes(row, &gem_body, &skill_body);
        for alias in row["gem_identity"]["declared_additional_stat_sets"]
            .as_array()
            .unwrap()
        {
            assert!(
                !f.catalog["skills"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|s| s["id"] == alias["id"])
            );
        }
    }
    let control = &f.proof["unresolved_control"];
    assert_eq!(
        control["gem"],
        *exact(
            &f.catalog["gems"],
            "/key",
            &json!("Metadata/Items/Gems/SkillGemSkeletalSniper")
        )
    );
    let missing: Vec<_> = f.catalog["missing_references"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["gem_key"] == control["gem"]["key"])
        .collect();
    assert_eq!(json!(missing), control["missing_references"]);
    assert_eq!(missing.len(), 2);
    rejected(|| {
        effects(&control["gem"], &f.catalog);
    });
}
pub(super) fn identity_controls() {
    let f = facts();
    let prior = read("dependencies.json");
    let row = &f.proof["families"][1];
    for pointer in [
        "/mapping/source/value/value/variant_id/value",
        "/skill_mapping/source/value/value/effect_id/value",
        "/role",
    ] {
        let mut bad = row.clone();
        *bad.pointer_mut(pointer).unwrap() = json!("unreviewed");
        rejected(|| identity(&bad, f, &prior));
    }
    let mut bad = row.clone();
    bad["gem_identity"]["winning_declaration"] = json!(1);
    rejected(|| identity(&bad, f, &prior));
}
pub(super) fn reference_controls() {
    let f = facts();
    let gem = &f.proof["families"][1]["gem_identity"];
    assert_eq!(effects(gem, &f.catalog).len(), 1);
    for field in [
        "declared_additional_effects",
        "constructed_additional_effects",
    ] {
        let mut bad = gem.clone();
        bad[field] = json!([{"index":1,"id":"missing-effect"}]);
        rejected(|| {
            effects(&bad, &f.catalog);
        });
    }
    let mut bad = gem.clone();
    bad["effect_list"]
        .as_array_mut()
        .unwrap()
        .push(json!("extra-effect"));
    rejected(|| {
        effects(&bad, &f.catalog);
    });
    let missing = &f.proof["unresolved_control"]["gem"];
    rejected(|| {
        effects(missing, &f.catalog);
    });
    let mut cleared = f.catalog.clone();
    cleared["missing_references"] = json!([]);
    rejected(|| {
        effects(missing, &cleared);
    });
    let mut pruned = missing.clone();
    pruned["declared_additional_effects"] = json!([]);
    pruned["constructed_additional_effects"] = json!([]);
    rejected(|| {
        effects(&pruned, &f.catalog);
    });
}
pub(super) fn source_controls() {
    let f = facts();
    for row in f.proof["families"].as_array().unwrap() {
        let g = span(f, &row["gem_declaration"]["source"]);
        let s = span(f, &row["skill_declaration"]["source"]);
        shapes(row, &g, &s);
        for addition in [
            "\t\tadditionalGrantedEffectId1 = \"unknown\",\n",
            "\t\tgrantedEffectDisplayOrder = { 0 },\n",
            "\t\tnewFactory = true,\n",
        ] {
            rejected(|| shapes(row, &format!("{g}{addition}"), &s));
        }
        rejected(|| shapes(row, &g, &format!("{s}\tinitialFunc = function() end,\n")));
        let changed = if row["role"] == "support_assignment" {
            s.replace("\tsupport = true,", "\tsupport = false,")
        } else {
            format!("{s}\tsupport = true,\n")
        };
        rejected(|| shapes(row, &g, &changed));
    }
}
pub(super) fn alias_controls() {
    let f = facts();
    let row = &f.proof["families"][2];
    let g = span(f, &row["gem_declaration"]["source"]);
    let s = span(f, &row["skill_declaration"]["source"]);
    aliases(row, &g, &s);
    let mut dropped = row.clone();
    dropped["gem_identity"]["declared_additional_stat_sets"]
        .as_array_mut()
        .unwrap()
        .pop();
    rejected(|| aliases(&dropped, &g, &s));
    let removed = s.replace("\t\t\tlabel = \"Cold-Infused\",\n", "");
    rejected(|| aliases(row, &g, &removed));
    let extra = format!("{s}\t\t[3] = {{\n\t\t\tlabel = \"Invented from alias\",\n");
    rejected(|| aliases(row, &g, &extra));
    let changed = g.replace("additionalStatSet1", "additionalGrantedEffectId1");
    rejected(|| aliases(row, &changed, &s));
}
