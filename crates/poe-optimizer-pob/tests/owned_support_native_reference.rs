//! Native preparation against complete authenticated upstream functions.
//! These component witnesses do not establish final delivery or whole-build parity.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, Lua, Table, Value};
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*,
    owned_supports::*,
};
use poe_optimizer_data::{
    owned_rules::OwnedRulePackage,
    owned_schema::{
        OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, SchemaPackageInput,
    },
    owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::owned_supports::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[path = "support/item_loading_runtime.rs"]
#[allow(dead_code)]
mod runtime;

fn oracle(warm: bool) -> runtime::Oracle {
    let oracle = runtime::Oracle::new();
    oracle.lua.globals().set("nativeWarm", warm).unwrap();
    oracle
        .lua
        .load(
            r#"
local originalRequire=require
nativeSupportCalcs=LoadModule("Modules/CalcBase")
require=function(name)
 if name=="Modules.CalcBase" then return nativeSupportCalcs end
 return originalRequire(name)
end
LoadModule("Modules/CalcSetup")
LoadModule("Modules/CalcActiveSkill")
require=originalRequire
function nativeInstance(id,level,quality)
 local effect=assert(data.skills[id],id)
 local gem=data.gemForSkill[effect]
 return {grantedEffect=effect,gemData=gem and data.gems[gem],
  level=level or 1,quality=quality or 0,actorLevel=90,
  statSet={index=1},statSetCalcs={index=1},isSupporting={}}
end
-- Synthetic data replaces only explicitly authored top-level facts. Complete
-- original functions remain unchanged; real source records retain their identity.
function nativeSynthetic(id,requires,adds,excludes)
 local instance=nativeInstance("SupportElementalArmamentPlayerTwo")
 local original=instance.grantedEffect;local definition={}
 for k,v in pairs(original) do definition[k]=v end
 definition.id=id;definition.gemFamily=nil;definition.plusVersionOf=nil
 definition.requireSkillTypes=requires or {};definition.excludeSkillTypes=excludes or {}
 definition.addSkillTypes=adds or {};definition.ignoreMinionTypes=false
 instance.grantedEffect=definition;instance.gemData=nil
 return instance
end
function nativeObserve(active,origins,summon)
 local selected={}
 for index,origin in ipairs(origins) do
  origin.nativeOrigin=index
  nativeSelectionBest(origin,selected,"MAIN")
 end
 local actor={enemy={}};actor.enemy.player=actor
 local prepared=nativeSupportCalcs.createActiveSkill(active,selected,{mode="MAIN"},actor,nil,summon)
 return {active=active,origins=origins,selected=selected,prepared=prepared,summon=summon}
end
function nativeSummoner(id)
 local actor={enemy={}};actor.enemy.player=actor
 return nativeSupportCalcs.createActiveSkill(nativeInstance(id),{}, {mode="MAIN"},actor)
end
if nativeWarm then jit.on() else jit.off();jit.flush() end
"#,
        )
        .set_name("@owned-native-support-observation-inputs")
        .exec()
        .unwrap();
    let init: Function = oracle
        .lua
        .globals()
        .get::<Table>("nativeSupportCalcs")
        .unwrap()
        .get("initEnv")
        .unwrap();
    for (name, global, line) in [
        ("addBestSupport", "nativeSelectionBest", 586),
        ("processGrantedEffect", "nativeProcessSupport", 630),
    ] {
        let function = original_upvalue(&oracle.lua, &init, name);
        assert_eq!(
            function.info().source.as_deref(),
            Some("@src/Modules/CalcSetup.lua")
        );
        assert_eq!(function.info().line_defined, Some(line));
        oracle.lua.globals().set(global, function).unwrap();
    }
    oracle
}

fn original_upvalue(lua: &Lua, function: &Function, requested: &str) -> Function {
    let mut found = None;
    for index in 1..=i32::from(function.info().num_upvalues) {
        // SAFETY: inspect an existing upvalue on a rooted Function; copy its
        // name/value without mutating the source closure or opening debug APIs.
        let (name, value): (String, Value) = unsafe {
            lua.exec_raw(function.clone(), |state| {
                let name = mlua::ffi::lua_getupvalue(state, 1, index);
                mlua::ffi::lua_pushstring(state, name);
                mlua::ffi::lua_insert(state, -2);
                mlua::ffi::lua_remove(state, 1);
            })
        }
        .unwrap();
        if name == requested {
            assert!(found.is_none());
            found = Some(value.as_function().unwrap().clone());
        }
    }
    found.expect("authenticated complete original closure")
}

fn key(s: impl Into<String>) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("support-reference", "v1").unwrap()
}
fn definition<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(namespace(), s).unwrap()
}
fn occurrence<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([94; 16]), n).unwrap())
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn flag(table: &Table, name: &str) -> bool {
    table.get::<Option<bool>>(name).unwrap().unwrap_or(false)
}
fn type_key(value: i64) -> OwnedDefinitionKey {
    key(format!("type-{value}"))
}
fn types(table: Option<Table>) -> Vec<OwnedDefinitionKey> {
    table
        .into_iter()
        .flat_map(|t| {
            t.pairs::<i64, bool>()
                .map(|p| {
                    let (id, present) = p.unwrap();
                    assert!(present);
                    type_key(id)
                })
                .collect::<Vec<_>>()
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn names(table: Option<Table>) -> Option<Vec<String>> {
    table.map(|t| t.sequence_values::<String>().map(Result::unwrap).collect())
}

/// Test-only source adapter. The oracle independently evaluates the untouched
/// postfix expression, including the OR of any residual stack values.
fn predicate(
    lua: &Lua,
    input: Table,
    vocabulary: &mut BTreeSet<OwnedDefinitionKey>,
) -> Option<SupportTypePredicate> {
    let operators: Table = lua.globals().get("SkillType").unwrap();
    let or: i64 = operators.get("OR").unwrap();
    let and: i64 = operators.get("AND").unwrap();
    let not: i64 = operators.get("NOT").unwrap();
    let mut stack = Vec::new();
    for value in input.sequence_values::<i64>() {
        let value = value.unwrap();
        if value == not {
            let last = stack.pop().expect("valid source NOT");
            stack.push(SupportTypePredicate::Not(Box::new(last)));
        } else if value == or || value == and {
            let right = stack.pop().expect("valid source binary right");
            let left = stack.pop().expect("valid source binary left");
            stack.push(if value == or {
                SupportTypePredicate::Any(vec![left, right])
            } else {
                SupportTypePredicate::All(vec![left, right])
            });
        } else {
            let symbol = type_key(value);
            vocabulary.insert(symbol.clone());
            stack.push(SupportTypePredicate::Type(symbol));
        }
    }
    if stack.is_empty() {
        None
    } else {
        Some(SupportTypePredicate::Any(stack))
    }
}

fn compare(lua: &Lua, source: Table) {
    let originals: Vec<Table> = source
        .get::<Table>("origins")
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect();
    let mut effect_names = BTreeSet::new();
    let mut effect_tables = BTreeMap::new();
    let mut family_names = BTreeSet::new();
    for original in &originals {
        let effect: Table = original.get("grantedEffect").unwrap();
        let name = effect.get::<String>("id").unwrap();
        if let Some(previous) = effect_tables.insert(name.clone(), effect.to_pointer()) {
            assert_eq!(
                previous,
                effect.to_pointer(),
                "test correspondence must preserve exact granted-effect identity"
            );
        }
        effect_names.insert(name);
        effect_names.extend(effect.get::<Option<String>>("plusVersionOf").unwrap());
        family_names.extend(
            names(effect.get("gemFamily").unwrap())
                .into_iter()
                .flatten(),
        );
    }
    let effect_symbols: BTreeMap<_, _> = effect_names
        .into_iter()
        .enumerate()
        .map(|(i, s)| (s, key(format!("effect-{i}"))))
        .collect();
    let family_symbols: BTreeMap<_, _> = family_names
        .into_iter()
        .enumerate()
        .map(|(i, s)| (s, key(format!("family-{i}"))))
        .collect();
    let mut vocabulary = BTreeSet::new();
    let mut supports = BTreeMap::new();
    let mut origins = Vec::new();
    for (index, original) in originals.iter().enumerate() {
        let effect: Table = original.get("grantedEffect").unwrap();
        let effect_name: String = effect.get("id").unwrap();
        let symbol = effect_symbols[&effect_name].clone();
        let gem = definition::<GemDefinition>(symbol.as_str());
        let added_types: Vec<_> = effect
            .get::<Table>("addSkillTypes")
            .unwrap()
            .sequence_values::<i64>()
            .map(|v| type_key(v.unwrap()))
            .collect();
        vocabulary.extend(added_types.iter().cloned());
        let preparation = SupportPreparationDefinition {
            effect: symbol,
            families: names(effect.get("gemFamily").unwrap()).map(|names| {
                names
                    .into_iter()
                    .map(|n| family_symbols[&n].clone())
                    .collect()
            }),
            plus_version_of: effect
                .get::<Option<String>>("plusVersionOf")
                .unwrap()
                .map(|n| effect_symbols[&n].clone()),
            requires: predicate(
                lua,
                effect.get("requireSkillTypes").unwrap(),
                &mut vocabulary,
            ),
            excludes: predicate(
                lua,
                effect.get("excludeSkillTypes").unwrap(),
                &mut vocabulary,
            ),
            added_types,
            gems_only: flag(&effect, "supportGemsOnly"),
            from_item: flag(&effect, "fromItem"),
            is_support: flag(&effect, "support"),
            is_trigger: flag(&effect, "isTrigger"),
            ignore_minion_types: flag(&effect, "ignoreMinionTypes"),
        };
        if let Some(prior) = supports.insert(gem.clone(), preparation.clone()) {
            assert_eq!(prior, preparation);
        }
        origins.push(ResolvedSupportOrigin {
            assignment: occurrence(index as u64 + 1),
            gem,
            enabled: Some(true),
            effective_level: Some(BoundedInteger::new(original.get("level").unwrap()).unwrap()),
            effective_quality: Some(
                FiniteQuantity::new(original.get("quality").unwrap(), definition("quality"))
                    .unwrap(),
            ),
        });
    }
    let active: Table = source.get("active").unwrap();
    let active_definition: Table = active.get("grantedEffect").unwrap();
    let context = |own: Option<Table>, minion: Option<Table>| SupportTypeContext {
        skill_types: DeclaredSet::complete(types(own)),
        minion_types: minion.map(|m| DeclaredSet::complete(types(Some(m)))),
    };
    let target = SupportPreparationTarget {
        target: SkillTarget::Authored(occurrence(1000)),
        enabled: Some(true),
        types: context(
            active_definition.get("skillTypes").unwrap(),
            active_definition.get("minionSkillTypes").unwrap(),
        ),
        summoner: source.get::<Option<Table>>("summon").unwrap().map(|s| {
            context(
                s.get("skillTypes").unwrap(),
                s.get("minionSkillTypes").unwrap(),
            )
        }),
        cannot_be_supported: Some(flag(&active_definition, "cannotBeSupported")),
        has_gem: Some(active.get::<Option<Table>>("gemData").unwrap().is_some()),
        from_item: Some(
            flag(&active_definition, "fromItem")
                || active_definition
                    .get::<Option<String>>("modSource")
                    .unwrap()
                    .is_some_and(|s| s.starts_with("Item"))
                || active
                    .get::<Option<Table>>("srcInstance")
                    .unwrap()
                    .is_some_and(|s| flag(&s, "fromItem")),
        ),
        is_player_actor: Some(true),
    };
    vocabulary.extend(target.types.skill_types.members.iter().cloned());
    vocabulary.extend(
        target
            .types
            .minion_types
            .iter()
            .flat_map(|m| m.members.iter().cloned()),
    );
    if let Some(summoner) = &target.summoner {
        vocabulary.extend(summoner.skill_types.members.iter().cloned());
        vocabulary.extend(
            summoner
                .minion_types
                .iter()
                .flat_map(|m| m.members.iter().cloned()),
        );
    }
    let package = package(
        supports,
        vocabulary,
        effect_symbols.into_values().collect(),
        family_symbols.into_values().collect(),
    );
    let SupportPreparationOutcome::Known(prepared) =
        prepare_supports(&package, &origins, &target, Default::default()).unwrap()
    else {
        panic!("complete source component inputs must resolve")
    };
    let selected: Vec<usize> = source
        .get::<Table>("selected")
        .unwrap()
        .sequence_values::<Table>()
        .map(|v| v.unwrap().get::<usize>("nativeOrigin").unwrap() - 1)
        .collect();
    assert_eq!(
        prepared
            .selected
            .iter()
            .map(|v| v.origin_index)
            .collect::<Vec<_>>(),
        selected,
        "ordered source selection"
    );
    let observed: Table = source.get("prepared").unwrap();
    let admitted: Vec<usize> = observed
        .get::<Table>("effectList")
        .unwrap()
        .sequence_values::<Table>()
        .skip(1)
        .map(|v| v.unwrap().get::<usize>("nativeOrigin").unwrap() - 1)
        .collect();
    assert_eq!(
        prepared
            .selected
            .iter()
            .filter(|p| p.applicable)
            .map(|p| p.origin_index)
            .collect::<Vec<_>>(),
        admitted,
        "final source applicability and retained multiplicity"
    );
    assert!(prepared.final_types_complete);
    assert_eq!(
        prepared.final_types,
        types(observed.get("skillTypes").unwrap()),
        "prepared source types"
    );
}

fn package(
    supports: BTreeMap<GemDefId, SupportPreparationDefinition>,
    types: BTreeSet<OwnedDefinitionKey>,
    effects: Vec<OwnedDefinitionKey>,
    families: Vec<OwnedDefinitionKey>,
) -> OwnedSupportPreparation {
    let mut definitions = vec![DefinitionDescriptor::Unit(known(
        definition("quality"),
        UnitSchema {
            dimension: UnitDimension::PercentagePoints,
        },
    ))];
    definitions.extend(supports.keys().map(|gem| {
        DefinitionDescriptor::Gem(known(
            gem.clone(),
            GemSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                roles: vec![AuthoredGemRole::SupportAssignment],
                skills: empty(),
                quality: QualityUseSchema {
                    presence: QualityPresence::Forbidden,
                    allowed_kinds: empty(),
                },
                declarations: declarations(),
            },
        ))
    }));
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("test"),
            semantics_version: key("test"),
            definitions,
            slots: vec![],
        },
        Default::default(),
    )
    .unwrap();
    let rules = OwnedRulePackage::new(
        RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("test"),
            semantics_version: key("test"),
            operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: vec![],
            receivers: empty(),
        },
        &schema,
        Default::default(),
    )
    .unwrap();
    OwnedSupportPreparation::new(
        SupportPreparationInput {
            schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
            namespace: namespace(),
            release: key("test"),
            definitions: schema.identity().clone(),
            rules: *rules.identity(),
            policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
            quality_unit: definition("quality"),
            types: types.into_iter().collect(),
            effects,
            families,
            supports: supports
                .into_iter()
                .map(|(gem, preparation)| SupportPreparationEntry {
                    gem,
                    preparation: SchemaState::Known(preparation),
                })
                .collect(),
        },
        &schema,
        &rules,
        Default::default(),
    )
    .unwrap()
}

