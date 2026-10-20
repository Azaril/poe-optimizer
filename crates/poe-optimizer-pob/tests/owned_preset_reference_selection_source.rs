//! Authenticated source COMPONENT execution, not a complete PoB/build oracle.
//! The host constructs identity-only activeSkill objects; it does not execute
//! calcs.createActiveSkill, initialize lazy stat maps, or authorize native import.
#![cfg(not(target_arch = "wasm32"))]

use flate2::read::GzDecoder;
use mlua::{Function, Lua, Table, Value};
use poe_optimizer_pob::source;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf, sync::OnceLock};

const DIRECTORY: &str = "data/owned/poe2/3887ae68/preset-reference-selection-source";
const AUTHORING_SHA: &str = "e1b72a92c6f3213a255631e26b7353a02f9228f2279fb48ba230e8b1d2090699";
const MANIFEST_SHA: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const IDS: [&str; 8] = [
    "SummonSkeletalArsonistsPlayer",
    "SummonSkeletalSnipersPlayer",
    "SummonSkeletalFrostMagesPlayer",
    "SummonSkeletalReaversPlayer",
    "SummonSandDjinnPlayer",
    "SummonWaterDjinnPlayer",
    "IceNovaPlayer",
    "FireboltPlayer",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn text(value: &Json) -> &str {
    value.as_str().unwrap()
}
fn pinned_file(pin: &Json) -> Vec<u8> {
    let bytes = fs::read(root().join(text(&pin["path"]))).unwrap();
    assert_eq!(json!(bytes.len()), pin["bytes"]);
    assert_eq!(hash(&bytes), text(&pin["sha256"]));
    bytes
}
struct Packet {
    evidence: Json,
    catalog: Json,
    sources: BTreeMap<String, String>,
}
impl Packet {
    fn span(&self, pin: &Json) -> String {
        let first = pin["first_line"].as_u64().unwrap() as usize;
        let last = pin["last_line"].as_u64().unwrap() as usize;
        let body = self.sources[text(&pin["path"])]
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(body.lines().count(), last - first + 1);
        assert_eq!(json!(body.len()), pin["bytes"]);
        assert_eq!(hash(body.as_bytes()), text(&pin["sha256"]));
        body
    }
    fn boundary(&self, name: &str) -> String {
        self.span(&self.evidence["boundaries"][name])
    }
    fn declaration(&self, row: &Json) -> String {
        let pin = &row["source"];
        let first = pin["line"].as_u64().unwrap() as usize;
        let last = pin["end_line"].as_u64().unwrap() as usize;
        let body = self.sources[text(&pin["path"])]
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(body.lines().count(), last - first + 1);
        assert_eq!(hash(body.as_bytes()), text(&pin["sha256"]));
        body
    }
}
fn packet() -> &'static Packet {
    static PACKET: OnceLock<Packet> = OnceLock::new();
    PACKET.get_or_init(|| {
        let dir = root().join(DIRECTORY);
        let bytes = fs::read(dir.join("authoring.json")).unwrap();
        assert_eq!(hash(&bytes), AUTHORING_SHA);
        let authoring: Json = serde_json::from_slice(&bytes).unwrap();
        for (name, pin) in authoring["artifacts"].as_object().unwrap() {
            let bytes = fs::read(dir.join(name)).unwrap();
            assert_eq!(json!(bytes.len()), pin["bytes"]);
            assert_eq!(hash(&bytes), text(&pin["sha256"]));
        }
        let evidence: Json =
            serde_json::from_slice(&fs::read(dir.join("evidence.json")).unwrap()).unwrap();
        assert_eq!(source::manifest_sha256(), MANIFEST_SHA);
        let manifest: Json =
            serde_json::from_slice(&pinned_file(&evidence["source_manifest"])).unwrap();
        assert_eq!(text(&evidence["source_manifest"]["sha256"]), MANIFEST_SHA);
        assert_eq!(
            text(&evidence["source_revision"]),
            source::UPSTREAM_REVISION
        );
        let catalog = serde_json::from_slice(&pinned_file(&evidence["catalog"])).unwrap();
        pinned_file(&evidence["original"]);
        pinned_file(&evidence["license"]);
        let mut sources = BTreeMap::new();
        assert_eq!(evidence["source_files"].as_array().unwrap().len(), 9);
        for pin in evidence["source_files"].as_array().unwrap() {
            assert!(manifest["files"].as_array().unwrap().contains(pin));
            let path = text(&pin["path"]);
            let body =
                source::read_verified_text(&root().join("vendor/path-of-building-poe2"), path)
                    .unwrap();
            assert_eq!(json!(body.len()), pin["bytes"]);
            assert_eq!(hash(body.as_bytes()), text(&pin["sha256"]));
            assert!(sources.insert(path.into(), body).is_none());
        }
        let export = &evidence["retained_export"];
        let compressed = fs::read(dir.join(text(&export["file"]))).unwrap();
        assert_eq!(json!(compressed.len()), export["compressed_bytes"]);
        assert_eq!(hash(&compressed), text(&export["compressed_sha256"]));
        let mut raw = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(1024 * 1024)
            .read_to_end(&mut raw)
            .unwrap();
        assert_eq!(json!(raw.len()), export["raw_bytes"]);
        assert_eq!(hash(&raw), text(&export["raw_sha256"]));
        assert_eq!(raw, sources["src/Modules/BuildExportPoE2.lua"].as_bytes());
        let result = Packet {
            evidence,
            catalog,
            sources,
        };
        for pin in result.evidence["boundaries"].as_object().unwrap().values() {
            result.span(pin);
        }
        result
    })
}

