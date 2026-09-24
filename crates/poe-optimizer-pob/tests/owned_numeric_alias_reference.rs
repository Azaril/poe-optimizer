//! Optional complete-source proof for one reviewed lexical alias. Missing and
//! malformed input policy remains independent from Lua's numeric fallback.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_schema::{DefinitionAddress, SchemaSubject},
};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, MappingPackageInput,
        SourceComponent,
    },
    owned_normalize::NormalizationPolicy,
    owned_source::SourceAttributeRef,
    owned_value::OwnedValueCodec,
    owned_value_policy::{
        CandidateValue, MissingValuePolicy, NumericTokenAlias, ValueCandidate, ValueLane,
        ValueOutcome, ValuePendingReason, ValueRecipe, ValueRecipeInput,
    },
};
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::Value as Json;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn complete_original_loading_matches_reviewed_numeric_alias_in_both_jit_modes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/owned-numeric-alias-reference-01");
    fs::create_dir_all(&destination).unwrap();
    if let Some(mode) = std::env::var_os("POE_NUMERIC_ALIAS_SOURCE_CHILD") {
        assert!(mode == "on" || mode == "off");
        let jit_enabled = mode == "on";
        let recipes = reviewed_recipes(&root);
        let keys: Vec<_> = recipes.keys().cloned().collect();
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-02.xml"))
                .unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("numericAliasJitEnabled", jit_enabled)?;
            lua.globals()
                .set("reviewedNumericAliasGemKeys", lua.to_value(&keys)?)?;
            lua.load("if numericAliasJitEnabled then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let result = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &xml,
            None,
            false,
            Some(&before),
            None,
            Some(&observe),
        )
        .unwrap();
        compare_native(&recipes, &result["additional_observation"]);
        fs::write(
            destination.join(if jit_enabled {
                "jit-on.json"
            } else {
                "jit-off.json"
            }),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        return;
    }
    for mode in ["off", "on"] {
        let log_path = destination.join(format!("jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "complete_original_loading_matches_reviewed_numeric_alias_in_both_jit_modes",
                "--nocapture",
            ])
            .env("POE_NUMERIC_ALIAS_SOURCE_CHILD", mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "source child failed: {}",
                    log_path.display()
                );
                break;
            }
            if started.elapsed() > Duration::from_secs(180) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source child deadline: {}", log_path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let off: Json =
        serde_json::from_slice(&fs::read(destination.join("jit-off.json")).unwrap()).unwrap();
    let on: Json =
        serde_json::from_slice(&fs::read(destination.join("jit-on.json")).unwrap()).unwrap();
    assert_eq!(off["source_hash"], on["source_hash"]);
    assert_eq!(off["additional_observation"], on["additional_observation"]);
}