fn run(cases: &[&str]) {
    for warm in [false, true] {
        let oracle = oracle(warm);
        for (index, case) in cases.iter().enumerate() {
            let source = oracle
                .lua
                .load(*case)
                .set_name(format!("@owned-native-support-case-{index}"))
                .eval()
                .unwrap();
            compare(&oracle.lua, source);
        }
    }
}

#[test]
fn real_definitions_match_same_effect_priority_family_replacement_and_multiplicity() {
    run(&[
        r#"return nativeObserve(nativeInstance("TwisterPlayer"),{
 nativeInstance("SupportElementalArmamentPlayerTwo",1,10),
 nativeInstance("SupportElementalArmamentPlayerTwo",1,10),
 nativeInstance("SupportElementalArmamentPlayerTwo",1,20),
 nativeInstance("SupportElementalArmamentPlayerTwo",2,0),
 nativeInstance("SupportElementalArmamentPlayerTwo",1,100)})"#,
        r#"return nativeObserve(nativeInstance("TwisterPlayer"),{
 nativeInstance("SupportElementalArmamentPlayerTwo",2,100),
 nativeInstance("SupportElementalArmamentPlayer",1,0)})"#,
        // Real definitions, synthetic combination. Last same-effect encounter
        // replaces only the first retained Salvo position after the family merge.
        r#"return nativeObserve(nativeInstance("TwisterPlayer"),{
 nativeInstance("SupportMultishotPlayer"),nativeInstance("SupportUnleashPlayer"),
 nativeInstance("SupportSalvoPlayer"),nativeInstance("SupportSalvoPlayer",2,0)})"#,
    ]);
}