/// Runs actual source constants/declarations/constructor with explicit host
/// infrastructure. Cosmetic copyTable and identity-only activeSkill objects are
/// test infrastructure, not substitutes for any claimed numerical calculation.
fn components(jit: bool) -> Lua {
    let p = packet();
    let lua = Lua::new();
    lua.set_memory_limit(128 * 1024 * 1024).unwrap();
    lua.load(if jit {
        "require('jit').on()"
    } else {
        "require('jit').off()"
    })
    .exec()
    .unwrap();
    lua.load(
        r#"
        function copyTable(t)
            local out = {}
            for k,v in pairs(t) do out[k] = type(v) == "table" and copyTable(v) or v end
            return out
        end
        bit = require("bit")
        "#,
    )
    .exec()
    .unwrap();
    lua.load(&p.sources["src/Data/Global.lua"])
        .set_name("@authenticated-src/Data/Global.lua")
        .exec()
        .unwrap();
    let mut definitions = p.boundary("mod_helpers");
    definitions.push_str(
        "\nlocal mod,flag,skill=makeSkillMod,makeFlagMod,makeSkillDataMod\nlocal skills={}\n",
    );
    for row in p.evidence["families"].as_array().unwrap() {
        definitions.push_str(&p.declaration(&row["skill_declaration"]));
    }
    for row in p.evidence["extra_declarations"].as_array().unwrap() {
        definitions.push_str(&p.declaration(row));
    }
    // Exact identity initialization from Data.lua:931–932, without claiming the
    // omitted stat-map initialization or its mutable hasGlobalEffect metadata.
    definitions.push_str(
        "\nfor skillId,grantedEffect in pairs(skills) do\n\
         grantedEffect.id=skillId\n\
         grantedEffect.modSource='Skill:'..skillId\n\
         end\nreturn skills",
    );
    let skills: Table = lua
        .load(definitions)
        .set_name("@authenticated-primary-declarations")
        .eval()
        .unwrap();
    let gems: Table = lua
        .load(&p.sources["src/Data/Gems.lua"])
        .set_name("@authenticated-src/Data/Gems.lua")
        .eval()
        .unwrap();
    let data = lua.create_table().unwrap();
    data.set("skills", skills).unwrap();
    for name in [
        "gemForSkill",
        "gemsByGameId",
        "gemNameForModSource",
        "gemForBaseName",
    ] {
        data.set(name, lua.create_table().unwrap()).unwrap();
    }
    lua.globals().set("data", data).unwrap();
    lua.globals().set("allGems", gems).unwrap();
    lua.load(format!(
        "{}\nconstructGem=setupGem",
        p.boundary("constructor")
    ))
    .set_name("@authenticated-Data.setupGem")
    .exec()
    .unwrap();
    // The loop and all admission conditions are verbatim source. Its body is an
    // explicit identity-only host observation; no effect preparation is run.
    lua.load(format!(
        "function admitted(group)\nlocal out={{}}\n{}\n\
         out[#out+1]={{activeEffect={{srcInstance=gemInstance,grantedEffect=grantedEffect}}}}\n\
         end end end end\nreturn out end\n\
         function selected(group,mode,list,groupIndex,selectedGroup)\n\
         local m_min=math.min\nlocal index=groupIndex\n\
         local env={{mode=mode,mainSocketGroup=selectedGroup,player={{}}}}\n\
         local socketGroupSkillList=list\n{}\nreturn env.player.mainSkill end\n\
         function loaded(attributes)\nlocal node={{attrib=attributes}}\nlocal socketGroup={{}}\n\
         {}\nreturn socketGroup end",
        p.boundary("admission_prefix"),
        p.boundary("select_group"),
        p.boundary("load_selectors"),
    ))
    .set_name("@authenticated-reference-components-with-identity-host")
    .exec()
    .unwrap();
    lua.load(format!(
        "local t_insert=table.insert\n{}\n\
         exportActive=activeSkillGems\nfunction exportSelected(group)\n{}\n{}\n\
         return activeGem\nend\nend",
        p.boundary("export_active"),
        p.boundary("export_group_gate"),
        p.boundary("export_reference"),
    ))
    .set_name("@authenticated-export-reference-components")
    .exec()
    .unwrap();
    lua
}
fn construct(lua: &Lua, primary: &str) -> Table {
    let family = packet().evidence["families"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["primary"] == primary)
        .unwrap();
    let key = text(&family["gem"]["key"]);
    let all: Table = lua.globals().get("allGems").unwrap();
    let raw: Table = all.get(key).unwrap();
    let gem: Table = lua
        .globals()
        .get::<Function>("copyTable")
        .unwrap()
        .call(raw)
        .unwrap();
    lua.globals()
        .get::<Function>("constructGem")
        .unwrap()
        .call::<()>((gem.clone(), key))
        .unwrap();
    gem
}
fn occurrence(lua: &Lua, gem: Table, name: &str) -> Table {
    let row = lua.create_table().unwrap();
    row.set("gemData", gem).unwrap();
    row.set("enabled", true).unwrap();
    row.set("enableGlobal1", true).unwrap();
    row.set("enableGlobal2", true).unwrap();
    row.set("testSource", name).unwrap();
    row
}
fn group(lua: &Lua, rows: &[Table]) -> Table {
    let group = lua.create_table().unwrap();
    group.set("enabled", true).unwrap();
    group
        .set("gemList", lua.create_sequence_from(rows.to_vec()).unwrap())
        .unwrap();
    group.set("mainActiveSkill", 1).unwrap();
    group.set("mainActiveSkillCalcs", 1).unwrap();
    group
}
fn admitted(lua: &Lua, group: &Table) -> Table {
    lua.globals()
        .get::<Function>("admitted")
        .unwrap()
        .call(group.clone())
        .unwrap()
}
fn selected(lua: &Lua, group: &Table, mode: &str, list: &Table) -> Option<Table> {
    lua.globals()
        .get::<Function>("selected")
        .unwrap()
        .call((group.clone(), mode, list.clone(), 1, 1))
        .unwrap()
}
fn source_of(active: &Table) -> Table {
    active
        .get::<Table>("activeEffect")
        .unwrap()
        .get("srcInstance")
        .unwrap()
}
fn effect_of(active: &Table) -> String {
    active
        .get::<Table>("activeEffect")
        .unwrap()
        .get::<Table>("grantedEffect")
        .unwrap()
        .get("id")
        .unwrap()
}

