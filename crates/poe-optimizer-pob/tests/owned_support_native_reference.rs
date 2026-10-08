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
LoadModule("Modules/CalcPerform")
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
 return {active=active,origins=origins,selected=selected,prepared=prepared,summon=summon,
  isPlayerActor=true}
end
-- Explicit component orchestration follows createMinionSkills' retained-list
-- boundary. It does not claim discovery of actors or full child skill setup.
function nativeObserveFamily(active,origins,children)
 local parent=nativeObserve(active,origins)
 parent.children={}
 for _,child in ipairs(children) do
  local actor={enemy={player={}}}
  local prepared=nativeSupportCalcs.createActiveSkill(child,parent.selected,{mode="MAIN"},actor,nil,parent.prepared)
  assert(prepared.supportList==parent.prepared.supportList)
  table.insert(parent.children,{active=child,origins=origins,selected=parent.selected,
   prepared=prepared,summon=parent.prepared,isPlayerActor=false})
 end
 return parent
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
    let perform: Function = oracle
        .lua
        .globals()
        .get::<Table>("nativeSupportCalcs")
        .unwrap()
        .get("perform")
        .unwrap();
    let transfer = original_upvalue(&oracle.lua, &perform, "addMinionModifiers");
    assert_eq!(
        transfer.info().source.as_deref(),
        Some("@src/Modules/CalcPerform.lua")
    );
    assert_eq!(transfer.info().line_defined, Some(1161));
    oracle
        .lua
        .globals()
        .set("nativeTransferMinionModifiers", transfer)
        .unwrap();
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
    let mut observations = vec![source.clone()];
    if let Some(children) = source.get::<Option<Table>>("children").unwrap() {
        observations.extend(children.sequence_values::<Table>().map(Result::unwrap));
    }
    let mut targets: Vec<_> = observations.iter().map(source_target).collect();
    for target in &targets {
        for context in std::iter::once(&target.types).chain(target.summoner.as_ref()) {
            vocabulary.extend(context.skill_types.members.iter().cloned());
            vocabulary.extend(
                context
                    .minion_types
                    .iter()
                    .flat_map(|m| m.members.iter().cloned()),
            );
        }
    }
    let package = package(
        supports,
        vocabulary,
        effect_symbols.into_values().collect(),
        family_symbols.into_values().collect(),
    );
    let SupportSelectionOutcome::Known(selected) =
        select_supports(&package, &origins, Default::default()).unwrap()
    else {
        panic!("complete source origin inputs must resolve")
    };
    assert_eq!(selected.origins(), origins);
    let mut parent_types: Option<SupportTypeContext> = None;
    for (index, (observation, target)) in observations.iter().zip(&mut targets).enumerate() {
        target.target = SkillTarget::Authored(occurrence(1000 + index as u64));
        if let Some(parent) = &parent_types {
            // Use the native parent's completed type state, not the oracle's
            // prepared result, as the child summoner input.
            target.summoner = Some(parent.clone());
        }
        let SupportPreparationOutcome::Known(prepared) =
            prepare_selected_supports(&package, &selected, target, Default::default()).unwrap()
        else {
            panic!("complete source component inputs must resolve")
        };
        if index == 0 {
            assert_eq!(
                prepare_supports(&package, &origins, target, Default::default()).unwrap(),
                SupportPreparationOutcome::Known(prepared.clone()),
                "composed and split preparation preserve the existing entry point"
            );
            parent_types = Some(SupportTypeContext {
                skill_types: DeclaredSet::complete(prepared.final_types.clone()),
                minion_types: target.types.minion_types.clone(),
            });
        }
        assert_prepared_matches(observation, &prepared);
    }
    assert_eq!(
        selected.origins(),
        origins,
        "target preparation keeps origin scalars"
    );
}

fn source_target(source: &Table) -> SupportPreparationTarget {
    let active: Table = source.get("active").unwrap();
    let active_definition: Table = active.get("grantedEffect").unwrap();
    let context = |own: Option<Table>, minion: Option<Table>| SupportTypeContext {
        skill_types: DeclaredSet::complete(types(own)),
        minion_types: minion.map(|m| DeclaredSet::complete(types(Some(m)))),
    };
    SupportPreparationTarget {
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
        is_player_actor: Some(flag(source, "isPlayerActor")),
    }
}