#[test]
fn synthetic_plus_version_and_empty_family_presence_match_branch_precedence() {
    run(&[
        r#"local base=nativeSynthetic("test-base");local plus=nativeSynthetic("test-plus")
 plus.grantedEffect.plusVersionOf="test-base"
 return nativeObserve(nativeInstance("TwisterPlayer"),{base,plus})"#,
        r#"local base=nativeSynthetic("test-base");local plus=nativeSynthetic("test-plus")
 plus.grantedEffect.plusVersionOf="test-base"
 return nativeObserve(nativeInstance("TwisterPlayer"),{plus,base})"#,
        r#"local base=nativeSynthetic("test-base");local plus=nativeSynthetic("test-plus")
 plus.grantedEffect.plusVersionOf="test-base"
 base.grantedEffect.gemFamily={};plus.grantedEffect.gemFamily={}
 return nativeObserve(nativeInstance("TwisterPlayer"),{base,plus})"#,
        r#"local base=nativeSynthetic("test-base");local plus=nativeSynthetic("test-plus")
 plus.grantedEffect.plusVersionOf="test-base"
 base.grantedEffect.gemFamily={"base"};plus.grantedEffect.gemFamily={"plus"}
 return nativeObserve(nativeInstance("TwisterPlayer"),{base,plus})"#,
    ]);
}