fn read<T: serde::de::DeserializeOwned>(root: &Path, relative: &str) -> T {
    serde_json::from_slice(&fs::read(root.join(relative)).unwrap()).unwrap()
}
fn reviewed_recipes(root: &Path) -> BTreeMap<String, ValueRecipeInput> {
    let policy: Json = read(
        root,
        "data/owned/poe2/3887ae68/support-gem-inputs/policy.json",
    );
    let identities = SkillIdentityCatalog::new(read(
        root,
        "data/owned/poe2/3887ae68/import/skill-identities.json",
    ))
    .unwrap();
    assert_eq!(
        policy["catalog_digest"],
        serde_json::to_value(
            digest_owned(
                "owned-skill-source-catalog-v1",
                identities.data(),
                64 * 1024 * 1024
            )
            .unwrap()
        )
        .unwrap()
    );
    let skills: BTreeMap<_, _> = identities
        .data()
        .skills
        .iter()
        .map(|row| (row.id.as_str(), row))
        .collect();
    let gems: BTreeMap<_, _> = identities
        .data()
        .gems
        .iter()
        .filter(|gem| {
            let effect = skills[gem.primary_effect_id.as_str()];
            effect.support == Some(true)
                && effect.from_tree != Some(true)
                && gem.declared_additional_effects.is_empty()
                && gem.constructed_additional_effects.is_empty()
                && gem.additional_effects.is_empty()
                && gem.declared_additional_stat_sets.is_empty()
                && gem.effect_list == [gem.primary_effect_id.clone()]
        })
        .map(|gem| (gem.key.clone(), gem))
        .collect();
    let selected: Vec<String> = serde_json::from_value(policy["source_gems"].clone()).unwrap();
    assert_eq!(selected, gems.keys().cloned().collect::<Vec<_>>());
    assert_eq!(selected.len(), 514);
    let mapping: MappingPackageInput = read(root, "data/owned/poe2/3887ae68/current/mapping.json");
    let mappings: BTreeMap<_, _> = mapping
        .entries
        .iter()
        .map(|row| (&row.source, &row.outcome))
        .collect();
    let normalization: NormalizationPolicy = read(
        root,
        "data/owned/poe2/3887ae68/support-gem-inputs/numeric-alias-normalization.json",
    );
    let gem_inputs = normalization.gem_inputs.unwrap();
    assert_eq!(gem_inputs.gems.len(), 516);
    let by_id: BTreeMap<_, _> = gem_inputs.gems.iter().map(|row| (&row.gem, row)).collect();
    assert_eq!(by_id.len(), gem_inputs.gems.len());
    let mut recipes = BTreeMap::new();
    for (key, source) in gems {
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(source.game_id.clone()),
            variant_id: SourceComponent::Text(source.variant_id.clone()),
        });
        let MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(id)),
            basis: MappingBasis::Exact,
        } = mappings[&selector]
        else {
            panic!("exact physical Gem mapping: {key}")
        };
        let rule = by_id[id];
        assert!(rule.guards.is_empty());
        let values: Vec<_> = rule
            .parameters
            .iter()
            .filter(|parameter| {
                parameter.value.tiers.iter().any(|tier| {
                    tier.selectors
                        .iter()
                        .any(|selector| selector.name == "corruptLevel")
                })
            })
            .collect();
        assert_eq!(values.len(), 1);
        let recipe = &values[0].value;
        assert!(matches!(recipe.missing, MissingValuePolicy::Pending));
        assert_eq!(recipe.tiers.len(), 1);
        assert_eq!(recipe.tiers[0].selectors.len(), 1);
        assert_eq!(recipe.tiers[0].selectors[0].lane, ValueLane::Attribute);
        assert_eq!(
            recipe.numeric_aliases,
            vec![NumericTokenAlias {
                token: "nil".into(),
                replacement: "0".into()
            }]
        );
        assert!(
            OwnedValueCodec::new(recipe.codec.clone(), Default::default())
                .unwrap()
                .decode("nil")
                .is_err()
        );
        assert!(recipes.insert(key, recipe.clone()).is_none());
    }
    assert_eq!(recipes.len(), 514);
    recipes
}