#[test]
fn source_packet_binds_full_definitions_and_the_exact_ten_original_pairs() {
    let p = packet();
    assert_eq!(
        p.evidence["candidate_tokens"],
        json!(["absent", "nil", "1"])
    );
    for key in [
        "complete_evaluator_started",
        "full_build_parity",
        "numerical_parity",
        "native_runtime_authority",
        "import_accounting_implemented",
    ] {
        assert_eq!(p.evidence["scope"][key], false);
    }
    assert_eq!(p.evidence["families"].as_array().unwrap().len(), IDS.len());
    for (family, id) in p.evidence["families"].as_array().unwrap().iter().zip(IDS) {
        assert_eq!(family["primary"], id);
        for (field, collection) in [
            ("gem", "gems"),
            ("skill", "skills"),
            ("gem_declaration", "gem_declarations"),
            ("skill_declaration", "skill_declarations"),
        ] {
            assert_eq!(
                p.catalog[collection]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|entry| **entry == family[field])
                    .count(),
                1
            );
        }
        assert_eq!(family["gem"]["primary_effect_id"], id);
        assert_eq!(family["gem"]["effect_list"][0], id);
        assert!(family["gem"]["display_order"].is_null());
        assert_eq!(family["skill"]["id"], id);
        assert_eq!(family["skill_declaration"]["id"], id);
        assert_eq!(
            family["skill"]["winning_declaration"],
            family["skill_declaration"]["index"]
        );
        assert_eq!(
            family["gem"]["winning_declaration"],
            family["gem_declaration"]["index"]
        );
        p.declaration(&family["gem_declaration"]);
        p.declaration(&family["skill_declaration"]);
    }
    let bytes = pinned_file(&p.evidence["original"]);
    let xml = std::str::from_utf8(&bytes).unwrap();
    let doc = roxmltree::Document::parse(xml).unwrap();
    let nodes: Vec<_> = doc.descendants().filter(|node| node.is_element()).collect();
    let pairs = p.evidence["original_pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 10);
    for pair in pairs {
        let g = nodes[pair["group_ordinal"].as_u64().unwrap() as usize];
        let s = nodes[pair["primary_ordinal"].as_u64().unwrap() as usize];
        assert_eq!(g.tag_name().name(), "Skill");
        assert_eq!(s.tag_name().name(), "Gem");
        assert_eq!(s.parent(), Some(g));
        assert_eq!(g.children().find(|node| node.is_element()), Some(s));
        assert_eq!(g.parent().unwrap().attribute("id"), Some("4"));
        for (node, field) in [(g, "group_attributes"), (s, "primary_attributes")] {
            let attrs: BTreeMap<_, _> = node.attributes().map(|a| (a.name(), a.value())).collect();
            assert_eq!(serde_json::to_value(attrs).unwrap(), pair[field]);
        }
        assert_eq!(s.attribute("enabled"), Some("true"));
        assert_eq!(s.attribute("enableGlobal1"), Some("true"));
        assert!(IDS.contains(&s.attribute("skillId").unwrap()));
        for name in ["mainActiveSkill", "mainActiveSkillCalcs"] {
            assert!(matches!(g.attribute(name), None | Some("nil" | "1")));
        }
    }
}