#[test]
fn real_type_additions_final_exclusions_and_summoner_scope_match_source() {
    run(&[
        r#"return nativeObserve(nativeInstance("FireboltPlayer"),{
 nativeInstance("ProlongedDurationSupportPlayer"),nativeInstance("SupportArcaneSurgePlayer")})"#,
        r#"return nativeObserve(nativeInstance("WolfPackPlayer"),{
 nativeInstance("SupportFeedingFrenzyPlayer"),nativeInstance("SupportBrutusBrainPlayer")})"#,
        r#"return nativeObserve(nativeInstance("MeleeAtAnimationSpeed"),{
 nativeInstance("SupportElementalArmamentPlayerTwo"),nativeInstance("SupportMeatShieldPlayerTwo"),
 nativeInstance("SupportFeedingFrenzyPlayer")},nativeSummoner("WolfPackPlayer"))"#,
        r#"local support=nativeSynthetic("test-summoner-scope",{SkillType.Attack},{},{SkillType.Attack})
 return nativeObserve(nativeInstance("MeleeAtAnimationSpeed"),{support},nativeSummoner("WolfPackPlayer"))"#,
        // Synthetic child facts isolate nil-versus-empty summoner fallback.
        r#"local child=nativeInstance("MeleeAtAnimationSpeed")
 local original=child.grantedEffect;child.grantedEffect={}
 for k,v in pairs(original) do child.grantedEffect[k]=v end
 child.grantedEffect.minionSkillTypes={[SkillType.ConsumesRage]=true}
 local summon=nativeSummoner("WolfPackPlayer");summon.minionSkillTypes=nil
 return nativeObserve(child,{nativeSynthetic("fallback",{SkillType.ConsumesRage},{})},summon)"#,
        r#"local child=nativeInstance("MeleeAtAnimationSpeed")
 local original=child.grantedEffect;child.grantedEffect={}
 for k,v in pairs(original) do child.grantedEffect[k]=v end
 child.grantedEffect.minionSkillTypes={[SkillType.ConsumesRage]=true}
 local summon=nativeSummoner("WolfPackPlayer");summon.minionSkillTypes={}
 return nativeObserve(child,{nativeSynthetic("no-fallback",{SkillType.ConsumesRage},{})},summon)"#,
    ]);
}

