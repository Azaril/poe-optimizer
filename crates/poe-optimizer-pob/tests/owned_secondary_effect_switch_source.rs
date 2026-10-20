//! Authenticated source COMPONENT proof only. No Import completion authority,
//! native model, complete active-skill preparation, or numerical parity.
#![cfg(not(target_arch = "wasm32"))]

use flate2::read::GzDecoder;
use mlua::{Function, Lua, Table, Value};
use poe_optimizer_pob::source;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf, sync::OnceLock};

const DIRECTORY: &str = "data/owned/poe2/3887ae68/secondary-effect-switch-source";
const AUTHORING_SHA: &str = "7dd2c02757f37c723c8fd4eaebd276e54f666c9c065b238b76a614f3f1c446c9";
const SINGLETONS: [&str; 2] = ["IceNovaPlayer", "FireboltPlayer"];
const MISSING: &str = "SummonSkeletalSnipersPlayer";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn text(value: &Json) -> &str {
    value.as_str().unwrap()
}
fn pinned(pin: &Json) -> Vec<u8> {
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
    fn family(&self, primary: &str) -> &Json {
        self.evidence["families"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["primary"] == primary)
            .unwrap()
    }
}
fn packet() -> &'static Packet {
    static PACKET: OnceLock<Packet> = OnceLock::new();
    PACKET.get_or_init(|| {
        let dir = root().join(DIRECTORY);
        let bytes = fs::read(dir.join("authoring.json")).unwrap();
        assert_eq!(bytes.len(), 568);
        assert_eq!(hash(&bytes), AUTHORING_SHA);
        let authoring: Json = serde_json::from_slice(&bytes).unwrap();
        for (name, pin) in authoring["artifacts"].as_object().unwrap() {
            let bytes = fs::read(dir.join(name)).unwrap();
            assert_eq!(json!(bytes.len()), pin["bytes"]);
            assert_eq!(hash(&bytes), text(&pin["sha256"]));
        }
        let evidence: Json =
            serde_json::from_slice(&fs::read(dir.join("evidence.json")).unwrap()).unwrap();
        assert_eq!(
            source::UPSTREAM_REVISION,
            text(&evidence["source_revision"])
        );
        assert_eq!(
            source::manifest_sha256(),
            text(&evidence["source_manifest"]["sha256"])
        );
        let manifest: Json = serde_json::from_slice(&pinned(&evidence["source_manifest"])).unwrap();
        let catalog = serde_json::from_slice(&pinned(&evidence["catalog"])).unwrap();
        pinned(&evidence["license"]);
        pinned(&evidence["original"]);
        let mut sources = BTreeMap::new();
        assert_eq!(evidence["source_files"].as_array().unwrap().len(), 9);
        for pin in evidence["source_files"].as_array().unwrap() {
            assert!(manifest["files"].as_array().unwrap().contains(pin));
            let name = text(&pin["path"]);
            let body =
                source::read_verified_text(&root().join("vendor/path-of-building-poe2"), name)
                    .unwrap();
            assert_eq!(json!(body.len()), pin["bytes"]);
            assert_eq!(hash(body.as_bytes()), text(&pin["sha256"]));
            assert!(sources.insert(name.into(), body).is_none());
        }
        let retained = &evidence["retained_export"];
        let compressed = fs::read(root().join(text(&retained["file"]))).unwrap();
        assert_eq!(json!(compressed.len()), retained["compressed_bytes"]);
        assert_eq!(hash(&compressed), text(&retained["compressed_sha256"]));
        let mut raw = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(1024 * 1024)
            .read_to_end(&mut raw)
            .unwrap();
        assert_eq!(json!(raw.len()), retained["raw_bytes"]);
        assert_eq!(hash(&raw), text(&retained["raw_sha256"]));
        assert_eq!(raw, sources["src/Modules/BuildExportPoE2.lua"].as_bytes());
        let p = Packet {
            evidence,
            catalog,
            sources,
        };
        assert_eq!(p.evidence["boundaries"].as_object().unwrap().len(), 13);
        for span in p.evidence["boundaries"].as_object().unwrap().values() {
            p.span(span);
        }
        p
    })
}

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
    lua.load(format!(
        "bit=require('bit')\n{}\n{}\n",
        p.boundary("copy_table"),
        p.boundary("array_search")
    ))
    .set_name("@authenticated-common-functions")
    .exec()
    .unwrap();
    lua.load(&p.sources["src/Data/Global.lua"])
        .set_name("@authenticated-Global")
        .exec()
        .unwrap();
    let mut definitions = p.boundary("mod_helpers");
    definitions.push_str(
        "\nlocal mod,flag,skill=makeSkillMod,makeFlagMod,makeSkillDataMod\nlocal skills={}\n",
    );
    for row in p.evidence["families"].as_array().unwrap() {
        definitions.push_str(&p.declaration(&row["skill_declaration"]));
    }
    // Only source identity initialization. No lazy maps or full preparation.
    definitions.push_str(
        "\nfor id,effect in pairs(skills) do effect.id=id;effect.modSource='Skill:'..id end\nreturn skills",
    );
    let skills: Table = lua.load(definitions).eval().unwrap();
    let gems: Table = lua.load(&p.sources["src/Data/Gems.lua"]).eval().unwrap();
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
        "local t_insert=table.insert\n{}\n{}\nconstructGem=setupGem\nprocessSourceMod=processMod",
        p.boundary("constructor"),
        p.boundary("process_mod"),
    ))
    .set_name("@authenticated-constructor-and-metadata-writer")
    .exec()
    .unwrap();
    lua.load(format!(
        "calcs={{}}\n{}\n\
         function admitted(group)\nlocal out={{}}\n{}\n\
         out[#out+1]={{activeEffect={{srcInstance=gemInstance,grantedEffect=grantedEffect}}}}\n\
         end end end end\nreturn out end\n\
         local t_insert=table.insert\n{}\nexportActive=activeSkillGems\n\
         function loadSwitches(attributes)\nlocal child={{attrib=attributes}}\nlocal gemInstance={{}}\n\
         {}\nreturn gemInstance end\n\
         function visibility(group)\nlocal self={{displayGroup=group}}\nlocal index=1\n\
         local slot={{count={{}},corruptLevel={{}},enableGlobal2={{}}}}\n{}\n{}\n{}\n\
         return slot.count.shown(),slot.corruptLevel.shown(),slot.enableGlobal2.shown() end",
        p.boundary("count_consumer"),
        p.boundary("admission_prefix"),
        p.boundary("export_active"),
        p.boundary("load_switches"),
        p.boundary("count_visibility"),
        p.boundary("corruption_visibility"),
        p.boundary("secondary_visibility"),
    ))
    .set_name("@authenticated-consumers-with-explicit-identity-host")
    .exec()
    .unwrap();
    // Synthetic observational instrumentation. A read has the same returned
    // value as a plain field, while exposing whether the source consulted it.
    lua.load(
        r#"
        function secondaryProbe(row, value)
            local probe = { reads = 0, value = value }
            setmetatable(row, { __index = function(_, key)
                if key == "enableGlobal2" then
                    probe.reads = probe.reads + 1
                    return probe.value
                end
            end })
            return probe
        end
        "#,
    )
    .exec()
    .unwrap();
    lua
}

