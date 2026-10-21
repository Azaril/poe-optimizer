//! Reverse-primary dispatch COMPONENT evidence, not native or Import authority.
//! Executes complete pinned skill/Gem assembly and the actual saved/generated
//! dispatchers. Does not execute provider acquisition, active-effect preparation,
//! support eligibility, damage, or a complete build. Existing source/slot/level
//! replacement and selector controls remain in owned_secondary_effect_switch_source.
#![cfg(not(target_arch = "wasm32"))]

use mlua::{Function, Lua, Table, Value};
use poe_optimizer_pob::source;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

const CATALOG: &str = "data/owned/poe2/3887ae68/import/skill-identities.json";
const PRIMARIES: [&str; 3] = [
    "FireboltPlayer",
    "SummonSandDjinnPlayer",
    "SummonWaterDjinnPlayer",
];
const COMMANDS: [&str; 2] = [
    "CommandSandDjinnKnifeThrowPlayer",
    "CommandWaterDjinnBubblePlayer",
];
const MODULES: [&str; 9] = [
    "act_str", "act_dex", "act_int", "other", "minion", "spectre", "sup_str", "sup_dex", "sup_int",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn catalog() -> &'static Json {
    static CATALOG_DATA: OnceLock<Json> = OnceLock::new();
    CATALOG_DATA.get_or_init(|| {
        let bytes = fs::read(root().join(CATALOG)).unwrap();
        assert_eq!(bytes.len(), 2_247_063);
        assert_eq!(
            hash(&bytes),
            "b9064ec447ccf3c3fc88244655a36bbb4453e50e16ffa64eda9880f756aed10e"
        );
        serde_json::from_slice(&bytes).unwrap()
    })
}
fn sources() -> &'static BTreeMap<String, String> {
    static SOURCES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        assert_eq!(
            source::UPSTREAM_REVISION,
            "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
        );
        assert_eq!(
            source::manifest_sha256(),
            "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
        );
        let mut paths = vec![
            "src/Modules/Common.lua".into(),
            "src/Modules/Data.lua".into(),
            "src/Modules/CalcSetup.lua".into(),
            "src/Modules/CalcTools.lua".into(),
            "src/Classes/SkillsTab.lua".into(),
            "src/Data/Gems.lua".into(),
            "src/Data/Global.lua".into(),
            "src/Data/SkillStatMap.lua".into(),
            "src/Data/Assets.lua".into(),
            "src/Data/Skills/SkillAssets.lua".into(),
        ];
        paths.extend(MODULES.map(|name| format!("src/Data/Skills/{name}.lua")));
        paths
            .into_iter()
            .map(|path| {
                let body =
                    source::read_verified_text(&root().join("vendor/path-of-building-poe2"), &path)
                        .unwrap();
                if let Some(expected) = catalog()["source"]["files"].get(&path) {
                    assert_eq!(hash(body.as_bytes()), expected.as_str().unwrap());
                }
                (path, body)
            })
            .collect()
    })
}
fn span(name: &str) -> String {
    let (path, first, last, digest) = match name {
        "prefix" => (
            "src/Modules/Data.lua",
            7,
            98,
            "ade70317f2414e729ec5e35bcc425cbba5ed0ccad12bbf4ff1f1ffe4380b149b",
        ),
        "assembly" => (
            "src/Modules/Data.lua",
            910,
            1069,
            "74247cdc849c052d47bc972dbb32cf4f6e81afd84417da21471a7ce79ccbed6a",
        ),
        "load" => (
            "src/Classes/SkillsTab.lua",
            303,
            405,
            "5e34bdfe94c3d4446ba85edbe1dc5d8c6cd5a1ab063e076a5aeedadf2555b9b6",
        ),
        "process" => (
            "src/Classes/SkillsTab.lua",
            1242,
            1322,
            "8737e8311f6cbef4b9bc10525e28831d00f52e602494a7f2bdfbccee68a33f32",
        ),
        "sanitize" => (
            "src/Modules/Common.lua",
            321,
            347,
            "ad81cfbd6579b726668f7069f9d58dab8ee166ca9243e31fd56a11fb7e3ab682",
        ),
        "copy" => (
            "src/Modules/Common.lua",
            495,
            505,
            "2e013585437512f70fd90debc4037fc3d987a91ec1ad57e144336ce558f15ae6",
        ),
        "concat" => (
            "src/Modules/Common.lua",
            625,
            634,
            "d95dabad8ddba9558f6202e172afe8101ad1937cee3e958b73be0ab1e74122e4",
        ),
        "wipe" => (
            "src/Modules/Common.lua",
            564,
            572,
            "eff97e60d9c16582d74b3f1430414f7129e6a805eb5773bbccae840b199209ad",
        ),
        "round" => (
            "src/Modules/Common.lua",
            722,
            728,
            "90585272c160e302702e5071f1ad55c649b8da02e83ef2d776d89a449250e1f2",
        ),
        "requirements" => (
            "src/Modules/CalcTools.lua",
            152,
            158,
            "ea60919f8f7438801f81cdc38f505e068269a99edb2550197a7caa624f45664f",
        ),
        "validate" => (
            "src/Modules/CalcTools.lua",
            60,
            76,
            "65f04133f20851a940a03900c9eb401db0baacabe048c97dc418bf4008d55fa7",
        ),
        "normalized" => (
            "src/Modules/CalcSetup.lua",
            667,
            676,
            "0204eac6356b36cd8f23799a373653d04332174edc5998d768ed11192fe9c3e4",
        ),
        "generated" => (
            "src/Modules/CalcSetup.lua",
            1690,
            1734,
            "0452d3bc3b3318477771421a87aaeeaaab5710c6796c71997e781d0326b20705",
        ),
        _ => panic!("unknown source span"),
    };
    let body = sources()[path]
        .lines()
        .skip(first - 1)
        .take(last - first + 1)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(body.lines().count(), last - first + 1);
    assert_eq!(hash(body.as_bytes()), digest);
    body
}