#[test]
fn synthetic_sparse_retry_retained_additions_and_residual_expression_match_source() {
    run(&[
        r#"return nativeObserve(nativeInstance("FireboltPlayer"),{
 nativeSynthetic("first",{SkillType.Duration},{SkillType.GeneratesRemnants}),
 nativeSynthetic("delayed",{SkillType.CreatesFissure},{SkillType.ConsumesRage}),
 nativeSynthetic("producer",{SkillType.Duration},{SkillType.CreatesFissure}),
 nativeSynthetic("consumer",{SkillType.ConsumesRage},{SkillType.Area}),
 nativeSynthetic("seed",{SkillType.Spell},{SkillType.Duration})})"#,
        r#"return nativeObserve(nativeInstance("FireboltPlayer"),{
 nativeSynthetic("producer",{SkillType.Duration},{SkillType.CreatesFissure}),
 nativeSynthetic("delayed",{SkillType.CreatesFissure},{SkillType.ConsumesRage}),
 nativeSynthetic("first",{SkillType.Duration},{SkillType.GeneratesRemnants}),
 nativeSynthetic("consumer",{SkillType.ConsumesRage},{SkillType.Area}),
 nativeSynthetic("seed",{SkillType.Spell},{SkillType.Duration})})"#,
        // The first support contributes a new type before Duration excludes it.
        r#"local active=nativeInstance("FireboltPlayer")
 assert(not active.grantedEffect.skillTypes[SkillType.ConsumesRage])
 return nativeObserve(active,{
 nativeSynthetic("former",{SkillType.Spell},{SkillType.ConsumesRage},{SkillType.Duration}),
 nativeSynthetic("invalidator",{SkillType.Spell},{SkillType.Duration})})"#,
        r#"return nativeObserve(nativeInstance("FireboltPlayer"),{
 nativeSynthetic("residual",{SkillType.Attack,SkillType.Spell},{SkillType.Area}),
 nativeSynthetic("compound",{SkillType.Attack,SkillType.NOT,SkillType.Spell,SkillType.AND},{})})"#,
    ]);
}