#[test]
fn actual_constructor_and_consumers_keep_real_primary_prefix_with_later_unknown_commands() {
    for jit in [false, true] {
        let lua = components(jit);
        for id in IDS {
            let gem = construct(&lua, id);
            let effect: Table = gem.get("grantedEffect").unwrap();
            assert!(matches!(
                effect.get::<Value>("support").unwrap(),
                Value::Nil
            ));
            assert!(matches!(
                effect.get::<Value>("hideFromSideBar").unwrap(),
                Value::Nil
            ));
            let list: Table = gem.get("grantedEffectList").unwrap();
            assert_eq!(
                list.get::<Table>(1).unwrap().get::<String>("id").unwrap(),
                id
            );
            let first = occurrence(&lua, gem, "actual-primary");
            let g = group(&lua, std::slice::from_ref(&first));
            let active = admitted(&lua, &g);
            assert_eq!(effect_of(&active.get::<Table>(1).unwrap()), id);
            for mode in ["MAIN", "CALCS"] {
                let chosen = selected(&lua, &g, mode, &active).unwrap();
                assert_eq!(source_of(&chosen).to_pointer(), first.to_pointer());
                assert_eq!(effect_of(&chosen), id);
            }
            let exported: Table = lua
                .globals()
                .get::<Function>("exportSelected")
                .unwrap()
                .call(g)
                .unwrap();
            assert_eq!(exported.to_pointer(), first.to_pointer());
            if id.starts_with("SummonSkeletal") {
                let family = packet().evidence["families"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|family| family["primary"] == id)
                    .unwrap();
                let declared = &family["gem"]["declared_additional_effects"];
                assert_eq!(declared.as_array().unwrap().len(), 1);
                let missing = &declared[0]["id"];
                assert!(
                    !packet().catalog["skills"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|skill| skill["id"] == *missing)
                );
                // Missing source reference remains declared, not certified empty.
                assert_eq!(list.raw_len(), 1);
            }
        }
    }
}