fn components(jit: bool) -> Lua {
    let lua = Lua::new();
    lua.set_memory_limit(256 * 1024 * 1024).unwrap();
    lua.load(if jit {
        "require('jit').on();bit=require('bit');data={}"
    } else {
        "require('jit').off();bit=require('bit');data={}"
    })
    .exec()
    .unwrap();
    for name in ["sanitize", "copy", "concat", "wipe", "round"] {
        lua.load(span(name)).exec().unwrap();
    }
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let observed = calls.clone();
    lua.globals()
        .set(
            "LoadModule",
            lua.create_function(move |lua, name: String| {
                let path = format!("src/{name}.lua");
                let body = sources().get(&path).ok_or_else(|| {
                    mlua::Error::RuntimeError(format!("unapproved dependency {path}"))
                })?;
                observed.lock().unwrap().push(path.clone());
                lua.load(body).set_name(format!("@{path}")).eval::<Value>()
            })
            .unwrap(),
        )
        .unwrap();
    // Same source boundary as the maintained identity extractor, including all
    // nine skill modules, lazy metadata setup and the whole pairs(data.gems)
    // construction loop. No filtered Gem subset supplies the reverse census.
    lua.load(format!(
        "{}\n{}\nconstructGem=setupGem",
        span("prefix"),
        span("assembly")
    ))
    .set_name("@authenticated-complete-skill-and-gem-construction")
    .exec()
    .unwrap();
    let skill_calls: Vec<_> = calls
        .lock()
        .unwrap()
        .iter()
        .filter(|p| p.starts_with("src/Data/Skills/") && !p.ends_with("SkillAssets.lua"))
        .cloned()
        .collect();
    assert_eq!(
        json!(skill_calls),
        catalog()["source"]["skill_module_order"]
    );
    lua.load(format!(
        "local m_min,m_max,m_floor=math.min,math.max,math.floor\nlocal t_insert=table.insert\n\
         SkillsTabClass={{}}\ncalcLib={{}}\n{}\n{}\n{}\n{}\n{}\n\
         function updateGeneratedDispatch(build,grants,mode)\n\
         local env={{mode=mode,grantedSkills=grants}}\nlocal accelerate={{}}\nlocal tempTable1={{}}\n{}\nend end\n\
         return build.skillsTab.socketGroupList end",
        span("validate"),span("requirements"),span("process"),span("load"),span("normalized"),span("generated"),
    )).set_name("@authenticated-saved-and-generated-dispatch-functions").exec().unwrap();
    lua.load(r#"
        function dispatchHost()
            local host=setmetatable({build={data=data},skillSets={{socketGroupList={}}}}, {__index=SkillsTabClass})
            host.build.skillsTab=host
            host.socketGroupList=host.skillSets[1].socketGroupList
            return host
        end
        function loadDispatch(host, attributes)
            local node={elem='Skill',attrib={enabled='true',slot='Weapon 1'},
                {elem='Gem',attrib=attributes}}
            host:LoadSkill(node,1)
            local group=host.socketGroupList[#host.socketGroupList]
            return group,group.gemList[1]
        end
        function routeAttributes(id)
            return {skillId=id,level='20',quality='0',enabled='true',enableGlobal1='true',enableGlobal2='true',count='3'}
        end
        function generatedGrant(id)
            -- Explicit acquisition result supplied by the component host.
            -- This source/slot identity is not a native provider certificate.
            return {skillId=id,source='SyntheticReviewedGrant:'..id,slotName='Weapon 1',level=20,
                nameSpec=data.skills[id].name,sourceNode={testIdentity='tree-supplier'}}
        end
        function primaryCandidates(effect)
            local keys={}
            for key,gem in pairs(data.gems) do
                if gem.grantedEffect==effect then keys[#keys+1]=key end
            end
            table.sort(keys)
            return keys
        end
    "#).set_name("@explicit-dispatch-component-host").exec().unwrap();
    lua
}

fn effect_ids(gem: &Table) -> Vec<String> {
    gem.get::<Table>("grantedEffectList")
        .unwrap()
        .sequence_values::<Table>()
        .map(|v| v.unwrap().get("id").unwrap())
        .collect()
}

#[test]
fn complete_catalog_object_census_keeps_zero_one_and_multiple_primary_candidates_distinct() {
    let mut expected = None;
    for jit in [false, true] {
        for _fresh in 0..2 {
            let lua = components(jit);
            let data: Table = lua.globals().get("data").unwrap();
            let skills: Table = data.get("skills").unwrap();
            let gems: Table = data.get("gems").unwrap();
            let reverse: Table = data.get("gemForSkill").unwrap();
            let raw: Table = lua.load(&sources()["src/Data/Gems.lua"]).eval().unwrap();
            let mut pointers = BTreeSet::new();
            let mut ids = BTreeSet::new();
            for row in skills.pairs::<String, Table>() {
                let (id, effect) = row.unwrap();
                assert_eq!(effect.get::<String>("id").unwrap(), id);
                assert!(
                    pointers.insert(effect.to_pointer() as usize),
                    "source skill objects alias: {id}"
                );
                assert!(ids.insert(id));
            }
            let expected_ids: BTreeSet<_> = catalog()["skills"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["id"].as_str().unwrap().to_owned())
                .collect();
            assert_eq!(ids, expected_ids);
            assert_eq!(ids.len(), 1436);
            let mut candidates: BTreeMap<String, Vec<String>> = BTreeMap::new();
            let mut keys = BTreeSet::new();
            for row in gems.pairs::<String, Table>() {
                let (key, gem) = row.unwrap();
                assert!(keys.insert(key.clone()));
                assert_eq!(gem.get::<String>("id").unwrap(), key);
                let id: String = gem.get("grantedEffectId").unwrap();
                let declaration: Table = raw.get(key.as_str()).unwrap();
                assert_eq!(declaration.get::<String>("grantedEffectId").unwrap(), id);
                for field in ["gameId", "variantId"] {
                    assert_eq!(
                        declaration.get::<String>(field).unwrap(),
                        gem.get::<String>(field).unwrap()
                    );
                }
                let primary: Table = gem.get("grantedEffect").unwrap();
                assert_eq!(
                    primary.to_pointer(),
                    skills.get::<Table>(id.as_str()).unwrap().to_pointer()
                );
                candidates.entry(id).or_default().push(key);
            }
            assert_eq!(keys.len(), 966);
            let expected_keys: BTreeSet<_> = catalog()["gems"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| g["key"].as_str().unwrap().to_owned())
                .collect();
            assert_eq!(keys, expected_keys);
            let mut raw_candidates: BTreeMap<String, Vec<String>> = BTreeMap::new();
            let mut raw_keys = BTreeSet::new();
            for row in raw.pairs::<String, Table>() {
                let (key, declaration) = row.unwrap();
                assert!(raw_keys.insert(key.clone()));
                raw_candidates
                    .entry(declaration.get("grantedEffectId").unwrap())
                    .or_default()
                    .push(key);
            }
            assert_eq!(raw_keys, keys);
            for values in candidates.values_mut() {
                values.sort();
            }
            for values in raw_candidates.values_mut() {
                values.sort();
            }
            assert_eq!(raw_candidates, candidates);
            for (id, values) in &candidates {
                let actual: String = reverse
                    .get(skills.get::<Table>(id.as_str()).unwrap())
                    .unwrap();
                assert!(values.contains(&actual));
                if values.len() == 1 {
                    assert_eq!(&actual, &values[0]);
                }
                // Multiple candidates remain a list; never assign authority to
                // the unordered winner even if this VM happened to choose one.
            }
            for id in PRIMARIES {
                assert_eq!(candidates[id].len(), 1);
                let gem: Table = gems.get(candidates[id][0].as_str()).unwrap();
                let expected_effects = match id {
                    "FireboltPlayer" => vec![id],
                    "SummonSandDjinnPlayer" => vec![id, COMMANDS[0]],
                    "SummonWaterDjinnPlayer" => vec![id, COMMANDS[1]],
                    _ => unreachable!(),
                };
                assert_eq!(effect_ids(&gem), expected_effects);
                if id == "FireboltPlayer" {
                    assert_eq!(
                        gem.get::<String>("additionalStatSet1").unwrap(),
                        "FireboltExplosionPlayer"
                    );
                    assert_eq!(
                        gem.get::<Table>("additionalGrantedEffects")
                            .unwrap()
                            .raw_len(),
                        0
                    );
                }
            }
            for id in COMMANDS {
                assert!(!candidates.contains_key(id));
                assert!(matches!(
                    reverse
                        .get::<Value>(skills.get::<Table>(id).unwrap())
                        .unwrap(),
                    Value::Nil
                ));
                assert!(catalog()["gems"].as_array().unwrap().iter().any(|g| {
                    g["additional_effects"]
                        .as_array()
                        .unwrap()
                        .contains(&json!(id))
                }));
            }
            let observed = json!(candidates);
            if let Some(before) = &expected {
                assert_eq!(&observed, before);
            } else {
                expected = Some(observed);
            }
        }
    }
}

#[test]
fn actual_loaders_distinguish_table_reverse_dispatch_explicit_gem_and_bare_string_fallback() {
    for jit in [false, true] {
        let lua = components(jit);
        lua.load(r#"
            for _,id in ipairs({'FireboltPlayer','SummonSandDjinnPlayer','SummonWaterDjinnPlayer'}) do
                local effect=data.skills[id]
                local candidates=primaryCandidates(effect)
                assert(#candidates==1)
                local key=candidates[1]
                local gem=data.gems[key]
                assert(data.gemForSkill[effect]==key and data.gemForSkill[id]==nil)
                local _,inferred=loadDispatch(dispatchHost(),routeAttributes(id))
                assert(inferred.gemId==key and inferred.gemData==gem and inferred.grantedEffect==nil)
                local explicit=routeAttributes('DefinitelyNotThePrimary')
                explicit.gemId=gem.gameId;explicit.variantId=gem.variantId
                local _,saved=loadDispatch(dispatchHost(),explicit)
                assert(saved.gemData==gem and saved.skillId==id)
                -- ProcessSocketGroup alone uses the STRING key at1267, unlike
                -- both preceding saved/generated TABLE-keyed lookup writers.
                local bare={skillId=id,level=20,quality=0,enabled=true}
                local host=dispatchHost()
                host:ProcessSocketGroup({gemList={bare}})
                assert(bare.gemData==nil and bare.grantedEffect==effect)
                -- Explicit internal Gem ID has its own source precedence; a
                -- missing explicit ID does not fall through to skillId.
                bare.gemId='NoSuchGem'
                host:ProcessSocketGroup({gemList={bare}})
                assert(bare.gemData==nil and bare.grantedEffect==nil)
                local generated=dispatchHost()
                local grant=generatedGrant(id)
                local groups=updateGeneratedDispatch(generated.build,{grant},'MAIN')
                assert(#groups==1 and groups[1].source==grant.source and groups[1].slot==grant.slotName)
                local row=groups[1].gemList[1]
                assert(row.gemData==gem and row.gemId==key and row.grantedEffect==nil)
                assert(groups[1].sourceNode==grant.sourceNode and row.fromItem==false)
                assert(row.level==20 and row.enableGlobal1==true)
                local beforeGroup,beforeRow=groups[1],row
                updateGeneratedDispatch(generated.build,{grant},'CALCS')
                assert(generated.socketGroupList[1]==beforeGroup and beforeGroup.gemList[1]==beforeRow)
            end
        "#).exec().unwrap();
    }
}

#[test]
fn complete_zero_candidate_commands_take_direct_route_but_deleted_map_is_not_absence_evidence() {
    for jit in [false, true] {
        let lua = components(jit);
        lua.load(r#"
            for _,id in ipairs({'CommandSandDjinnKnifeThrowPlayer','CommandWaterDjinnBubblePlayer'}) do
                local effect=data.skills[id]
                assert(#primaryCandidates(effect)==0 and data.gemForSkill[effect]==nil)
                local _,saved=loadDispatch(dispatchHost(),routeAttributes(id))
                assert(saved.gemId==nil and saved.gemData==nil and saved.grantedEffect==effect)
                local generated=dispatchHost()
                local group=updateGeneratedDispatch(generated.build,{generatedGrant(id)},'MAIN')[1]
                local row=group.gemList[1]
                assert(row.gemId==nil and row.gemData==nil and row.grantedEffect==effect)
                assert(row.skillId==id)
            end
            local id='FireboltPlayer'
            local effect=data.skills[id]
            local candidate=primaryCandidates(effect)
            assert(#candidate==1)
            data.gemForSkill[effect]=nil
            assert(#primaryCandidates(effect)==1)
            local _,row=loadDispatch(dispatchHost(),routeAttributes(id))
            assert(row.gemData==nil and row.grantedEffect==effect)
            -- Same direct shape, different evidence: this mutation cannot be
            -- promoted to a checked zero-candidate source dispatch certificate.
            data.gemForSkill[effect]=candidate[1]
        "#).exec().unwrap();
    }
}

#[test]
fn alias_constructor_orders_and_display_reordering_do_not_change_primary_candidate_meaning() {
    for jit in [false, true] {
        for alias_last in [false, true] {
            let lua = components(jit);
            lua.globals().set("aliasLast", alias_last).unwrap();
            lua.load(r#"
                local primary=data.skills.SummonSandDjinnPlayer
                local command=data.skills.CommandSandDjinnKnifeThrowPlayer
                local candidates=primaryCandidates(primary)
                assert(#candidates==1 and #primaryCandidates(command)==0)
                local key=candidates[1]
                local original=data.gems[key]
                -- Copy the RAW declaration, not the constructed object graph.
                local raw=LoadModule('Data/Gems')
                local alias=copyTable(raw[key])
                alias.gameId='SyntheticAliasGame';alias.variantId='SyntheticAliasVariant'
                alias.grantedEffectDisplayOrder={1,0}
                if aliasLast then
                    constructGem(original,key);constructGem(alias,'SyntheticAlias')
                else
                    constructGem(alias,'SyntheticAlias');constructGem(original,key)
                end
                data.gems.SyntheticAlias=alias
                assert(alias.grantedEffect==primary)
                assert(alias.grantedEffectList[1]==command and alias.grantedEffectList[2]==primary)
                assert(alias.additionalGrantedEffects[1]==command)
                local all=primaryCandidates(primary)
                assert(#all==2 and all[1]==key and all[2]=='SyntheticAlias')
                assert(#primaryCandidates(command)==0 and data.gemForSkill[command]==nil)
                local winner=aliasLast and 'SyntheticAlias' or key
                assert(data.gemForSkill[primary]==winner)
                local _,inferred=loadDispatch(dispatchHost(),routeAttributes(primary.id))
                assert(inferred.gemData==data.gems[winner])
                local generated=dispatchHost()
                local row=updateGeneratedDispatch(generated.build,{generatedGrant(primary.id)},'MAIN')[1].gemList[1]
                assert(row.gemData==data.gems[winner])
                local attrs=routeAttributes(primary.id)
                attrs.gemId=original.gameId;attrs.variantId=original.variantId
                local _,explicit=loadDispatch(dispatchHost(),attrs)
                assert(explicit.gemData==original)
                -- Cardinality stays two regardless of winner/order. Neither
                -- ordered source choice may be serialized as unique authority.
            "#).exec().unwrap();
        }
    }
}