fn raw_gem(lua: &Lua, primary: &str) -> Table {
    let key = text(&packet().family(primary)["gem"]["key"]);
    lua.globals()
        .get::<Table>("allGems")
        .unwrap()
        .get(key)
        .unwrap()
}
fn copy(lua: &Lua, table: &Table) -> Table {
    lua.globals()
        .get::<Function>("copyTable")
        .unwrap()
        .call(table.clone())
        .unwrap()
}
fn setup(lua: &Lua, gem: &Table, primary: &str) {
    let key = text(&packet().family(primary)["gem"]["key"]);
    lua.globals()
        .get::<Function>("constructGem")
        .unwrap()
        .call::<()>((gem.clone(), key))
        .unwrap();
}
fn construct(lua: &Lua, primary: &str) -> Table {
    let gem = copy(lua, &raw_gem(lua, primary));
    setup(lua, &gem, primary);
    gem
}
fn effect(lua: &Lua, primary: &str) -> Table {
    lua.globals()
        .get::<Table>("data")
        .unwrap()
        .get::<Table>("skills")
        .unwrap()
        .get(primary)
        .unwrap()
}
fn set_global_metadata(lua: &Lua, effect: &Table, present: Option<bool>) {
    effect.set("hasGlobalEffect", present).unwrap();
    if present == Some(true) {
        effect.set("hasGlobalEffect", Value::Nil).unwrap();
        // Synthetic modifier; actual source processMod is the metadata writer.
        let modifier: Table = lua
            .load("return {name='SyntheticControl',type='BASE',value=1,{type='GlobalEffect',effectType='Buff'}}")
            .eval()
            .unwrap();
        lua.globals()
            .get::<Function>("processSourceMod")
            .unwrap()
            .call::<()>((effect.clone(), modifier))
            .unwrap();
        assert!(effect.get::<bool>("hasGlobalEffect").unwrap());
    }
}
fn occurrence(lua: &Lua, gem: &Table, secondary: Option<bool>) -> (Table, Table) {
    let row = lua.create_table().unwrap();
    row.set("gemData", gem.clone()).unwrap();
    row.set("enabled", true).unwrap();
    row.set("enableGlobal1", true).unwrap();
    row.set("count", 3).unwrap();
    let probe = lua
        .globals()
        .get::<Function>("secondaryProbe")
        .unwrap()
        .call((row.clone(), secondary))
        .unwrap();
    (row, probe)
}
fn group(lua: &Lua, row: &Table) -> Table {
    let group = lua.create_table().unwrap();
    group
        .set("gemList", lua.create_sequence_from([row.clone()]).unwrap())
        .unwrap();
    group
}
fn active_ids(lua: &Lua, group: &Table, row: &Table) -> Vec<String> {
    let active: Table = lua
        .globals()
        .get::<Function>("admitted")
        .unwrap()
        .call(group.clone())
        .unwrap();
    active
        .sequence_values::<Table>()
        .map(|active| {
            let active: Table = active.unwrap().get("activeEffect").unwrap();
            let source: Table = active.get("srcInstance").unwrap();
            assert_eq!(source.to_pointer(), row.to_pointer());
            active
                .get::<Table>("grantedEffect")
                .unwrap()
                .get("id")
                .unwrap()
        })
        .collect()
}
fn export_count(lua: &Lua, group: &Table, row: &Table) -> usize {
    let exported: Table = lua
        .globals()
        .get::<Function>("exportActive")
        .unwrap()
        .call(group.clone())
        .unwrap();
    let mut count = 0;
    for instance in exported.sequence_values::<Table>() {
        assert_eq!(instance.unwrap().to_pointer(), row.to_pointer());
        count += 1;
    }
    count
}
fn count_for(lua: &Lua, group: &Table, effect: &Table) -> (i64, bool) {
    let active = lua.create_table().unwrap();
    let active_effect = lua.create_table().unwrap();
    active_effect.set("grantedEffect", effect.clone()).unwrap();
    active.set("activeEffect", active_effect).unwrap();
    active.set("socketGroup", group.clone()).unwrap();
    lua.globals()
        .get::<Table>("calcs")
        .unwrap()
        .get::<Function>("getActiveSkillCount")
        .unwrap()
        .call(active)
        .unwrap()
}
fn visibility(lua: &Lua, group: &Table) -> (bool, bool, Option<bool>) {
    lua.globals()
        .get::<Function>("visibility")
        .unwrap()
        .call(group.clone())
        .unwrap()
}
fn no_additional_ids(table: &Table) -> bool {
    table.pairs::<Value, Value>().all(|entry| {
        let (key, _) = entry.unwrap();
        !matches!(key, Value::String(key) if key.to_str().unwrap().starts_with("additionalGrantedEffectId"))
    })
}
fn empty_table(table: &Table) -> bool {
    table.pairs::<Value, Value>().next().is_none()
}
/// Conservative TEST-ONLY admission predicate. It is not a production Import
/// matcher, and cannot manufacture source facts from a raw_len observation.
fn singleton_candidate(declared: &Table, constructed: &Table, primary: &Table) -> bool {
    if !no_additional_ids(declared) || !no_additional_ids(constructed) {
        return false;
    }
    let (Ok(list), Ok(additional), Ok(actual_primary)) = (
        constructed.get::<Table>("grantedEffectList"),
        constructed.get::<Table>("additionalGrantedEffects"),
        constructed.get::<Table>("grantedEffect"),
    ) else {
        return false;
    };
    if !empty_table(&additional) || actual_primary.to_pointer() != primary.to_pointer() {
        return false;
    }
    let entries = list
        .pairs::<Value, Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    if entries.len() != 1 {
        return false;
    }
    let first_index = match &entries[0].0 {
        Value::Integer(value) => *value == 1,
        Value::Number(value) => *value == 1.0,
        _ => false,
    };
    first_index
        && matches!(&entries[0].1, Value::Table(value) if value.to_pointer() == primary.to_pointer())
}

#[test]
fn packet_authenticates_real_singletons_and_retains_missing_reference_counterexample() {
    let p = packet();
    for field in [
        "full_build_parity",
        "numerical_parity",
        "import_accounting_implemented",
        "native_runtime_authority",
        "complete_skill_preparation",
        "synthetic_controls_are_real_game_states",
    ] {
        assert_eq!(p.evidence["scope"][field], false);
    }
    assert_eq!(p.evidence["families"].as_array().unwrap().len(), 3);
    for id in [MISSING, SINGLETONS[0], SINGLETONS[1]] {
        let family = p.family(id);
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
                    .filter(|row| **row == family[field])
                    .count(),
                1
            );
        }
        for (field, declaration) in [("gem", "gem_declaration"), ("skill", "skill_declaration")] {
            assert_eq!(
                family[field]["winning_declaration"],
                family[declaration]["index"]
            );
            p.declaration(&family[declaration]);
        }
        assert_eq!(family["gem"]["effect_list"], json!([id]));
        if SINGLETONS.contains(&id) {
            for name in [
                "declared_additional_effects",
                "constructed_additional_effects",
                "additional_effects",
            ] {
                assert_eq!(family["gem"][name], json!([]));
            }
        } else {
            let missing = &family["gem"]["declared_additional_effects"][0]["id"];
            assert!(missing.is_string());
            assert!(
                !p.catalog["skills"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|row| row["id"] == *missing)
            );
        }
    }
    let original = pinned(&p.evidence["original"]);
    let doc = roxmltree::Document::parse(std::str::from_utf8(&original).unwrap()).unwrap();
    let nodes = doc
        .descendants()
        .filter(|n| n.is_element())
        .collect::<Vec<_>>();
    let pairs = p.evidence["original_pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 2);
    for pair in pairs {
        let row = nodes[pair["primary_ordinal"].as_u64().unwrap() as usize];
        let group = nodes[pair["group_ordinal"].as_u64().unwrap() as usize];
        assert_eq!(row.parent(), Some(group));
        for (node, field) in [(row, "primary_attributes"), (group, "group_attributes")] {
            let attributes = node
                .attributes()
                .map(|a| (a.name(), a.value()))
                .collect::<BTreeMap<_, _>>();
            assert_eq!(serde_json::to_value(attributes).unwrap(), pair[field]);
        }
    }
}

#[test]
fn singleton_secondary_is_unread_with_multiple_stat_sets_vaal_and_global_metadata() {
    for jit in [false, true] {
        let lua = components(jit);
        for id in SINGLETONS {
            for vaal in [false, true] {
                for global in [None, Some(false), Some(true)] {
                    for secondary in [None, Some(false), Some(true)] {
                        let gem = construct(&lua, id);
                        gem.set("vaalGem", vaal).unwrap(); // Synthetic branch control.
                        let primary = effect(&lua, id);
                        assert!(primary.get::<Table>("statSets").unwrap().raw_len() > 1);
                        set_global_metadata(&lua, &primary, global);
                        assert!(singleton_candidate(&raw_gem(&lua, id), &gem, &primary));
                        let (row, probe) = occurrence(&lua, &gem, secondary);
                        let group = group(&lua, &row);
                        assert_eq!(active_ids(&lua, &group, &row), [id]);
                        assert_eq!(export_count(&lua, &group, &row), 1);
                        assert_eq!(count_for(&lua, &group, &primary), (3, true));
                        let shown = visibility(&lua, &group);
                        let from_provider = primary.get::<Option<bool>>("fromItem").unwrap()
                            == Some(true)
                            || primary.get::<Option<bool>>("fromTree").unwrap() == Some(true);
                        assert!(shown.0);
                        assert_eq!(shown.1, !from_provider);
                        assert_ne!(shown.2, Some(true));
                        assert_eq!(probe.get::<u32>("reads").unwrap(), 0);
                    }
                }
            }
        }
    }
}