#[test]
fn real_admission_gate_rejects_hidden_support_disabled_and_false_global_counterexamples() {
    for jit in [false, true] {
        let lua = components(jit);
        for mutation in ["hidden", "support", "disabled", "global"] {
            let gem = construct(&lua, "IceNovaPlayer");
            let effect: Table = gem.get("grantedEffect").unwrap();
            let old_hidden: Value = effect.get("hideFromSideBar").unwrap();
            let old_support: Value = effect.get("support").unwrap();
            let old_global: Value = effect.get("hasGlobalEffect").unwrap();
            let first = occurrence(&lua, gem, mutation);
            match mutation {
                "hidden" => effect.set("hideFromSideBar", true).unwrap(),
                "support" => effect.set("support", true).unwrap(),
                "disabled" => first.set("enabled", false).unwrap(),
                "global" => {
                    effect.set("hasGlobalEffect", true).unwrap();
                    first.set("enableGlobal1", false).unwrap();
                }
                _ => unreachable!(),
            }
            let g = group(&lua, &[first]);
            assert_eq!(admitted(&lua, &g).raw_len(), 0, "{mutation}");
            let exported: Option<Table> = lua
                .globals()
                .get::<Function>("exportSelected")
                .unwrap()
                .call(g)
                .unwrap();
            assert!(exported.is_none(), "{mutation}");
            effect.set("hideFromSideBar", old_hidden).unwrap();
            effect.set("support", old_support).unwrap();
            effect.set("hasGlobalEffect", old_global).unwrap();
        }
        let gem = construct(&lua, "IceNovaPlayer");
        let effect: Table = gem.get("grantedEffect").unwrap();
        effect.set("hasGlobalEffect", true).unwrap();
        let first = occurrence(&lua, gem, "global-metadata-true");
        let g = group(&lua, &[first]);
        assert_eq!(admitted(&lua, &g).raw_len(), 1);
        // This proves true global1 admits either metadata state; it does not
        // prove global1 inert or initialize the actual mutable metadata.
    }
}

#[test]
fn actual_constructor_reordering_and_duplicate_source_controls_preserve_exact_identity() {
    for jit in [false, true] {
        let lua = components(jit);
        let gem = construct(&lua, "SummonSandDjinnPlayer");
        let list: Table = gem.get("grantedEffectList").unwrap();
        assert_eq!(list.raw_len(), 2);
        gem.set(
            "grantedEffectDisplayOrder",
            lua.create_sequence_from([1, 0]).unwrap(),
        )
        .unwrap();
        let key: String = gem.get("id").unwrap();
        lua.globals()
            .get::<Function>("constructGem")
            .unwrap()
            .call::<()>((gem.clone(), key))
            .unwrap();
        let changed = occurrence(&lua, gem, "synthetic-display-reorder");
        let g = group(&lua, &[changed]);
        assert_eq!(
            effect_of(&admitted(&lua, &g).get::<Table>(1).unwrap()),
            "CommandSandDjinnKnifeThrowPlayer"
        );
        let gem = construct(&lua, "IceNovaPlayer");
        let a = occurrence(&lua, gem.clone(), "copy-a");
        let b = occurrence(&lua, gem, "copy-b");
        for rows in [[a.clone(), b.clone()], [b.clone(), a.clone()]] {
            let g = group(&lua, &rows);
            let active = admitted(&lua, &g);
            assert_eq!(active.raw_len(), 2);
            assert_ne!(rows[0].to_pointer(), rows[1].to_pointer());
            assert_eq!(
                source_of(&selected(&lua, &g, "MAIN", &active).unwrap()).to_pointer(),
                rows[0].to_pointer()
            );
        }
    }
}