fn assert_prepared_matches(source: &Table, prepared: &PreparedSupports) {
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
            support_discovery: None,
            existing_actor_rules: None,
            contribution_queries: None,
            effect_applications: None,
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
fn inherited_selection_reuses_origin_values_and_keeps_child_admission_independent() {
    run(&[
        // Actual source definitions, with controlled component quality values
        // and explicit synthetic parent/child pairings, not actor discovery.
        r#"local elemental=nativeInstance("SupportElementalArmamentPlayerTwo",1,17)
 local meat=nativeInstance("SupportMeatShieldPlayerTwo",1,23)
 local family=nativeObserveFamily(nativeInstance("WolfPackPlayer"),{elemental,meat},{
  nativeInstance("MeleeAtAnimationSpeed"),nativeInstance("HealSkeletonClericMinion")})
 assert(#family.prepared.effectList==3)
 for _,child in ipairs(family.children) do
  assert(#child.prepared.effectList==3)
  assert(child.selected[1]==elemental and child.selected[2]==meat)
  assert(elemental.level==1 and elemental.quality==17 and meat.level==1 and meat.quality==23)
 end
 return family"#,
        // Synthetic eligibility flags isolate child-owned facts. Both supports
        // pass the parent; lack of a physical child Gem and player actor reject
        // them independently when preparing the child.
        r#"local gemOnly=nativeSynthetic("gem-only",{SkillType.Attack})
 gemOnly.grantedEffect.supportGemsOnly=true
 local trigger=nativeSynthetic("trigger",{SkillType.Attack})
 trigger.grantedEffect.isTrigger=true
 local family=nativeObserveFamily(nativeInstance("TwisterPlayer"),{
  gemOnly,trigger,nativeInstance("SupportElementalArmamentPlayerTwo")},{nativeInstance("MeleeAtAnimationSpeed")})
 assert(#family.prepared.effectList==4 and #family.children[1].prepared.effectList==2)
 return family"#,
        // Parent rejection does not remove the retained position. Explicit
        // synthetic instance facts make the child eligible for that same effect.
        r#"local support=nativeSynthetic("item-origin",{SkillType.Attack})
 support.grantedEffect.fromItem=true
 local parent=nativeInstance("TwisterPlayer");parent.srcInstance={fromItem=true}
 local family=nativeObserveFamily(parent,{support},{nativeInstance("MeleeAtAnimationSpeed")})
 assert(#family.prepared.effectList==1 and #family.prepared.supportList==1)
 assert(#family.children[1].prepared.effectList==2)
 return family"#,
        r#"local support=nativeSynthetic("gem-only",{SkillType.Attack})
 support.grantedEffect.supportGemsOnly=true
 local parent=nativeInstance("TwisterPlayer");parent.gemData=nil
 local family=nativeObserveFamily(parent,{support},{nativeInstance("FireboltPlayer")})
 assert(#family.prepared.effectList==1 and #family.children[1].prepared.effectList==2)
 return family"#,
        r#"local child=nativeInstance("MeleeAtAnimationSpeed")
 local original=child.grantedEffect;child.grantedEffect={}
 for k,v in pairs(original) do child.grantedEffect[k]=v end
 child.grantedEffect.cannotBeSupported=true
 local family=nativeObserveFamily(nativeInstance("TwisterPlayer"),{
  nativeInstance("SupportElementalArmamentPlayerTwo")},{child})
 assert(#family.prepared.effectList==2 and #family.children[1].prepared.effectList==1)
 assert(#family.children[1].selected==1)
 return family"#,
    ]);
}

#[test]
fn inherited_preparation_restarts_child_types_and_retains_duplicate_positions() {
    run(&[
        // Explicit synthetic type interactions: the parent keeps a type added
        // by a support that is no longer admitted. The child starts from its
        // own definition, while predicates see the parent's completed types.
        r#"local family=nativeObserveFamily(nativeInstance("FireboltPlayer"),{
 nativeSynthetic("former",{SkillType.Spell},{SkillType.ConsumesRage},{SkillType.Duration}),
 nativeSynthetic("invalidator",{SkillType.Spell},{SkillType.Duration}),
 nativeSynthetic("consumer",{SkillType.ConsumesRage},{SkillType.Area})
 },{nativeInstance("MeleeAtAnimationSpeed")})
 assert(family.prepared.skillTypes[SkillType.ConsumesRage])
 local child=family.children[1].prepared
 assert(not child.skillTypes[SkillType.ConsumesRage])
 assert(child.skillTypes[SkillType.Duration] and child.skillTypes[SkillType.Area])
 assert(#family.prepared.effectList==3 and #child.effectList==3)
 return family"#,
        // One original replacement can occupy two selected positions. Calling
        // the selector again on that retained list would incorrectly collapse it.
        r#"local a=nativeSynthetic("family-a");a.grantedEffect.gemFamily={"a"}
 local b=nativeSynthetic("family-b");b.grantedEffect.gemFamily={"b"}
 local replacement=nativeSynthetic("replacement");replacement.grantedEffect.gemFamily={"a","b"}
 replacement.grantedEffect.levels={[7]=replacement.grantedEffect.levels[1]}
 replacement.level=7;replacement.quality=31
 local family=nativeObserveFamily(nativeInstance("WolfPackPlayer"),{a,b,replacement},{nativeInstance("MeleeAtAnimationSpeed")})
 assert(#family.selected==2 and family.selected[1]==replacement and family.selected[2]==replacement)
 assert(#family.prepared.effectList==3 and #family.children[1].prepared.effectList==3)
 local reselected={}
 for _,effect in ipairs(family.selected) do nativeSelectionBest(effect,reselected,"MAIN") end
 assert(#reselected==1 and replacement.level==7 and replacement.quality==31)
 return family"#,
    ]);
}

#[test]
fn original_actor_transfer_and_action_predicates_are_distinct_receiver_operations() {
    for warm in [false, true] {
        oracle(warm)
            .lua
            .load(
                r#"
local meat=nativeInstance("SupportMeatShieldPlayerTwo")
local elemental=nativeInstance("SupportElementalArmamentPlayerTwo")
local family=nativeObserveFamily(nativeInstance("SummonSkeletalClericsPlayer"),{meat},{
 nativeInstance("HealSkeletonClericMinion")})
local parentMods=new("ModList"):ModList()
for _,effect in ipairs(family.prepared.effectList) do
 if effect.grantedEffect.support then nativeSupportCalcs.mergeSkillInstanceMods({},parentMods,effect) end
end
-- Original merge retains wrapper effects instead of immediately modifying the
-- summoning skill's damage. The untouched transfer closure moves them once into
-- this explicit actor. No full CalcPerform environment is emulated here.
assert(parentMods:More(nil,"Damage")==1 and parentMods:More(nil,"DamageTaken")==1)
local minion={type="RaisedSkeletonCleric",modDB=new("ModDB"):ModDB()}
nativeTransferMinionModifiers(parentMods,{},minion)
assert(minion.modDB:More(nil,"Damage")==0.6 and minion.modDB:More(nil,"DamageTaken")==0.6)
assert(minion.modDB:More({keywordFlags=KeywordFlag.Attack},"ElementalDamage")==1)
for _,child in ipairs(family.children) do
 local actionMods=new("ModList"):ModList(minion.modDB)
 for _,effect in ipairs(child.prepared.effectList) do
  if effect.grantedEffect.support then nativeSupportCalcs.mergeSkillInstanceMods({},actionMods,effect) end
 end
 -- Repeated child-local wrappers remain nested. Actor inheritance contributes
 -- one factor, not one extra factor per admitted child support or child action.
 assert(actionMods:More(nil,"Damage")==0.6 and actionMods:More(nil,"DamageTaken")==0.6)
 assert(#child.prepared.effectList==2)
end
assert(minion.modDB:More(nil,"Damage")==0.6)
-- A distinct original action modifier remains query-filtered. This controlled
-- modifier query observes its predicate without inventing a Cleric attack.
local elementalMods=new("ModList"):ModList()
nativeSupportCalcs.mergeSkillInstanceMods({},elementalMods,elemental)
assert(elementalMods:More({keywordFlags=KeywordFlag.Attack},"ElementalDamage")==1.25)
assert(elementalMods:More({keywordFlags=KeywordFlag.Spell},"ElementalDamage")==1)
"#,
            )
            .set_name("@original-minion-transfer-and-child-action-components")
            .exec()
            .unwrap();
    }
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