#[test]
fn primary_gate_and_source_loader_remain_separate_from_secondary_non_consumption() {
    for jit in [false, true] {
        let lua = components(jit);
        for token in [None, Some("nil"), Some("false"), Some("true")] {
            let attributes = lua.create_table().unwrap();
            if let Some(token) = token {
                attributes.set("enableGlobal2", token).unwrap();
            }
            let loaded: Table = lua
                .globals()
                .get::<Function>("loadSwitches")
                .unwrap()
                .call(attributes)
                .unwrap();
            assert_eq!(
                loaded.get::<bool>("enableGlobal2").unwrap(),
                token == Some("true")
            );
            assert!(loaded.get::<bool>("enableGlobal1").unwrap());
        }
        for vaal in [false, true] {
            let gem = construct(&lua, "IceNovaPlayer");
            gem.set("vaalGem", vaal).unwrap();
            let primary = effect(&lua, "IceNovaPlayer");
            set_global_metadata(&lua, &primary, Some(true));
            let (row, probe) = occurrence(&lua, &gem, Some(true));
            row.set("enableGlobal1", false).unwrap();
            let group = group(&lua, &row);
            assert!(active_ids(&lua, &group, &row).is_empty());
            assert_eq!(export_count(&lua, &group, &row), 0);
            // Direct component query, not a claim this disabled effect executes.
            assert_eq!(count_for(&lua, &group, &primary), (3, !vaal));
            assert_eq!(probe.get::<u32>("reads").unwrap(), 0);
        }
    }
}

#[test]
fn a_real_unresolved_declaration_or_synthetic_gap_cannot_be_certified_from_length_one() {
    for jit in [false, true] {
        let lua = components(jit);
        let gem = construct(&lua, MISSING);
        assert_eq!(gem.get::<Table>("grantedEffectList").unwrap().raw_len(), 1);
        assert!(!singleton_candidate(
            &raw_gem(&lua, MISSING),
            &gem,
            &effect(&lua, MISSING)
        ));
        for field in ["additionalGrantedEffectId1", "additionalGrantedEffectId2"] {
            let declared = copy(&lua, &raw_gem(&lua, "IceNovaPlayer"));
            declared.set(field, "MissingSyntheticEffect").unwrap();
            let gem = copy(&lua, &declared);
            setup(&lua, &gem, "IceNovaPlayer");
            assert_eq!(gem.get::<Table>("grantedEffectList").unwrap().raw_len(), 1);
            assert!(!singleton_candidate(
                &declared,
                &gem,
                &effect(&lua, "IceNovaPlayer")
            ));
        }
    }
}

#[test]
fn two_effects_and_actual_display_reordering_expose_secondary_reads_and_exact_effect_identity() {
    for jit in [false, true] {
        let lua = components(jit);
        for reordered in [false, true] {
            for vaal in [false, true] {
                for secondary in [false, true] {
                    let declared = copy(&lua, &raw_gem(&lua, "IceNovaPlayer"));
                    declared
                        .set("additionalGrantedEffectId1", "FireboltPlayer")
                        .unwrap();
                    if reordered {
                        declared
                            .set(
                                "grantedEffectDisplayOrder",
                                lua.create_sequence_from([1, 0]).unwrap(),
                            )
                            .unwrap();
                    }
                    let gem = copy(&lua, &declared);
                    setup(&lua, &gem, "IceNovaPlayer");
                    gem.set("vaalGem", vaal).unwrap();
                    let primary = effect(&lua, "IceNovaPlayer");
                    let other = effect(&lua, "FireboltPlayer");
                    for effect in [&primary, &other] {
                        set_global_metadata(&lua, effect, Some(true));
                    }
                    assert!(!singleton_candidate(&declared, &gem, &primary));
                    let (row, probe) = occurrence(&lua, &gem, Some(secondary));
                    let group = group(&lua, &row);
                    let first = if reordered {
                        "FireboltPlayer"
                    } else {
                        "IceNovaPlayer"
                    };
                    let second = if reordered {
                        "IceNovaPlayer"
                    } else {
                        "FireboltPlayer"
                    };
                    let mut expected = vec![first.to_owned()];
                    if secondary {
                        expected.push(second.to_owned());
                    }
                    assert_eq!(active_ids(&lua, &group, &row), expected);
                    assert_eq!(export_count(&lua, &group, &row), 1 + usize::from(secondary));
                    let before = probe.get::<u32>("reads").unwrap();
                    assert!(before > 0, "positive admission/export access control");
                    assert_eq!(
                        count_for(&lua, &group, &effect(&lua, second)),
                        (3, !vaal || secondary)
                    );
                    assert_eq!(probe.get::<u32>("reads").unwrap(), before + u32::from(vaal));
                    // Force the visibility loop past index1 to observe its index2 gate.
                    row.set("enableGlobal1", false).unwrap();
                    let shown = visibility(&lua, &group);
                    // The provider guard reads gem.grantedEffect (Ice Nova),
                    // before its loop; display order does not change that owner.
                    assert_eq!((shown.0, shown.1), (secondary, secondary));
                    assert_eq!(shown.2, Some(vaal));
                    assert!(probe.get::<u32>("reads").unwrap() > before);
                }
            }
        }
    }
}

#[test]
fn late_resolved_sparse_and_constructor_generated_additions_reject_singleton_authority() {
    for jit in [false, true] {
        let lua = components(jit);
        let raw = raw_gem(&lua, "IceNovaPlayer");
        let primary = effect(&lua, "IceNovaPlayer");
        let secondary = effect(&lua, "FireboltPlayer");
        for index in [2, 3] {
            let gem = construct(&lua, "IceNovaPlayer");
            gem.get::<Table>("grantedEffectList")
                .unwrap()
                .set(index, secondary.clone())
                .unwrap();
            assert!(
                !singleton_candidate(&raw, &gem, &primary),
                "including a sparse late addition"
            );
            if index == 2 {
                set_global_metadata(&lua, &secondary, Some(true));
                let (row, probe) = occurrence(&lua, &gem, Some(false));
                let group = group(&lua, &row);
                assert_eq!(active_ids(&lua, &group, &row), ["IceNovaPlayer"]);
                probe.set("value", true).unwrap();
                assert_eq!(
                    active_ids(&lua, &group, &row),
                    ["IceNovaPlayer", "FireboltPlayer"]
                );
                assert!(probe.get::<u32>("reads").unwrap() > 0);
            }
        }
        let duplicate = copy(&lua, &raw);
        duplicate
            .set("additionalGrantedEffectId1", "IceNovaPlayer")
            .unwrap();
        let gem = copy(&lua, &duplicate);
        setup(&lua, &gem, "IceNovaPlayer");
        assert_eq!(gem.get::<Table>("grantedEffectList").unwrap().raw_len(), 2);
        assert!(!singleton_candidate(&duplicate, &gem, &primary));
        let declared = copy(&lua, &raw);
        declared.set("name", "Barbs I").unwrap(); // Synthetic input to the actual constructor factory.
        lua.globals()
            .get::<Table>("data")
            .unwrap()
            .get::<Table>("skills")
            .unwrap()
            .set("TriggeredBarbsPlayer", secondary.clone())
            .unwrap();
        let gem = copy(&lua, &declared);
        setup(&lua, &gem, "IceNovaPlayer");
        assert_eq!(
            gem.get::<String>("additionalGrantedEffectId1").unwrap(),
            "TriggeredBarbsPlayer"
        );
        assert_eq!(gem.get::<Table>("grantedEffectList").unwrap().raw_len(), 2);
        assert!(!singleton_candidate(&declared, &gem, &primary));
        let gem = construct(&lua, "IceNovaPlayer");
        gem.get::<Table>("additionalGrantedEffects")
            .unwrap()
            .set(1, secondary)
            .unwrap();
        assert!(
            !singleton_candidate(&raw, &gem, &primary),
            "inconsistent parallel source list"
        );
    }
}