fn compare_native(recipes: &BTreeMap<String, ValueRecipeInput>, observation: &Json) {
    let aliases = observation["aliases"].as_array().unwrap();
    assert_eq!(aliases.len(), 514);
    assert_eq!(
        aliases
            .iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        recipes.keys().map(String::as_str).collect::<Vec<_>>()
    );
    for row in aliases {
        let input = &recipes[row["id"].as_str().unwrap()];
        assert_eq!(row["input"], "nil");
        assert_selected(input, "nil", row);
    }
    let contrasts = observation["contrasts"].as_array().unwrap();
    assert_eq!(contrasts.len(), 6);
    let input = recipes.first_key_value().unwrap().1;
    let recipe = ValueRecipe::new(input.clone(), Default::default()).unwrap();
    assert!(matches!(
        recipe.decide(&[]).unwrap().outcome,
        ValueOutcome::Pending {
            reason: ValuePendingReason::Missing
        }
    ));
    for row in contrasts {
        match row["label"].as_str().unwrap() {
            "negative" | "fractional" | "zero" => {
                assert_selected(input, row["input"].as_str().unwrap(), row)
            }
            "absent" => {
                assert_eq!(row["before"], 0);
                assert_eq!(row["after"], 0);
            }
            "invalid" | "uppercase" => {
                let text = row["input"].as_str().unwrap();
                let origin = origin(text);
                let decision = recipe
                    .decide(&[ValueCandidate {
                        selector: &input.tiers[0].selectors[0],
                        origin,
                        value: CandidateValue::Decoded(text),
                    }])
                    .unwrap();
                assert!(matches!(
                    decision.outcome,
                    ValueOutcome::Pending {
                        reason: ValuePendingReason::Decode(_)
                    }
                ));
                assert_eq!(row["before"], 0);
                assert_eq!(row["after"], 0);
            }
            unexpected => panic!("unexpected contrast {unexpected}"),
        }
    }
}
fn origin(text: &str) -> SourceAttributeRef {
    let xml = format!("<PathOfBuilding2><Gem corruptLevel=\"{text}\"/></PathOfBuilding2>");
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([0x79; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    SourceAttributeRef {
        occurrence: source
            .occurrences()
            .iter()
            .find(|row| row.name() == "Gem")
            .unwrap()
            .id(),
        index: 0,
    }
}
fn assert_selected(input: &ValueRecipeInput, text: &str, source: &Json) {
    let recipe = ValueRecipe::new(input.clone(), Default::default()).unwrap();
    let origin = origin(text);
    let result = recipe
        .decide(&[ValueCandidate {
            selector: &input.tiers[0].selectors[0],
            origin,
            value: CandidateValue::Decoded(text),
        }])
        .unwrap();
    let ValueOutcome::Selected {
        origin: selected,
        value: ParameterValue::Quantity(value),
    } = result.outcome
    else {
        panic!("expected exact selected numeric input")
    };
    assert_eq!(selected, origin);
    assert_eq!(result.matched.len(), 1);
    assert_eq!(result.matched[0].origin, origin);
    assert_eq!(source["before"].as_f64(), Some(value.value()));
    assert_eq!(source["after"].as_f64(), Some(value.value()));
    assert_eq!(source["before_type"], "number");
    assert_eq!(source["after_type"], "number");
    let ParameterValue::Quantity(zero) =
        OwnedValueCodec::new(input.codec.clone(), Default::default())
            .unwrap()
            .decode("0")
            .unwrap()
    else {
        panic!()
    };
    assert_eq!(value.unit(), zero.unit());
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVATION)
        .set_name("@owned-numeric-alias-source-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
const OBSERVATION: &str = r#"
local function original(f,path,line)
 local info=debug.getinfo(f,"S")
 local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==line)
 return f
end
local loadSkill=original(build.skillsTab.LoadSkill,"Classes/SkillsTab.lua",303)
local process=original(build.skillsTab.ProcessSocketGroup,"Classes/SkillsTab.lua",1242)
local function load(id,delta)
 local gem=data.gems[id]
 assert(gem and gem.grantedEffect.support and not gem.grantedEffect.fromTree)
 assert(#gem.grantedEffectList==1 and gem.grantedEffectList[1]==gem.grantedEffect)
 for key in pairs(gem) do
  assert(not key:match("^additionalGrantedEffectId%d+$") and not key:match("^additionalStatSet%d+$"))
 end
 local tab=setmetatable({build=build,skillSets={[1]={socketGroupList={}}}},{__index=build.skillsTab})
 local row={id=id,input=delta}
 -- Scoped observer invokes the complete original processing method unchanged.
 function tab:ProcessSocketGroup(group)
  row.before=group.gemList[1].corruptLevel;row.before_type=type(row.before)
  local result=process(self,group)
  row.after=group.gemList[1].corruptLevel;row.after_type=type(row.after)
  return result
 end
 local attributes={gemId=gem.gameId,variantId=gem.variantId,level="1",quality="20",corrupted="false",corruptLevel=delta}
 loadSkill(tab,{elem="Skill",attrib={enabled="true"},{elem="Gem",attrib=attributes}},1)
 local group=tab.skillSets[1].socketGroupList[1]
 assert(group and #group.gemList==1 and group.gemList[1].gemData==gem)
 assert(group.gemList[1].gemId==id and group.gemList[1].level==1 and group.gemList[1].quality==20)
 assert(group.gemList[1].corrupted==false and attributes.corruptLevel==delta)
 assert(build.skillsTab.LoadSkill==loadSkill and build.skillsTab.ProcessSocketGroup==process)
 return row
end
local result={aliases={},contrasts={}}
assert(#reviewedNumericAliasGemKeys==514)
for _,id in ipairs(reviewedNumericAliasGemKeys) do
 local row=load(id,"nil")
 assert(row.before==0 and row.after==0 and row.before_type=="number" and row.after_type=="number")
 result.aliases[#result.aliases+1]=row
end
for _,case in ipairs({{label="absent",expected=0},{label="invalid",value="bad",expected=0},
 {label="uppercase",value="NIL",expected=0},{label="negative",value="-1",expected=-1},
 {label="fractional",value="0.5",expected=0.5},{label="zero",value="0",expected=0}}) do
 local row=load(reviewedNumericAliasGemKeys[1],case.value);row.label=case.label
 assert(row.before==case.expected and row.after==case.expected)
 result.contrasts[#result.contrasts+1]=row
end
return result
"#;