#[test]
fn actual_loader_modes_export_and_out_of_domain_clamps_remain_distinct_observations() {
    for jit in [false, true] {
        let lua = components(jit);
        let gem = construct(&lua, "IceNovaPlayer");
        let a = occurrence(&lua, gem.clone(), "copy-a");
        let b = occurrence(&lua, gem, "copy-b");
        let g = group(&lua, &[a.clone(), b.clone()]);
        let active = admitted(&lua, &g);
        for token in [None, Some("nil"), Some("1")] {
            let attrs = lua.create_table().unwrap();
            if let Some(token) = token {
                attrs.set("mainActiveSkill", token).unwrap();
                attrs.set("mainActiveSkillCalcs", token).unwrap();
            }
            let loaded: Table = lua
                .globals()
                .get::<Function>("loaded")
                .unwrap()
                .call(attrs)
                .unwrap();
            assert_eq!(loaded.get::<i64>("mainActiveSkill").unwrap(), 1);
            assert_eq!(loaded.get::<i64>("mainActiveSkillCalcs").unwrap(), 1);
        }
        g.set("mainActiveSkill", 1).unwrap();
        g.set("mainActiveSkillCalcs", 2).unwrap();
        assert_eq!(
            source_of(&selected(&lua, &g, "MAIN", &active).unwrap()).to_pointer(),
            a.to_pointer()
        );
        assert_eq!(
            source_of(&selected(&lua, &g, "CALCS", &active).unwrap()).to_pointer(),
            b.to_pointer()
        );
        g.set("displaySkillList", active.clone()).unwrap();
        let exported: Table = lua
            .globals()
            .get::<Function>("exportSelected")
            .unwrap()
            .call(g.clone())
            .unwrap();
        assert_eq!(exported.to_pointer(), a.to_pointer());
        for value in [0.0, -1.0, 1.5] {
            g.set("mainActiveSkill", value).unwrap();
            assert!(selected(&lua, &g, "MAIN", &active).is_none());
        }
        g.set("mainActiveSkill", 99).unwrap();
        assert_eq!(
            source_of(&selected(&lua, &g, "MAIN", &active).unwrap()).to_pointer(),
            b.to_pointer()
        );
        assert_eq!(g.get::<i64>("mainActiveSkill").unwrap(), 2);
        let none: Option<Table> = lua
            .globals()
            .get::<Function>("selected")
            .unwrap()
            .call((g.clone(), "MAIN", active, 1, 2))
            .unwrap();
        assert!(
            none.is_none(),
            "unselected group cannot choose the main skill"
        );
        let empty = group(&lua, &[]);
        assert!(selected(&lua, &empty, "MAIN", &admitted(&lua, &empty)).is_none());
        // The omitted later source fallback creates MeleeUnarmedPlayer. The
        // component deliberately does not claim that as an imported target.
        assert!(packet().boundary("fallback").contains("MeleeUnarmedPlayer"));
        // Source fallback for malformed strings is observable but excluded from
        // the candidate domain. This is not a new Import decoding policy.
        let attrs = lua.create_table().unwrap();
        attrs.set("mainActiveSkill", "malformed").unwrap();
        let loaded: Table = lua
            .globals()
            .get::<Function>("loaded")
            .unwrap()
            .call(attrs)
            .unwrap();
        assert_eq!(loaded.get::<i64>("mainActiveSkill").unwrap(), 1);
    }
}

fn exported(lua: &Lua, group: &Table, calculated: Option<&Table>) -> Option<Table> {
    // Publish only this stage's identity-only result. This models the two
    // export paths, not arbitrary stale UI caches or complete CalcSetup.
    group.set("displaySkillList", calculated.cloned()).unwrap();
    lua.globals()
        .get::<Function>("exportSelected")
        .unwrap()
        .call(group.clone())
        .unwrap()
}

fn synthetic_support(lua: &Lua, name: &str) -> Table {
    // Explicit synthetic companion: the actual admission predicate must skip
    // its only effect. This supplies no real support construction authority.
    let effect = lua.create_table().unwrap();
    effect.set("id", name).unwrap();
    effect.set("support", true).unwrap();
    let gem = lua.create_table().unwrap();
    gem.set("grantedEffect", effect.clone()).unwrap();
    gem.set(
        "grantedEffectList",
        lua.create_sequence_from([effect]).unwrap(),
    )
    .unwrap();
    occurrence(lua, gem, name)
}