#[test]
fn exact_true_primary_is_non_restricting_but_other_admission_gates_and_counts_remain_distinct() {
    for jit in [false, true] {
        let lua = components(jit);
        for vaal in [false, true] {
            for metadata in [None, Some(false), Some(true)] {
                for count in [0, 7] {
                    let gem = construct(&lua, "IceNovaPlayer");
                    gem.set("vaalGem", vaal).unwrap();
                    let primary = effect(&lua, "IceNovaPlayer");
                    set_global_metadata(&lua, &primary, metadata);
                    let (row, probe) = occurrence(&lua, &gem, Some(false));
                    assert!(row.get::<bool>("enableGlobal1").unwrap());
                    row.set("count", count).unwrap();
                    let group = group(&lua, &row);
                    assert_eq!(active_ids(&lua, &group, &row), ["IceNovaPlayer"]);
                    assert_eq!(export_count(&lua, &group, &row), 1);
                    assert_eq!(count_for(&lua, &group, &primary), (count, true));
                    assert_eq!(probe.get::<u32>("reads").unwrap(), 0);
                }
            }
            for gate in ["enabled", "support", "hidden"] {
                let gem = construct(&lua, "IceNovaPlayer");
                gem.set("vaalGem", vaal).unwrap();
                let primary = effect(&lua, "IceNovaPlayer");
                set_global_metadata(&lua, &primary, Some(true));
                let (row, probe) = occurrence(&lua, &gem, Some(true));
                let old_support: Value = primary.get("support").unwrap();
                let old_hidden: Value = primary.get("hideFromSideBar").unwrap();
                match gate {
                    "enabled" => row.set("enabled", false).unwrap(),
                    "support" => primary.set("support", true).unwrap(),
                    "hidden" => primary.set("hideFromSideBar", true).unwrap(),
                    _ => unreachable!(),
                }
                let group = group(&lua, &row);
                assert!(active_ids(&lua, &group, &row).is_empty());
                assert_eq!(export_count(&lua, &group, &row), 0);
                // A direct count call is not an admission check. The non-Vaal
                // support case reaches the documented default, not count3.
                assert_eq!(
                    count_for(&lua, &group, &primary),
                    (if gate == "support" && !vaal { 1 } else { 3 }, true)
                );
                assert_eq!(probe.get::<u32>("reads").unwrap(), 0);
                primary.set("support", old_support).unwrap();
                primary.set("hideFromSideBar", old_hidden).unwrap();
            }
        }
    }
}

#[test]
fn exact_true_secondary_is_non_restricting_without_enabling_other_gates_or_changing_count() {
    for jit in [false, true] {
        let lua = components(jit);
        for reordered in [false, true] {
            for vaal in [false, true] {
                for metadata in [None, Some(false), Some(true)] {
                    for count in [0, 7] {
                        let declared = copy(&lua, &raw_gem(&lua, "IceNovaPlayer"));
                        declared
                            .set("additionalGrantedEffectId1", "FireboltPlayer")
                            .unwrap();
                        if reordered {
                            declared
                                .set(
                                    "grantedEffectDisplayOrder",
                                    lua.create_sequence_from([1, 0]).unwrap(),
                                )
                                .unwrap();
                        }
                        let gem = copy(&lua, &declared);
                        setup(&lua, &gem, "IceNovaPlayer");
                        gem.set("vaalGem", vaal).unwrap();
                        let (first_id, second_id) = if reordered {
                            ("FireboltPlayer", "IceNovaPlayer")
                        } else {
                            ("IceNovaPlayer", "FireboltPlayer")
                        };
                        let first = effect(&lua, first_id);
                        let second = effect(&lua, second_id);
                        set_global_metadata(&lua, &first, Some(true));
                        set_global_metadata(&lua, &second, metadata);
                        let (row, probe) = occurrence(&lua, &gem, Some(true));
                        row.set("count", count).unwrap();
                        let group = group(&lua, &row);
                        assert_eq!(active_ids(&lua, &group, &row), [first_id, second_id]);
                        assert_eq!(export_count(&lua, &group, &row), 2);
                        assert_eq!(count_for(&lua, &group, &second), (count, true));
                        if metadata == Some(true) || vaal {
                            assert!(probe.get::<u32>("reads").unwrap() > 0);
                        }

                        // Literal true removes only its own indexed gate. It
                        // cannot enable a disabled source or a hidden/support
                        // effect, and it does not determine the count value.
                        for gate in ["enabled", "support", "hidden"] {
                            let old_support: Value = second.get("support").unwrap();
                            let old_hidden: Value = second.get("hideFromSideBar").unwrap();
                            match gate {
                                "enabled" => row.set("enabled", false).unwrap(),
                                "support" => second.set("support", true).unwrap(),
                                "hidden" => second.set("hideFromSideBar", true).unwrap(),
                                _ => unreachable!(),
                            }
                            let expected = if gate == "enabled" {
                                vec![]
                            } else {
                                vec![first_id.to_owned()]
                            };
                            assert_eq!(active_ids(&lua, &group, &row), expected);
                            assert_eq!(export_count(&lua, &group, &row), expected.len());
                            row.set("enabled", true).unwrap();
                            second.set("support", old_support).unwrap();
                            second.set("hideFromSideBar", old_hidden).unwrap();
                        }
                    }
                }
            }
        }
    }
}

const SINGLE_SUPPORT: &str = "SupportRapidCastingPlayer";
const CHILD_SUPPORT: &str = "SupportCorruptingCryPlayer";
const SUPPORT_CHILD: &str = "TriggeredCorruptingCryPlayer";

fn catalog_row<'a>(collection: &'a Json, field: &str, value: &str) -> &'a Json {
    let rows = collection
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row[field] == value)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1, "exact catalog join: {field}={value}");
    rows[0]
}