#[test]
fn saved_twister_cleric_and_sniper_groups_match_controlled_preparation() {
    let directory = runtime::repository().join("tests/fixtures/builds/breadth-20260908");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json")).unwrap()).unwrap();
    for warm in [false, true] {
        let oracle = oracle(warm);
        for (number, target, expected_count) in [
            (1, "SummonSkeletalClericsPlayer", 4),
            (2, "TwisterPlayer", 5),
            (5, "SummonSkeletalSnipersPlayer", 0),
        ] {
            let xml =
                std::fs::read_to_string(directory.join(format!("build-{number:02}.xml"))).unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(xml.as_bytes())),
                manifest["builds"][number - 1]["xml_sha256"]
                    .as_str()
                    .unwrap()
            );
            oracle.lua.globals().set("nativeOriginalXml", xml).unwrap();
            oracle
                .lua
                .globals()
                .set("nativeOriginalTarget", target)
                .unwrap();
            oracle
                .lua
                .globals()
                .set("nativeExpectedCount", expected_count)
                .unwrap();
            let observed = oracle.lua.load(r#"
local roots,err=originalXml.ParseXML(nativeOriginalXml);assert(roots,err)
local skills
for _,child in ipairs(roots[1]) do if child.elem=="Skills" then skills=child end end
assert(skills)
local selectedSet
for _,child in ipairs(skills) do
 if child.elem=="SkillSet" and child.attrib.id==skills.attrib.activeSkillSet then selectedSet=child end
end
assert(selectedSet)
local group
for _,candidate in ipairs(selectedSet) do
 if candidate.elem=="Skill" and candidate[1] and candidate[1].attrib.skillId==nativeOriginalTarget then
  assert(not group);group=candidate
 end
end
assert(group and group.attrib.enabled=="true" and not group.attrib.source and not group.attrib.slot)
-- Complete original construction receives explicit empty surroundings. These
-- controlled effective support values are not full-build effective-level facts.
local env={mode="MAIN",modDB=new("ModList"):ModList(),player={itemList={},modDB=new("ModList"):ModList()}}
local cfg,processed,origins={},{},{}
for index,node in ipairs(group) do
 assert(node.elem=="Gem" and node.attrib.enabled=="true")
 local a=node.attrib
 local gem=assert(assert(data.gemsByGameId[a.gemId])[a.variantId])
 assert(gem.grantedEffect.id==a.skillId)
 local instance={gemData=gem,level=assert(tonumber(a.level)),quality=assert(tonumber(a.quality))}
 local perOrigin={}
 nativeProcessSupport(gem.grantedEffect,instance,env,cfg,index,{},processed,{perOrigin})
 -- These original rows have exactly one support effect per physical support.
 -- A separate discovery contract is needed before claiming additional effects.
 for _,additional in ipairs(gem.additionalGrantedEffects) do assert(not additional.support) end
 if index==1 then assert(#perOrigin==0)
 else
  assert(#perOrigin==1)
  assert(perOrigin[1].level==1 and perOrigin[1].quality==0)
  table.insert(origins,perOrigin[1])
 end
end
assert(#origins==nativeExpectedCount)
local a=group[1].attrib
return nativeObserve(nativeInstance(a.skillId,tonumber(a.level),tonumber(a.quality)),origins)
"#).set_name("@original-selected-support-native-reference").eval().unwrap();
            compare(&oracle.lua, observed);
        }
    }
}