#[test]
fn zero_or_one_candidate_selectors_are_equivalent_across_matched_filter_transitions() {
    for jit in [false, true] {
        let lua = components(jit);
        for filter in ["hidden", "disabled", "global", "support"] {
            for main in [None, Some("nil"), Some("1")] {
                for calcs in [None, Some("nil"), Some("1")] {
                    let gem = construct(&lua, "IceNovaPlayer");
                    let effect: Table = gem.get("grantedEffect").unwrap();
                    let old_hidden: Value = effect.get("hideFromSideBar").unwrap();
                    let old_support: Value = effect.get("support").unwrap();
                    let old_global: Value = effect.get("hasGlobalEffect").unwrap();
                    let row = occurrence(&lua, gem, "same-physical-candidate");
                    let g = group(
                        &lua,
                        &[
                            synthetic_support(&lua, "synthetic-before"),
                            row.clone(),
                            synthetic_support(&lua, "synthetic-after"),
                        ],
                    );
                    let attrs = lua.create_table().unwrap();
                    attrs.set("mainActiveSkill", main).unwrap();
                    attrs.set("mainActiveSkillCalcs", calcs).unwrap();
                    let loaded: Table = lua
                        .globals()
                        .get::<Function>("loaded")
                        .unwrap()
                        .call(attrs)
                        .unwrap();
                    for field in ["mainActiveSkill", "mainActiveSkillCalcs"] {
                        let value = loaded.get::<i64>(field).unwrap();
                        assert_eq!(value, 1);
                        g.set(field, value).unwrap();
                    }

                    // Keep this same group and decoded selectors through all
                    // stages. In particular, an empty stage must not clamp a
                    // selector to zero and poison the later singleton stage.
                    for closed in [true, false, true] {
                        match filter {
                            "hidden" => effect.set("hideFromSideBar", closed).unwrap(),
                            "disabled" => row.set("enabled", !closed).unwrap(),
                            "global" => {
                                effect.set("hasGlobalEffect", true).unwrap();
                                row.set("enableGlobal1", !closed).unwrap();
                            }
                            "support" => effect.set("support", closed).unwrap(),
                            _ => unreachable!(),
                        }
                        let active = admitted(&lua, &g);
                        assert_eq!(
                            active.raw_len(),
                            usize::from(!closed),
                            "{jit}/{filter}/{main:?}/{calcs:?}"
                        );
                        let expected = (!closed).then_some(row.to_pointer());
                        for mode in ["MAIN", "CALCS"] {
                            let chosen = selected(&lua, &g, mode, &active);
                            assert_eq!(
                                chosen.as_ref().map(|value| source_of(value).to_pointer()),
                                expected,
                                "{jit}/{filter}/{mode}/{main:?}/{calcs:?}"
                            );
                            if let Some(chosen) = chosen {
                                assert_eq!(effect_of(&chosen), "IceNovaPlayer");
                            }
                            let unselected: Option<Table> = lua
                                .globals()
                                .get::<Function>("selected")
                                .unwrap()
                                .call((g.clone(), mode, active.clone(), 1, 2))
                                .unwrap();
                            assert!(unselected.is_none());
                        }
                        for calculated in [None, Some(&active)] {
                            assert_eq!(
                                exported(&lua, &g, calculated).map(|value| value.to_pointer()),
                                expected,
                                "matched fresh/calculated exporter paths"
                            );
                        }
                        for field in ["mainActiveSkill", "mainActiveSkillCalcs"] {
                            assert_eq!(g.get::<i64>(field).unwrap(), 1);
                        }
                    }
                    effect.set("hideFromSideBar", old_hidden).unwrap();
                    effect.set("support", old_support).unwrap();
                    effect.set("hasGlobalEffect", old_global).unwrap();
                }
            }
        }

        // Equal decoded indices alone cannot reveal a mistaken MAIN/CALCS
        // read. Zero is a negative sentinel outside the proposed receipt domain:
        // only its own context loses the singleton; export reads MAIN only.
        let row = occurrence(&lua, construct(&lua, "IceNovaPlayer"), "mode-control");
        let g = group(&lua, std::slice::from_ref(&row));
        let active = admitted(&lua, &g);
        assert_eq!(active.raw_len(), 1);
        for (main, calcs) in [(1, 0), (0, 1)] {
            g.set("mainActiveSkill", main).unwrap();
            g.set("mainActiveSkillCalcs", calcs).unwrap();
            for (mode, index) in [("MAIN", main), ("CALCS", calcs)] {
                assert_eq!(
                    selected(&lua, &g, mode, &active).map(|value| source_of(&value).to_pointer()),
                    (index == 1).then_some(row.to_pointer())
                );
            }
            for calculated in [None, Some(&active)] {
                assert_eq!(
                    exported(&lua, &g, calculated).map(|value| value.to_pointer()),
                    (main == 1).then_some(row.to_pointer())
                );
            }
            assert_eq!(g.get::<i64>("mainActiveSkill").unwrap(), main);
            assert_eq!(g.get::<i64>("mainActiveSkillCalcs").unwrap(), calcs);
        }
        // Empty selection never asserts an imported fallback, positive native
        // participation, effect readiness, numerical results or whole-build parity.
    }
}

#[path = "support/manual_direct_selector.rs"]
mod manual_direct_selector;
