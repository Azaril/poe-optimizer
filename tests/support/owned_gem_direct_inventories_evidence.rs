//! Reviewed ownership boundaries, not a general Lua parser or evaluator.
use super::{GEMS, SKILLS, bounded_read, exact, hash, potential, read, rejected, root};
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Read, sync::OnceLock};

const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
struct Facts {
    proof: Value,
    established: Value,
    sources: BTreeMap<String, String>,
    boundaries: BTreeMap<String, String>,
}
fn string(v: &Value) -> &str {
    v.as_str().unwrap()
}
fn facts() -> &'static Facts {
    static FACTS: OnceLock<Facts> = OnceLock::new();
    FACTS.get_or_init(|| {
        let proof = read("evidence.json");
        assert_eq!(proof["source_revision"], REVISION);
        let pin = &proof["source_manifest"];
        assert_eq!(
            pin["path"],
            "crates/poe-optimizer-pob/data/pob-source-manifest.json"
        );
        assert_eq!(pin["sha256"], MANIFEST);
        let bytes = bounded_read(root().join(string(&pin["path"])), 1024 * 1024);
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(hash(&bytes), MANIFEST);
        let manifest: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(manifest["upstream_revision"], REVISION);
        let prior_pins = proof["potential_membership_packet"].as_array().unwrap();
        assert_eq!(prior_pins.len(), 4);
        for (pin, name) in prior_pins.iter().zip([
            "authoring.json",
            "dependencies.json",
            "evidence.json",
            "migration.json",
        ]) {
            assert_eq!(
                pin["path"],
                format!("data/owned/poe2/3887ae68/gem-skill-inventories/{name}")
            );
            let bytes = bounded_read(potential::data().join(name), 1024 * 1024);
            assert_eq!(pin["bytes"], bytes.len());
            assert_eq!(pin["sha256"], hash(&bytes));
        }
        let established: Value = serde_json::from_slice(&bounded_read(
            potential::data().join("evidence.json"),
            1024 * 1024,
        ))
        .unwrap();
        let expected = [
            "src/Export/Scripts/skills.lua",
            "src/Modules/Data.lua",
            "src/Modules/CalcSetup.lua",
            "src/Modules/CalcActiveSkill.lua",
            "src/Modules/ModParser.lua",
            "src/Data/Skills/act_int.lua",
            "src/Data/Skills/sup_int.lua",
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
            let bytes = bounded_read(root().join(archive), 1024 * 1024);
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
        let expected = [
            ("export-primary", "src/Export/Scripts/skills.lua", 124, 483),
            (
                "export-level-quality",
                "src/Export/Scripts/skills.lua",
                227,
                320,
            ),
            ("export-statsets", "src/Export/Scripts/skills.lua", 485, 909),
            (
                "export-gem-writer",
                "src/Export/Scripts/skills.lua",
                918,
                1009,
            ),
            ("gem-constructor", "src/Modules/Data.lua", 979, 1069),
            ("external-properties", "src/Modules/CalcSetup.lua", 543, 584),
            (
                "external-generated-groups",
                "src/Modules/CalcSetup.lua",
                1671,
                1734,
            ),
            ("thorns-factory", "src/Modules/CalcSetup.lua", 1784, 1834),
            ("active-dispatch", "src/Modules/CalcSetup.lua", 1987, 2123),
            (
                "skill-stat-dispatch",
                "src/Modules/CalcActiveSkill.lua",
                80,
                140,
            ),
            (
                "skill-constructor",
                "src/Modules/CalcActiveSkill.lua",
                144,
                234,
            ),
            (
                "source-properties",
                "src/Modules/CalcActiveSkill.lua",
                236,
                262,
            ),
            (
                "totem-provenance",
                "src/Modules/CalcActiveSkill.lua",
                355,
                393,
            ),
            (
                "effect-stat-consumer",
                "src/Modules/CalcActiveSkill.lua",
                795,
                808,
            ),
            (
                "minion-factory",
                "src/Modules/CalcActiveSkill.lua",
                895,
                1031,
            ),
            (
                "minion-skills",
                "src/Modules/CalcActiveSkill.lua",
                1116,
                1185,
            ),
            (
                "external-extra-support",
                "src/Modules/ModParser.lua",
                2219,
                2253,
            ),
            (
                "support-receiving-modifiers",
                "src/Modules/CalcActiveSkill.lua",
                731,
                770,
            ),
        ];
        let rows = proof["boundaries"].as_array().unwrap();
        assert_eq!(rows.len(), expected.len());
        let mut boundaries = BTreeMap::new();
        for (row, (id, path, first, last)) in rows.iter().zip(expected) {
            assert_eq!(row["id"], id);
            assert_eq!(row["path"], path);
            assert_eq!(row["first_line"], first);
            assert_eq!(row["last_line"], last);
            assert!(
                boundaries
                    .insert(id.to_owned(), span(&sources, row))
                    .is_none()
            );
        }
        ownership(&boundaries);
        Facts {
            proof,
            established,
            sources,
            boundaries,
        }
    })
}
fn span(sources: &BTreeMap<String, String>, row: &Value) -> String {
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
    let lines: Vec<_> = sources[string(&row["path"])]
        .split_inclusive('\n')
        .collect();
    assert!(last <= lines.len());
    let body = lines[first - 1..last].concat();
    assert_eq!(hash(body.as_bytes()), string(&row["sha256"]));
    body
}
fn require(body: &str, markers: &[&str]) {
    for marker in markers {
        assert!(
            body.contains(marker),
            "missing reviewed ownership edge: {marker}"
        );
    }
}
fn ownership(b: &BTreeMap<String, String>) {
    require(
        &b["export-primary"],
        &[
            "local granted = dat(\"GrantedEffects\"):GetRow(\"Id\", grantedId)",
            "dat(\"GemEffects\"):GetRow(\"GrantedEffect\", granted)",
            "dat(\"GemEffects\"):GetRow(\"AdditionalGrantedEffects\", granted)",
        ],
    );
    require(
        &b["export-level-quality"],
        &[
            "dat(\"GrantedEffectsPerLevel\"):GetRowList(\"GrantedEffect\", granted)",
            "dat(\"GrantedEffectStatSetsPerLevel\"):GetRowList(\"GrantedEffect\", granted)",
            "dat(\"GrantedEffectQualityStats\"):GetRow(\"GrantedEffect\", granted)",
        ],
    );
    require(
        &b["export-statsets"],
        &[
            "directiveTable.set = function(state, args, out)",
            "dat(\"GrantedEffectStatSets\"):GetRow(\"Id\", statSetId)",
            "local skill = state.skill",
            "skill.sets[args] = set",
            "state.set = nil",
        ],
    );
    require(
        &b["export-gem-writer"],
        &[
            "for skillGem in dat(\"SkillGems\"):Rows() do",
            "for _, gemEffect in ipairs(skillGem.GemEffects) do",
            "gemEffect.GrantedEffect.Id",
            "gemEffect.GrantedEffect.AdditionalStatSets",
            "gemEffect.AdditionalGrantedEffects",
            "additionalGrantedEffectId",
            "additionalStatSet",
            "out:close()",
        ],
    );
    require(
        &b["gem-constructor"],
        &[
            "local function setupGem(gem, gemId)",
            "gem.grantedEffect = data.skills[gem.grantedEffectId]",
            "while gem[\"additionalGrantedEffectId\"..i] do",
            "table.insert(gem.grantedEffectList, skill)",
            "if gem.name:sub(1, 7) == \"Barbs I\"",
            "TriggeredBarbsPlayer",
            "if gem.grantedEffectDisplayOrder then",
            "local loc, _ = gemId:find('Vaal')",
        ],
    );
    require(
        &b["external-properties"],
        &[
            "return env.modDB:Tabulate(\"LIST\", gemCfg, \"GemProperty\")",
            "effect[value.key] = (effect[value.key] or 0) + value.value",
            "env.modDB:List(socketCfg, \"SocketProperty\")",
            "modLib.setSource(value.value, modSource or groupCfg.slotName or \"\")",
        ],
    );
    require(
        &b["external-generated-groups"],
        &[
            "grantedSkill.sourceNode = node",
            "env.grantedSkills = tableConcat(env.grantedSkillsNodes, env.grantedSkillsItems)",
            "group.sourceItem = grantedSkill.sourceItem",
            "group.sourceNode = grantedSkill.sourceNode",
            "activeGemInstance.fromItem = grantedSkill.sourceItem ~= nil",
            "activeGemInstance.gemId = data.gemForSkill[data.skills[grantedSkill.skillId]]",
        ],
    );
    require(
        &b["external-extra-support"],
        &[
            "local function extraSupport(name, level, slot)",
            "mod(\"ExtraSupport\", \"LIST\"",
            "if additional.support then",
            "mod(\"ExtraSkill\", \"LIST\"",
            "{ type = \"SocketedIn\", slotName = slot }",
            "data.gems[gemId][\"additionalGrantedEffectId\"..i]",
        ],
    );
    require(
        &b["thorns-factory"],
        &[
            "local hasThornsDamage = modDBHasThornsDamage(env.modDB)",
            "if grantedEffect and grantedEffect.grantsThornsDamage then",
            "source = \"Thorns\"",
            "skillId = \"ThornsPlayer\"",
        ],
    );
    require(
        &b["active-dispatch"],
        &[
            "for index, grantedEffect in ipairs(grantedEffectList) do",
            "if not grantedEffect.support and not grantedEffect.hideFromSideBar",
            "srcInstance = gemInstance",
            "calcs.createActiveSkill(activeEffect, appliedSupportList",
            "t_insert(env.player.activeSkillList, activeSkill)",
        ],
    );
    require(
        &b["skill-stat-dispatch"],
        &[
            "calcLib.buildSkillInstanceStats(skillEffect, grantedEffect, set",
            "modOrGroup.source = string.format(\"Skill:%s\", grantedEffect.id)",
            "mod.source = string.format(\"Skill:%s\", grantedEffect.id)",
            "local grantedEffect = skillEffect.grantedEffect",
            "mergeStatSet(set, nil, skillEffect, grantedEffect",
        ],
    );
    require(
        &b["skill-constructor"],
        &[
            "function calcs.createActiveSkill(activeEffect, supportList, env, actor, socketGroup, summonSkill)",
            "activeEffect = activeEffect",
            "actor = actor",
            "summonSkill = summonSkill",
            "activeSkill.effectList = { activeEffect }",
            "t_insert(activeSkill.effectList, supportEffect)",
            "for _, skillType in pairs(supportEffect.grantedEffect.addSkillTypes) do",
            "activeSkill.skillTypes[skillType] = true",
        ],
    );
    require(
        &b["source-properties"],
        &[
            "local sourceGem = activeEffect.srcInstance",
            "supportEffect.isSupporting[sourceGem]",
            "calcs.mergeSkillInstanceMods(env, modList, supportEffect)",
        ],
    );
    require(
        &b["totem-provenance"],
        &[
            "totemBase.grantedEffect = activeSkill.activeEffect.grantedEffect",
            "support.isSupporting[activeSkill.activeEffect.srcInstance]",
            "totemBase.grantedEffect = support.gemData.grantedEffect",
        ],
    );
    require(
        &b["effect-stat-consumer"],
        &["calcs.mergeSkillInstanceMods(env, skillModList, activeEffect, activeStatSet"],
    );
    require(
        &b["minion-factory"],
        &[
            "activeSkill.skillData[value.key] = value.value",
            "minionList = copyTable(activeGrantedEffect.minionList)",
            "skillEffect.grantedEffect.support and skillEffect.grantedEffect.addMinionList",
            "activeSkill.minionList = minionList",
            "activeSkill.minion = minion",
            "minion.minionData = env.data.minions[minionType]",
        ],
    );
    require(
        &b["minion-skills"],
        &[
            "function calcs.createMinionSkills(env, activeSkill)",
            "local minion = activeSkill.minion",
            "\"ExtraMinionSkill\"",
            "minion.mainSkill = minion.activeSkillList[skillIndex]",
        ],
    );
    require(
        &b["support-receiving-modifiers"],
        &[
            "for _, skillEffect in pairs(activeSkill.effectList) do",
            "if skillEffect.grantedEffect.support then",
            "calcs.mergeSkillInstanceMods(env, skillModList, skillEffect)",
            "activeSkill.triggeredBy = skillEffect",
        ],
    );
}
fn family(row: &Value, established: &Value, body: &str, prior: &Value) {
    let index = GEMS.iter().position(|id| row["gem"] == *id).unwrap();
    assert_eq!(row["gem"], established["gem"]);
    assert_eq!(row["primary_effect"], established["skill_identity"]["id"]);
    assert_eq!(row["source"], established["skill_declaration"]["source"]);
    assert_eq!(row["direct_actor_members"], json!([]));
    let gem = &established["gem_identity"];
    assert!(!string(&gem["name"]).starts_with("Barbs I"));
    assert!(!string(&gem["key"]).contains("Vaal"));
    for field in [
        "declared_additional_effects",
        "constructed_additional_effects",
        "additional_effects",
    ] {
        assert_eq!(
            gem[field],
            json!([]),
            "no unreviewed additional carrier effect"
        );
    }
    assert_eq!(gem["effect_list"], json!([row["primary_effect"]]));
    assert_eq!(
        established["skill_identity"]["support"],
        if index == 0 { json!(true) } else { Value::Null }
    );
    assert!(body.starts_with(&format!(
        "skills[\"{}\"] = {{",
        string(&row["primary_effect"])
    )));
    // An explicit source factory outside the reviewed primary chain cannot be
    // accepted on the strength of a primary-only catalog result.
    for field in ["grantsThornsDamage", "initialFunc", "preSkillTypeFunc"] {
        assert!(!body.contains(&format!("\t{field} =")));
    }
    let (obligation, descriptions): (&str, &[&str]) = match index {
        0 => (
            "supported-hit-freeze-ground",
            &[
                "Supports any skill that Hits enemies, causing it to create patches of Chilled Ground around Frozen enemies.",
                "\tsupport = true,",
                "\taddSkillTypes = { SkillType.CreatesGroundEffect, },",
                "{ \"chilling_ice_freezing_area_base_radius\", 30 }",
                "{ \"chilling_ice_create_chilled_ground_on_freeze_base_duration_ms\", 8000 }",
            ],
        ),
        1 => (
            "orb-exposure-detonation-infusion",
            &[
                "Create a pulsing Orb of frost.",
                "leaving behind a Cold Infusion.",
            ],
        ),
        2 => (
            "frostbolt-origin-infusion-ground",
            &[
                "originate from the Frostbolt",
                "leave a patch of Chilled Ground.",
            ],
        ),
        3 => (
            "offering-spike-minion-lifecycle",
            &[
                "The bone spike is itself a Minion.",
                "If it dies, the effect ends immediately.",
            ],
        ),
        _ => unreachable!(),
    };
    assert_eq!(row["creation_obligation"], obligation);
    require(body, descriptions);
    if index == 0 {
        assert!(row["descendant_owner"].is_null());
        assert_eq!(row["receiving_subject"], "exact_supported_skill");
        assert_eq!(
            row["unresolved_constants"],
            json!([
                {"stat":"chilling_ice_freezing_area_base_radius","value":30},
                {"stat":"chilling_ice_create_chilled_ground_on_freeze_base_duration_ms","value":8000}
            ])
        );
    } else {
        let child = exact(&prior["skills"], "/value/id/key", &json!(SKILLS[index - 1]));
        assert_eq!(row["descendant_owner"], child["value"]["id"]);
        for field in ["grants", "actors", "skill_grants"] {
            assert_eq!(
                child["value"]["schema"]["value"]["declarations"][field]["closure"]["kind"],
                "partial"
            );
        }
    }
}
pub(super) fn check(prior: &Value) {
    let f = facts();
    assert_eq!(f.proof["scope"], read("authoring.json")["scope"]);
    let rows = f.proof["families"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    for (row, id) in rows.iter().zip(GEMS) {
        assert_eq!(row["gem"], id);
        let established = exact(&f.established["families"], "/gem", &row["gem"]);
        family(row, established, &span(&f.sources, &row["source"]), prior);
    }
}
pub(super) fn ownership_controls() {
    let f = facts();
    for (id, from, to) in [
        (
            "external-generated-groups",
            "group.sourceItem = grantedSkill.sourceItem",
            "group.sourceItem = nil",
        ),
        (
            "external-generated-groups",
            "group.sourceNode = grantedSkill.sourceNode",
            "group.sourceNode = nil",
        ),
        (
            "external-extra-support",
            "mod(\"ExtraSkill\", \"LIST\"",
            "mod(\"IgnoredSkill\", \"LIST\"",
        ),
        (
            "skill-stat-dispatch",
            "string.format(\"Skill:%s\", grantedEffect.id)",
            "string.format(\"Gem:%s\", grantedEffect.id)",
        ),
        (
            "minion-factory",
            "activeSkill.minion = minion",
            "activeEffect.gemData.minion = minion",
        ),
        (
            "source-properties",
            "supportEffect.isSupporting[sourceGem]",
            "supportEffect.isSupporting[activeSkill]",
        ),
        (
            "support-receiving-modifiers",
            "calcs.mergeSkillInstanceMods(env, skillModList, skillEffect)",
            "calcs.mergeSkillInstanceMods(env, skillEffect.modList, skillEffect)",
        ),
        (
            "skill-constructor",
            "activeSkill.skillTypes[skillType] = true",
            "supportEffect.grantedEffect.skillTypes[skillType] = true",
        ),
    ] {
        let mut bad = f.boundaries.clone();
        let body = bad.get_mut(id).unwrap();
        assert!(body.contains(from));
        *body = body.replace(from, to);
        rejected(|| ownership(&bad));
    }
}
pub(super) fn factory_controls() {
    let f = facts();
    let prior = read("dependencies.json");
    for row in f.proof["families"].as_array().unwrap() {
        let established = exact(&f.established["families"], "/gem", &row["gem"]);
        let body = span(&f.sources, &row["source"]);
        for field in ["grantsThornsDamage", "initialFunc", "preSkillTypeFunc"] {
            let changed = format!("{body}\t{field} = true,\n");
            rejected(|| family(row, established, &changed, &prior));
        }
        for (field, value) in [
            ("name", json!("Barbs III")),
            ("key", json!("Metadata/Items/Gems/SkillGemVaalUnreviewed")),
            (
                "constructed_additional_effects",
                json!([{"index":1,"id":"unreviewed"}]),
            ),
        ] {
            let mut changed = established.clone();
            changed["gem_identity"][field] = value;
            rejected(|| family(row, &changed, &body, &prior));
        }
    }
}
pub(super) fn descendant_controls() {
    let f = facts();
    let prior = read("dependencies.json");
    for row in f.proof["families"].as_array().unwrap() {
        let established = exact(&f.established["families"], "/gem", &row["gem"]);
        let body = span(&f.sources, &row["source"]);
        let without_description = body
            .lines()
            .filter(|line| !line.starts_with("\tdescription ="))
            .collect::<Vec<_>>()
            .join("\n");
        rejected(|| family(row, established, &without_description, &prior));
        let mut changed = row.clone();
        changed["creation_obligation"] = json!("none");
        rejected(|| family(&changed, established, &body, &prior));
        let mut changed = row.clone();
        changed["descendant_owner"] =
            established["mapping"]["outcome"]["value"]["target"]["value"]["value"].clone();
        rejected(|| family(&changed, established, &body, &prior));
    }
    let row = &f.proof["families"][0];
    let established = exact(&f.established["families"], "/gem", &row["gem"]);
    let body = span(&f.sources, &row["source"]);
    let mut changed = row.clone();
    changed["receiving_subject"] = json!("physical_gem");
    rejected(|| family(&changed, established, &body, &prior));
    let mut changed = row.clone();
    changed["unresolved_constants"] = json!([]);
    rejected(|| family(&changed, established, &body, &prior));
}
