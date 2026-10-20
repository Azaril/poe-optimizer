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