/// Additional real declarations come from the SAME already authenticated
/// catalog/manifest. The old packet is unchanged; these are component anchors,
/// not an assertion that their complete effects or receiving rules execute.
fn support_components(jit: bool) -> Lua {
    let lua = components(jit);
    let p = packet();
    let mut definitions = p.boundary("mod_helpers");
    definitions.push_str(
        "\nlocal mod,flag,skill=makeSkillMod,makeFlagMod,makeSkillDataMod\nlocal skills=data.skills\n",
    );
    for id in [SINGLE_SUPPORT, CHILD_SUPPORT, SUPPORT_CHILD] {
        let row = catalog_row(&p.catalog["skills"], "id", id);
        let declaration = p.catalog["skill_declarations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|candidate| candidate["index"] == row["winning_declaration"])
            .collect::<Vec<_>>();
        assert_eq!(declaration.len(), 1);
        let declaration = declaration[0];
        assert_eq!(declaration["id"], id);
        assert_eq!(declaration["identity"]["support"], row["support"]);
        let pin = &declaration["source"];
        // This API validates the complete source bytes against the manifest
        // whose identity packet() authenticated, before taking the exact slice.
        let body = source::read_verified_text(
            &root().join("vendor/path-of-building-poe2"),
            text(&pin["path"]),
        )
        .unwrap();
        let first = pin["line"].as_u64().unwrap() as usize;
        let last = pin["end_line"].as_u64().unwrap() as usize;
        let declaration = body
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(declaration.lines().count(), last - first + 1);
        assert_eq!(hash(declaration.as_bytes()), text(&pin["sha256"]));
        definitions.push_str(&declaration);
        definitions.push_str(&format!(
            "\nskills['{id}'].id='{id}';skills['{id}'].modSource='Skill:{id}'\n"
        ));
    }
    lua.load(definitions)
        .set_name("@authenticated-support-declarations")
        .exec()
        .unwrap();
    for id in [SINGLE_SUPPORT, CHILD_SUPPORT, SUPPORT_CHILD] {
        assert_eq!(
            effect(&lua, id).get::<Option<bool>>("support").unwrap(),
            catalog_row(&p.catalog["skills"], "id", id)["support"].as_bool()
        );
    }
    // Only reads of the saved usage fields are intercepted. Identity, enabled
    // state and source construction remain ordinary tables. This instrumentation
    // neither constructs an active effect nor decides which source owns it.
    lua.load(
        r#"
        function supportUsageProbe(row, count, primary, secondary)
            local probe = {
                count = count, enableGlobal1 = primary, enableGlobal2 = secondary,
                countReads = 0, primaryReads = 0, secondaryReads = 0
            }
            setmetatable(row, { __index = function(_, key)
                if key == "count" then
                    probe.countReads = probe.countReads + 1
                    return probe.count
                elseif key == "enableGlobal1" then
                    probe.primaryReads = probe.primaryReads + 1
                    return probe.enableGlobal1
                elseif key == "enableGlobal2" then
                    probe.secondaryReads = probe.secondaryReads + 1
                    return probe.enableGlobal2
                end
            end })
            return probe
        end
        "#,
    )
    .exec()
    .unwrap();
    lua
}

fn construct_catalog_support(lua: &Lua, id: &str) -> (Table, Table) {
    let p = packet();
    let row = catalog_row(&p.catalog["gems"], "primary_effect_id", id);
    let declaration = p.catalog["gem_declarations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|candidate| candidate["index"] == row["winning_declaration"])
        .collect::<Vec<_>>();
    assert_eq!(declaration.len(), 1);
    assert_eq!(declaration[0]["key"], row["key"]);
    p.declaration(declaration[0]);
    let raw: Table = lua
        .globals()
        .get::<Table>("allGems")
        .unwrap()
        .get(text(&row["key"]))
        .unwrap();
    assert_eq!(raw.get::<String>("grantedEffectId").unwrap(), id);
    assert_eq!(raw.get::<String>("gameId").unwrap(), text(&row["game_id"]));
    assert_eq!(
        raw.get::<String>("variantId").unwrap(),
        text(&row["variant_id"])
    );
    let gem = copy(lua, &raw);
    lua.globals()
        .get::<Function>("constructGem")
        .unwrap()
        .call::<()>((gem.clone(), text(&row["key"])))
        .unwrap();
    let ids = gem
        .get::<Table>("grantedEffectList")
        .unwrap()
        .sequence_values::<Table>()
        .map(|entry| entry.unwrap().get::<String>("id").unwrap())
        .collect::<Vec<_>>();
    assert_eq!(json!(ids), row["effect_list"]);
    (raw, gem)
}

fn support_occurrence(
    lua: &Lua,
    gem: &Table,
    count: i64,
    primary: Option<bool>,
    secondary: Option<bool>,
) -> (Table, Table) {
    let row = lua.create_table().unwrap();
    row.set("gemData", gem.clone()).unwrap();
    row.set("enabled", true).unwrap();
    let probe = lua
        .globals()
        .get::<Function>("supportUsageProbe")
        .unwrap()
        .call((row.clone(), count, primary, secondary))
        .unwrap();
    (row, probe)
}

fn usage_reads(probe: &Table) -> (u32, u32, u32) {
    (
        probe.get("countReads").unwrap(),
        probe.get("primaryReads").unwrap(),
        probe.get("secondaryReads").unwrap(),
    )
}

/// Feed the actual identity tables returned by the authenticated admission loop
/// to the actual count function. The synthetic body does not prepare modifiers,
/// validate compatibility, retain supports, or establish gameplay triggers.
fn admitted_source_counts(
    lua: &Lua,
    group: &Table,
) -> Vec<(String, *const std::ffi::c_void, i64, bool)> {
    let active: Table = lua
        .globals()
        .get::<Function>("admitted")
        .unwrap()
        .call(group.clone())
        .unwrap();
    let count = lua
        .globals()
        .get::<Table>("calcs")
        .unwrap()
        .get::<Function>("getActiveSkillCount")
        .unwrap();
    active
        .sequence_values::<Table>()
        .map(|active| {
            let active = active.unwrap();
            active.set("socketGroup", group.clone()).unwrap();
            let effect: Table = active.get("activeEffect").unwrap();
            let source: Table = effect.get("srcInstance").unwrap();
            let granted: Table = effect.get("grantedEffect").unwrap();
            let (count, enabled) = count.call::<(i64, bool)>(active).unwrap();
            (
                granted.get("id").unwrap(),
                source.to_pointer(),
                count,
                enabled,
            )
        })
        .collect()
}

#[test]
fn singleton_support_usage_fields_cannot_donate_to_a_distinct_admitted_receiver() {
    for jit in [false, true] {
        let lua = support_components(jit);
        let (raw, gem) = construct_catalog_support(&lua, SINGLE_SUPPORT);
        let support = effect(&lua, SINGLE_SUPPORT);
        assert!(singleton_candidate(&raw, &gem, &support));
        let receiver_gem = construct(&lua, "IceNovaPlayer");
        let receiver_effect = effect(&lua, "IceNovaPlayer");
        assert_ne!(support.to_pointer(), receiver_effect.to_pointer());
        let (receiver, _) = occurrence(&lua, &receiver_gem, Some(true));
        receiver.set("count", 7).unwrap();
        for vaal in [false, true] {
            gem.set("vaalGem", vaal).unwrap(); // Synthetic branch control.
            for metadata in [None, Some(false), Some(true)] {
                set_global_metadata(&lua, &support, metadata);
                for primary in [None, Some(false), Some(true)] {
                    for secondary in [None, Some(false), Some(true)] {
                        for receiver_first in [false, true] {
                            let (one, one_probe) =
                                support_occurrence(&lua, &gem, 13, primary, secondary);
                            let (two, two_probe) =
                                support_occurrence(&lua, &gem, 17, primary, secondary);
                            assert_ne!(one.to_pointer(), two.to_pointer());
                            let group = group(&lua, &receiver);
                            let rows = if receiver_first {
                                [receiver.clone(), one.clone(), two.clone()]
                            } else {
                                [two.clone(), one.clone(), receiver.clone()]
                            };
                            group
                                .set("gemList", lua.create_sequence_from(rows).unwrap())
                                .unwrap();
                            assert_eq!(
                                admitted_source_counts(&lua, &group),
                                [("IceNovaPlayer".into(), receiver.to_pointer(), 7, true)]
                            );
                            assert_eq!(export_count(&lua, &group, &receiver), 1);
                            for row in [&one, &two] {
                                let only_support = group_with_rows(&lua, [row.clone()]);
                                assert!(admitted_source_counts(&lua, &only_support).is_empty());
                                assert_eq!(export_count(&lua, &only_support, row), 0);
                                let shown = visibility(&lua, &only_support);
                                assert_eq!((shown.0, shown.1), (false, false));
                                assert_ne!(shown.2, Some(true));
                            }
                            assert_eq!(usage_reads(&one_probe), (0, 0, 0));
                            assert_eq!(usage_reads(&two_probe), (0, 0, 0));
                            group.set("groupCount", 5).unwrap();
                            assert_eq!(
                                admitted_source_counts(&lua, &group),
                                [("IceNovaPlayer".into(), receiver.to_pointer(), 5, true)]
                            );
                            assert_eq!(usage_reads(&one_probe), (0, 0, 0));
                            assert_eq!(usage_reads(&two_probe), (0, 0, 0));
                        }
                    }
                }
            }
        }
    }
}

fn group_with_rows<const N: usize>(lua: &Lua, rows: [Table; N]) -> Table {
    let group = lua.create_table().unwrap();
    group
        .set("gemList", lua.create_sequence_from(rows).unwrap())
        .unwrap();
    group
}

#[test]
fn a_real_support_child_has_its_own_source_count_and_secondary_gate() {
    for jit in [false, true] {
        let lua = support_components(jit);
        let (raw, gem) = construct_catalog_support(&lua, CHILD_SUPPORT);
        assert!(!singleton_candidate(
            &raw,
            &gem,
            &effect(&lua, CHILD_SUPPORT)
        ));
        let child = effect(&lua, SUPPORT_CHILD);
        assert_ne!(child.get::<Option<bool>>("support").unwrap(), Some(true));
        assert_ne!(
            child.get::<Option<bool>>("hideFromSideBar").unwrap(),
            Some(true)
        );
        // The child declaration is real. Synthetic metadata exercises the exact
        // indexed gate; it makes no claim that the real Cry initializes it.
        set_global_metadata(&lua, &child, Some(true));
        let receiver_gem = construct(&lua, "IceNovaPlayer");
        let (receiver, _) = occurrence(&lua, &receiver_gem, Some(true));
        receiver.set("count", 7).unwrap();
        for secondary in [false, true] {
            let (source, probe) = support_occurrence(&lua, &gem, 13, Some(false), Some(secondary));
            let group = group_with_rows(&lua, [source.clone(), receiver.clone()]);
            let mut expected = Vec::new();
            if secondary {
                expected.push((SUPPORT_CHILD.into(), source.to_pointer(), 13, true));
            }
            expected.push(("IceNovaPlayer".into(), receiver.to_pointer(), 7, true));
            assert_eq!(admitted_source_counts(&lua, &group), expected);
            let (count, primary, secondary_reads) = usage_reads(&probe);
            assert_eq!(primary, 0, "support primary is excluded before its gate");
            assert_eq!(count, u32::from(secondary));
            assert!(secondary_reads > 0, "positive observer access control");
        }
        // No metadata-derived gate is needed for the real additional-effect
        // count relation. This is still construction identity, not eligibility
        // to trigger, prepared level, or calculated damage.
        child.set("hasGlobalEffect", Value::Nil).unwrap();
        let (source, probe) = support_occurrence(&lua, &gem, 19, Some(false), Some(false));
        let group = group_with_rows(&lua, [receiver.clone(), source.clone()]);
        assert_eq!(
            admitted_source_counts(&lua, &group),
            [
                ("IceNovaPlayer".into(), receiver.to_pointer(), 7, true),
                (SUPPORT_CHILD.into(), source.to_pointer(), 19, true),
            ]
        );
        assert_eq!(usage_reads(&probe), (1, 0, 0));
    }
}

#[test]
fn direct_vaal_support_count_probe_is_not_an_actual_active_effect_consumer() {
    for jit in [false, true] {
        let lua = support_components(jit);
        let (_, gem) = construct_catalog_support(&lua, SINGLE_SUPPORT);
        let support = effect(&lua, SINGLE_SUPPORT);
        for primary in [false, true] {
            gem.set("vaalGem", true).unwrap(); // Deliberate synthetic trap.
            let (row, probe) = support_occurrence(&lua, &gem, 23, Some(primary), Some(false));
            let group = group_with_rows(&lua, [row.clone()]);
            assert!(admitted_source_counts(&lua, &group).is_empty());
            assert_eq!(usage_reads(&probe), (0, 0, 0));
            // An arbitrary direct helper query bypasses admission and DOES
            // consult the Vaal primary. A zero-read assertion must use the
            // actual constructed receiver instead of this manufactured input.
            assert_eq!(count_for(&lua, &group, &support), (23, primary));
            assert_eq!(usage_reads(&probe), (1, 1, 0));
            gem.set("vaalGem", false).unwrap();
            assert_eq!(count_for(&lua, &group, &support), (1, true));
            assert_eq!(usage_reads(&probe), (1, 1, 0));
        }
    }
}

// Generated-source COMPONENT observations. These additions use the exact saved
// Firebolt frame and actual source routing functions, not an Import receipt or
// a complete CalcSetup/evaluator. The host supplies an already-discovered grant;
// item/tree acquisition, explosion/Thorns factories and support preparation are
// deliberately outside this bounded route. All original input bytes stay intact.
fn generated_routing_span(name: &str) -> String {
    let (path, first, last, digest) = match name {
        "process" => (
            "src/Classes/SkillsTab.lua",
            1242,
            1322,
            "8737e8311f6cbef4b9bc10525e28831d00f52e602494a7f2bdfbccee68a33f32",
        ),
        "load_group" => (
            "src/Classes/SkillsTab.lua",
            308,
            317,
            "53649e0303418dd509f1196a2966c10931552643b10470e22f1b96e1e7e5a652",
        ),
        "load_gem" => (
            "src/Classes/SkillsTab.lua",
            319,
            361,
            "bf8f8770f61527ac3443caf0cee485186060dd84b578769002a4b27d57767585",
        ),
        "normalized" => (
            "src/Modules/CalcSetup.lua",
            667,
            676,
            "0204eac6356b36cd8f23799a373653d04332174edc5998d768ed11192fe9c3e4",
        ),
        "update" => (
            "src/Modules/CalcSetup.lua",
            1690,
            1734,
            "0452d3bc3b3318477771421a87aaeeaaab5710c6796c71997e781d0326b20705",
        ),
        "cleanup" => (
            "src/Modules/CalcSetup.lua",
            1837,
            1850,
            "b056255b34bc9c33ff78218fba23683f387d04c6ef619f62fcc41e3444b57d7d",
        ),
        "selected" => (
            "src/Modules/CalcSetup.lua",
            2141,
            2154,
            "2243d1734717c0349251cf1cc02f58019b15c978b2fbb3ca8438d9ea382c190c",
        ),
        "export" => (
            "src/Modules/BuildExportPoE2.lua",
            137,
            146,
            "927943e1709a5670917d3488649afa3c6ee11a72e6b077c6049080e90e8ad1df",
        ),
        "wipe" => (
            "src/Modules/Common.lua",
            564,
            572,
            "eff97e60d9c16582d74b3f1430414f7129e6a805eb5773bbccae840b199209ad",
        ),
        "sanitize" => (
            "src/Modules/Common.lua",
            321,
            347,
            "ad81cfbd6579b726668f7069f9d58dab8ee166ca9243e31fd56a11fb7e3ab682",
        ),
        "round" => (
            "src/Modules/Common.lua",
            722,
            728,
            "90585272c160e302702e5071f1ad55c649b8da02e83ef2d776d89a449250e1f2",
        ),
        "validate" => (
            "src/Modules/CalcTools.lua",
            60,
            76,
            "65f04133f20851a940a03900c9eb401db0baacabe048c97dc418bf4008d55fa7",
        ),
        "requirements" => (
            "src/Modules/CalcTools.lua",
            152,
            158,
            "ea60919f8f7438801f81cdc38f505e068269a99edb2550197a7caa624f45664f",
        ),
        _ => panic!("unknown generated routing span"),
    };
    static TOOLS: OnceLock<String> = OnceLock::new();
    let body = if path == "src/Modules/CalcTools.lua" {
        TOOLS.get_or_init(|| {
            source::read_verified_text(&root().join("vendor/path-of-building-poe2"), path).unwrap()
        })
    } else {
        &packet().sources[path]
    };
    let span = body
        .lines()
        .skip(first - 1)
        .take(last - first + 1)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(span.lines().count(), last - first + 1);
    assert_eq!(hash(span.as_bytes()), digest);
    span
}

fn generated_routing_components(jit: bool) -> Lua {
    let lua = components(jit);
    let data: Table = lua.globals().get("data").unwrap();
    let gems = lua.create_table().unwrap();
    for primary in SINGLETONS {
        let gem = construct(&lua, primary);
        gems.set(text(&packet().family(primary)["gem"]["key"]), gem)
            .unwrap();
    }
    data.set("gems", gems).unwrap();
    // The complete saved source bytes are independently authenticated by packet().
    // Selecting this occurrence is test setup, never a runtime build-name rule.
    let original = pinned(&packet().evidence["original"]);
    let doc = roxmltree::Document::parse(std::str::from_utf8(&original).unwrap()).unwrap();
    let groups: Vec<_> = doc
        .descendants()
        .filter(|n| {
            n.has_tag_name("Skill")
                && n.attribute("source") == Some("Item:28:New Item, Ashen Staff")
                && n.parent()
                    .is_some_and(|p| p.has_tag_name("SkillSet") && p.attribute("id") == Some("4"))
        })
        .collect();
    assert_eq!(groups.len(), 1);
    let children: Vec<_> = groups[0].children().filter(|n| n.is_element()).collect();
    assert_eq!(children.len(), 1);
    assert!(children[0].has_tag_name("Gem"));
    assert_eq!(children[0].attribute("skillId"), Some("FireboltPlayer"));
    for (name, node) in [
        ("savedGeneratedGroup", groups[0]),
        ("savedGeneratedGem", children[0]),
    ] {
        let attrs = lua.create_table().unwrap();
        for a in node.attributes() {
            attrs.set(a.name(), a.value()).unwrap();
        }
        lua.globals().set(name, attrs).unwrap();
    }
    let support = [
        "wipe",
        "sanitize",
        "round",
        "validate",
        "requirements",
        "process",
        "normalized",
    ]
    .map(generated_routing_span)
    .join("\n");
    lua.load(format!(
        "local m_min,m_max,m_floor=math.min,math.max,math.floor\n\
         local t_insert,t_remove=table.insert,table.remove\n\
         local activeSkillGems=exportActive\n\
         calcLib={{}}\nSkillsTabClass={{}}\n{}\n\
         function loadGenerated(groupAttributes,gemAttributes)\n\
         local self=setmetatable({{build={{data=data}}}},{{__index=SkillsTabClass}})\n\
         local node={{attrib=groupAttributes}}\n{}\n\
         local child={{attrib=gemAttributes}}\n{}\n\
         socketGroup.gemList[1]=gemInstance\n\
         self:ProcessSocketGroup(socketGroup)\nreturn socketGroup,gemInstance end\n\
         function generatedBuild(groups)\nlocal build={{data=data}}\n\
         build.skillsTab=setmetatable({{build=build,socketGroupList=groups}},{{__index=SkillsTabClass}})\nreturn build end\n\
         function updateGenerated(build,grants,mode)\n\
         local env={{mode=mode,grantedSkills=grants}}\nlocal accelerate={{}}\nlocal tempTable1={{}}\n{}\n{}\nend\nreturn build.skillsTab.socketGroupList end\n\
         function generatedSelected(group,mode,list,selectedGroup)\n\
         local env={{mode=mode,mainSocketGroup=selectedGroup,player={{}}}}\nlocal index=1\n\
         local socketGroupSkillList=list\n{}\nreturn env.player.mainSkill end\n\
         function generatedExportSelected(group)\n{}\nreturn activeGem\nend\nend",
        support, generated_routing_span("load_group"), generated_routing_span("load_gem"),
        generated_routing_span("update"), generated_routing_span("cleanup"),
        generated_routing_span("selected"), generated_routing_span("export"),
    )).set_name("@authenticated-generated-routing-components").exec().unwrap();
    lua.load(r#"
        function actualGeneratedFrame()
            return loadGenerated(copyTable(savedGeneratedGroup),copyTable(savedGeneratedGem))
        end
        function suppliedGeneratedGrant()
            -- Host-supplied acquisition result, keyed to this exact source frame.
            -- This is not proof of item parsing, equipment selection or rolls.
            return {source=savedGeneratedGroup.source,slotName=savedGeneratedGroup.slot,
                skillId=savedGeneratedGem.skillId,level=tonumber(savedGeneratedGem.level),
                nameSpec=savedGeneratedGem.nameSpec,sourceItem={testIdentity='exact-supplier'}}
        end
        function observeGenerated(group,row,probe,expected)
            local active=admitted(group)
            assert(#active==expected)
            for _,a in ipairs(active) do
                assert(a.activeEffect.srcInstance==row)
                assert(a.activeEffect.grantedEffect==data.skills.FireboltPlayer)
                a.socketGroup=group
                local count,enabled=calcs.getActiveSkillCount(a)
                assert(count==(group.groupCount or row.count) and enabled==true)
            end
            for _,mode in ipairs({'MAIN','CALCS'}) do
                local chosen=generatedSelected(group,mode,active,1)
                assert(chosen==(expected==1 and active[1] or nil))
                assert(generatedSelected(group,mode,active,2)==nil)
            end
            for _,calculated in ipairs({false,true}) do
                group.displaySkillList=calculated and active or nil
                local chosen=generatedExportSelected(group)
                assert(chosen==(expected==1 and row or nil))
            end
            group.displaySkillList=nil
            if probe then assert(probe.reads==0) end
            return tostring(#active)..':'..tostring(group.mainActiveSkill)..':'..tostring(group.mainActiveSkillCalcs)
        end
    "#).set_name("@explicit-generated-identity-test-host").exec().unwrap();
    lua
}

#[test]
fn actual_saved_generated_singleton_uses_catalog_routing_in_independent_lifecycles() {
    let mut expected = None;
    for jit in [false, true] {
        for _replay in 0..2 {
            let lua = generated_routing_components(jit);
            let observed: String = lua.load(r#"
                local key='Metadata/Items/Gems/SkillGemFirebolt'
                local candidates=0
                for id,gem in pairs(allGems) do
                    if gem.grantedEffectId=='FireboltPlayer' then
                        candidates=candidates+1;assert(id==key)
                    end
                end
                assert(candidates==1)
                local g,row=actualGeneratedFrame()
                local primary=data.skills.FireboltPlayer
                local gem=data.gems[key]
                assert(gem.additionalStatSet1=='FireboltExplosionPlayer')
                assert(#gem.grantedEffectList==1 and gem.grantedEffectList[1]==primary)
                assert(gem.additionalGrantedEffectId1==nil)
                assert(row.gemData==gem and row.grantedEffect==nil)
                assert(row.count==1 and row.level==11 and row.quality==0)
                assert(row.enableGlobal1==true and row.enableGlobal2==false)
                row.enableGlobal2=nil
                local probe=secondaryProbe(row,false)
                local build=generatedBuild({g})
                local grant=suppliedGeneratedGrant()
                local signatures={}
                -- Standalone CALCS first must not borrow a preceding MAIN update.
                for _,mode in ipairs({'CALCS','MAIN','CALCS','MAIN'}) do
                    local groups=updateGenerated(build,{grant},mode)
                    assert(#groups==1 and groups[1]==g and g.gemList[1]==row)
                    assert(row.gemData==gem and row.skillId=='FireboltPlayer')
                    if mode=='MAIN' then assert(g.sourceItem==grant.sourceItem and row.fromItem) end
                    signatures[#signatures+1]=observeGenerated(g,row,probe,1)
                end
                row.count=3
                observeGenerated(g,row,probe,1) -- Distinguishes catalog from direct fallback count1.
                g.groupCount=0;observeGenerated(g,row,probe,1)
                g.groupCount=4;observeGenerated(g,row,probe,1)
                assert(probe.reads==0)
                return table.concat(signatures,'|')
            "#).eval().unwrap();
            if let Some(expected) = &expected {
                assert_eq!(&observed, expected);
            } else {
                expected = Some(observed);
            }
        }
    }
}

#[test]
fn generated_singleton_secondary_and_independent_selectors_survive_zero_one_zero_filters() {
    for jit in [false, true] {
        let lua = generated_routing_components(jit);
        lua.load(r#"
            for _,main in ipairs({'absent','nil','1'}) do
                for _,calcs in ipairs({'absent','nil','1'}) do
                    for _,secondary in ipairs({'absent','nil','false','true'}) do
                        for _,filter in ipairs({'hidden','disabled','global'}) do
                            local ga,ra=copyTable(savedGeneratedGroup),copyTable(savedGeneratedGem)
                            ga.mainActiveSkill=main~='absent' and main or nil
                            ga.mainActiveSkillCalcs=calcs~='absent' and calcs or nil
                            ra.enableGlobal2=secondary~='absent' and secondary or nil
                            local g,row=loadGenerated(ga,ra)
                            local decoded=row.enableGlobal2
                            assert(decoded==(secondary=='true'))
                            row.enableGlobal2=nil
                            local probe=secondaryProbe(row,decoded)
                            local primary=data.skills.FireboltPlayer
                            local oldHidden,oldGlobal=primary.hideFromSideBar,primary.hasGlobalEffect
                            local build=generatedBuild({g})
                            updateGenerated(build,{suppliedGeneratedGrant()},'MAIN')
                            assert(build.skillsTab.socketGroupList[1]==g and g.gemList[1]==row)
                            for _,closed in ipairs({true,false,true}) do
                                if filter=='hidden' then primary.hideFromSideBar=closed
                                elseif filter=='disabled' then row.enabled=not closed
                                else primary.hasGlobalEffect=true;row.enableGlobal1=not closed end
                                observeGenerated(g,row,probe,closed and 0 or 1)
                                assert(g.mainActiveSkill==1 and g.mainActiveSkillCalcs==1)
                            end
                            primary.hideFromSideBar,primary.hasGlobalEffect=oldHidden,oldGlobal
                        end
                    end
                end
            end
            -- Independent out-of-domain zero controls expose accidental shared selectors.
            local g,row=actualGeneratedFrame()
            local active=admitted(g)
            g.mainActiveSkill=0;g.mainActiveSkillCalcs=1
            assert(generatedSelected(g,'MAIN',active,1)==nil)
            assert(generatedSelected(g,'CALCS',active,1)==active[1])
            assert(generatedExportSelected(g)==nil)
            g.mainActiveSkill=1;g.mainActiveSkillCalcs=0
            assert(generatedSelected(g,'MAIN',active,1)==active[1])
            assert(generatedSelected(g,'CALCS',active,1)==nil)
            assert(generatedExportSelected(g)==row)
        "#).exec().unwrap();
    }
}

#[test]
fn generated_routing_counterexamples_preserve_exact_source_and_cardinality_boundaries() {
    for jit in [false, true] {
        let lua = generated_routing_components(jit);
        lua.load(r#"
            -- The actual constructor replaces unmatched saved groups. A same
            -- effect output is not a receipt for the old physical source object.
            for _,field in ipairs({'source','slot','level'}) do
                local g,row=actualGeneratedFrame()
                if field=='level' then row.level=12 else g[field]='unmatched-source-control' end
                local build=generatedBuild({g})
                local groups=updateGenerated(build,{suppliedGeneratedGrant()},'MAIN')
                assert(#groups==1 and groups[1]~=g and groups[1].gemList[1]~=row)
                assert(groups[1].gemList[1].skillId=='FireboltPlayer')
            end
            local a,ar=actualGeneratedFrame()
            local b,br=actualGeneratedFrame()
            local duplicates=updateGenerated(generatedBuild({a,b}),{suppliedGeneratedGrant()},'MAIN')
            assert(#duplicates==1)
            assert(duplicates[1].gemList[1]==ar or duplicates[1].gemList[1]==br)
            -- Do not claim which duplicate pairs() chooses. Existing Import
            -- correspondence must refuse this ambiguous saved representation.

            local primary=data.skills.FireboltPlayer
            local map=data.gemForSkill
            local savedMap=map[primary]
            map[primary]=nil
            local ga,ra=copyTable(savedGeneratedGroup),copyTable(savedGeneratedGem)
            ra.gemId=nil;ra.variantId=nil;ra.count='3'
            local g,row=loadGenerated(ga,ra)
            assert(row.gemData==nil and row.grantedEffect==primary)
            updateGenerated(generatedBuild({g}),{suppliedGeneratedGrant()},'MAIN')
            assert(row.gemData==nil and row.grantedEffect==primary)
            local active=admitted(g)
            assert(#active==1 and active[1].activeEffect.srcInstance==row)
            active[1].socketGroup=g
            local count,enabled=calcs.getActiveSkillCount(active[1])
            assert(count==1 and enabled==true and row.count==3)
            map[primary]=savedMap

            -- A second catalog Gem sharing the primary can change the reverse
            -- route without changing the primary ID. Real constructor used on
            -- an explicitly synthetic declaration; no new gameplay claim.
            local alias=copyTable(allGems['Metadata/Items/Gems/SkillGemFirebolt'])
            alias.variantId='SyntheticFireboltAlias'
            alias.additionalGrantedEffectId1='IceNovaPlayer'
            constructGem(alias,'SyntheticFireboltAlias')
            data.gems.SyntheticFireboltAlias=alias
            local other=data.skills.IceNovaPlayer
            other.hasGlobalEffect=true
            local g,row=actualGeneratedFrame()
            row.enableGlobal2=nil
            local probe=secondaryProbe(row,false)
            updateGenerated(generatedBuild({g}),{suppliedGeneratedGrant()},'MAIN')
            assert(row.gemData==alias and #alias.grantedEffectList==2)
            assert(#admitted(g)==1 and probe.reads>0)
            probe.value=true
            local active=admitted(g)
            assert(#active==2 and active[2].activeEffect.srcInstance==row)
            assert(active[2].activeEffect.grantedEffect==other)
            g.mainActiveSkill=1;g.mainActiveSkillCalcs=2
            assert(generatedSelected(g,'MAIN',active,1)==active[1])
            assert(generatedSelected(g,'CALCS',active,1)==active[2])
        "#).exec().unwrap();
    }
}

#[test]
fn generated_singleton_selector_neutrality_survives_actual_group_replacement() {
    let mut expected = None;
    for jit in [false, true] {
        for _replay in 0..2 {
            let lua = generated_routing_components(jit);
            let observed: String = lua.load(r#"
                local signatures={}
                local primary=data.skills.FireboltPlayer
                local oldHidden=primary.hideFromSideBar
                for _,main in ipairs({'nil','1'}) do
                    for _,calcs in ipairs({'nil','1'}) do
                        for _,secondary in ipairs({'nil','false','true'}) do
                            local previous={}
                            for _,replacement in ipairs({false,true}) do
                                local ga,ra=copyTable(savedGeneratedGroup),copyTable(savedGeneratedGem)
                                ga.mainActiveSkill=main
                                ga.mainActiveSkillCalcs=calcs
                                ra.enableGlobal2=secondary
                                -- Exact level12 is only a source matching control:
                                -- the host supplies level11, so the real update
                                -- must replace this saved group. No native level,
                                -- quality, count or participation claim follows.
                                if replacement then ra.level='12' end
                                local savedGroup,savedRow=loadGenerated(ga,ra)
                                assert(savedGroup.mainActiveSkill==1 and savedGroup.mainActiveSkillCalcs==1)
                                local build=generatedBuild({savedGroup})
                                local grant=suppliedGeneratedGrant()
                                local groups=updateGenerated(build,{grant},'MAIN')
                                assert(#groups==1)
                                local group,row=groups[1],groups[1].gemList[1]
                                assert(#group.gemList==1)
                                if replacement then
                                    assert(group~=savedGroup and row~=savedRow)
                                    assert(group.mainActiveSkill==nil and group.mainActiveSkillCalcs==nil)
                                    assert(row.enableGlobal2==nil)
                                else
                                    assert(group==savedGroup and row==savedRow)
                                    assert(group.mainActiveSkill==1 and group.mainActiveSkillCalcs==1)
                                    assert(row.enableGlobal2==(secondary=='true'))
                                end
                                -- Preserve the observed state difference in the
                                -- replay signature. Only the selected-effect
                                -- meaning, not the objects or raw state, agrees.
                                signatures[#signatures+1]=table.concat({
                                    tostring(replacement),main,calcs,secondary,
                                    tostring(group.mainActiveSkill),tostring(group.mainActiveSkillCalcs),
                                    tostring(row.enableGlobal2)},':')
                                assert(group.source==grant.source and group.slot==grant.slotName)
                                assert(row.gemData==data.gems['Metadata/Items/Gems/SkillGemFirebolt'])
                                assert(row.grantedEffect==nil)
                                local originalSecondary=row.enableGlobal2
                                row.enableGlobal2=nil
                                local probe=secondaryProbe(row,originalSecondary)
                                local trace={}
                                -- This predicate control isolates selection
                                -- cardinality; it does not model gameplay
                                -- activation or compare enabled/count values.
                                for _,closed in ipairs({true,false,true}) do
                                    primary.hideFromSideBar=closed
                                    local active=admitted(group)
                                    local count=closed and 0 or 1
                                    assert(#active==count)
                                    for _,skill in ipairs(active) do
                                        assert(skill.activeEffect.srcInstance==row)
                                        assert(skill.activeEffect.grantedEffect==primary)
                                    end
                                    for _,mode in ipairs({'MAIN','CALCS'}) do
                                        local chosen=generatedSelected(group,mode,active,1)
                                        assert(chosen==(count==1 and active[1] or nil))
                                        assert(generatedSelected(group,mode,active,2)==nil)
                                        trace[#trace+1]=mode..':'..tostring(chosen and chosen.activeEffect.grantedEffect.id)
                                    end
                                    for _,calculated in ipairs({false,true}) do
                                        group.displaySkillList=calculated and active or nil
                                        assert(generatedExportSelected(group)==(count==1 and row or nil))
                                        trace[#trace+1]='export:'..tostring(calculated)..':'..tostring(count)
                                    end
                                    group.displaySkillList=nil
                                    assert(probe.reads==0)
                                end
                                primary.hideFromSideBar=oldHidden
                                if replacement then
                                    assert(table.concat(trace,'|')==table.concat(previous,'|'))
                                else
                                    previous=trace
                                end
                                signatures[#signatures+1]=table.concat(trace,'|')
                            end
                        end
                    end
                end
                return table.concat(signatures,'\n')
            "#).eval().unwrap();
            if let Some(expected) = &expected {
                assert_eq!(&observed, expected);
            } else {
                expected = Some(observed);
            }
        }
    }
}

#[path = "support/constructed_source_selector.rs"]
mod constructed_source_selector;
