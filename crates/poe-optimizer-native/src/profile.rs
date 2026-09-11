//! Strict source-document projection for the closed native Spark and Mace profiles.
use poe_optimizer_core::{evaluation::*, options::*};
use poe_optimizer_engine::{
    mace::{MaceInput, MaceWeapon},
    spark::{SparkInput, SparkQuestRewards},
};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, SourceOccurrenceId},
    selected_view::{DomainSelection, SelectedView, SelectionDomain, SetOrigin},
};
use roxmltree::{Document, Node, ParsingOptions};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy)]
pub(crate) enum NativeInput {
    Spark(SparkInput),
    Mace(MaceInput),
}
pub(crate) struct Profile {
    pub input: NativeInput,
    pub support_keys: Vec<String>,
    pub support_order: Vec<String>,
    pub prepared_supports: Option<poe_optimizer_engine::mace_supports::PreparedMaceSupports>,
    pub weapon_record: Option<poe_optimizer_import::mace_item::ValidatedMaceWeapon>,
    pub prepared_weapon: Option<poe_optimizer_engine::weapon::PreparedWeaponStats>,
    pub equipment: BTreeMap<String, poe_optimizer_import::equipment::ValidatedEquipmentItem>,
    pub actor_modifiers: poe_optimizer_import::actor_modifiers::ValidatedActorModifiers,
    pub actor_quests: poe_optimizer_engine::actor::ActorQuestSelection,
    pub prepared_actor: poe_optimizer_engine::actor::PreparedActorResources,
    pub prepared_armour: BTreeMap<String, poe_optimizer_engine::armour::PreparedArmour>,
    pub tree: crate::tree::NativeTree,
    pub enemy_level: u32,
    pub config: BTreeMap<String, Scalar>,
    pub authored_config: BTreeMap<String, Scalar>,
    pub export_xml: String,
    pub group_label: Option<String>,
}
/// The legacy adapter consumes explicit source scalars. A valid injected loader
/// policy may rewrite them; such a change needs the general effective pipeline,
/// not a calculation from stale raw XML. Encounter overrides apply only later.
pub(crate) fn validate_loaded_configuration_projection(
    authored: &BTreeMap<String, Scalar>,
    blocks: &[poe_optimizer_import::actor_modifiers::ActorModifierBlock],
    prepared: &crate::configuration::PreparedConfiguration,
) -> Result<(), EvaluationError> {
    use crate::configuration::{
        ConfigurationBlockText, ConfigurationPrefixStatus, ConfigurationValue,
    };
    let report = prepared.report();
    if report.status != ConfigurationPrefixStatus::Prepared {
        return Err(unsupported(
            "Authored configuration prefix did not finish; the legacy adapter cannot bypass its source failure or unsupported boundary",
        ));
    }
    let active = report.active_set.as_ref().ok_or_else(|| {
        unsupported("Authored configuration has no selected set for the legacy adapter")
    })?;
    if Some(active) != report.view_selected_set.as_ref() {
        return Err(unsupported(
            "Authored configuration activation has not reached the requested set",
        ));
    }
    let set = report
        .sets
        .iter()
        .find(|set| set.winner && &set.origin == active)
        .ok_or_else(|| unsupported("Authored configuration selected set is unavailable"))?;
    // Raw actor parsing is a separate source consumer, including legacy customMods.
    // Compare its post-migration block projection, not just scalar encounter keys.
    // Both consumers trim outer ASCII whitespace before parsing modifier lines.
    let same_blocks = if blocks.is_empty() {
        set.blocks.len() == 1
            && set.blocks[0].enabled
            && matches!(&set.blocks[0].text,
                ConfigurationBlockText::Value { value: ConfigurationValue::Text(text) }
                    if text.trim_ascii().is_empty())
    } else {
        blocks.len() == set.blocks.len()
            && blocks.iter().zip(&set.blocks).all(|(raw, loaded)| {
                raw.title == loaded.title
                    && raw.enabled == loaded.enabled
                    && matches!(&loaded.text,
                    ConfigurationBlockText::Value { value: ConfigurationValue::Text(text) }
                        if text.trim_ascii() == raw.text.trim_ascii())
            })
    };
    if !same_blocks {
        return Err(unsupported(
            "Processed configuration modifier blocks differ from the raw legacy adapter; effective configuration preparation is required",
        ));
    }
    for (key, expected) in authored {
        let same = match (expected, set.inputs.get(key)) {
            (Scalar::Boolean(a), Some(ConfigurationValue::Boolean(b))) => a == b,
            (Scalar::Number(a), Some(ConfigurationValue::Number(b))) => *a == b.value(),
            (Scalar::Text(a), Some(ConfigurationValue::Text(b))) => a == b,
            _ => false,
        };
        if !same {
            return Err(unsupported(format!(
                "Processed configuration input {key} differs from the raw legacy adapter; effective configuration preparation is required"
            )));
        }
    }
    Ok(())
}

fn unsupported(message: impl Into<String>) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::UnsupportedCapability, message)
}
fn child<'a, 'input>(
    node: Node<'a, 'input>,
    name: &str,
) -> Result<Node<'a, 'input>, EvaluationError> {
    let values: Vec<_> = node.children().filter(|n| n.has_tag_name(name)).collect();
    if values.len() != 1 {
        return Err(unsupported(format!(
            "Native build profile requires exactly one {name}"
        )));
    }
    Ok(values[0])
}
/// The closed profile consumes the shared view's selected source, while its
/// existing cardinality and raw-value guards continue to limit native admission.
/// Source ranges are resolved in one temporary document, never reserialized.
fn selected_set<'a, 'input>(
    parent: Node<'a, 'input>,
    view: &SelectedView<'_>,
    selection: &DomainSelection,
    domain: SelectionDomain,
    tag: &str,
) -> Result<Node<'a, 'input>, EvaluationError> {
    let record = selection
        .selected
        .as_ref()
        .ok_or_else(|| unsupported(format!("Native profile requires a selected {tag}")))?;
    let SetOrigin::Authored { instance, source } = record.origin else {
        return Err(unsupported(format!(
            "Native profile requires an authored {tag}; loader defaults need further preparation"
        )));
    };
    let correct_domain = matches!(
        (domain, instance),
        (SelectionDomain::Skills, AuthoredInstanceId::SkillSet(_))
            | (SelectionDomain::Items, AuthoredInstanceId::ItemSet(_))
            | (
                SelectionDomain::Passives,
                AuthoredInstanceId::PassiveSpec(_)
            )
            | (
                SelectionDomain::Configuration,
                AuthoredInstanceId::ConfigSet(_)
            )
    );
    if selection.domain != domain || !correct_domain {
        return Err(binding_error("Selected set has a foreign domain"));
    }
    let binding = view
        .build()
        .binding(instance)
        .map_err(|error| binding_error(error.to_string()))?;
    if binding.source() != source {
        return Err(binding_error(
            "Selected set source differs from its owned instance",
        ));
    }
    let selected = source_node(parent.document(), view, source, tag)?;
    // Retain the prior exactly-one-set domain until inactive load effects and
    // multi-set export/report semantics have their own source-backed admission.
    if child(parent, tag)? != selected {
        return Err(binding_error(
            "Selected set is outside its source container",
        ));
    }
    Ok(selected)
}

fn selected_group<'a, 'input>(
    parent: Node<'a, 'input>,
    view: &SelectedView<'_>,
) -> Result<Node<'a, 'input>, EvaluationError> {
    let members = &view
        .report()
        .skills
        .selected
        .as_ref()
        .ok_or_else(|| unsupported("Native profile requires a selected skill set"))?
        .members;
    let [instance @ AuthoredInstanceId::SkillGroup(_)] = members.as_slice() else {
        return Err(unsupported(
            "Native profile requires exactly one selected authored skill group",
        ));
    };
    let binding = view
        .build()
        .binding(*instance)
        .map_err(|error| binding_error(error.to_string()))?;
    let selected = source_node(parent.document(), view, binding.source(), "Skill")?;
    if child(parent, "Skill")? != selected {
        return Err(binding_error(
            "Selected skill group is outside its source set",
        ));
    }
    Ok(selected)
}

fn source_node<'a, 'input>(
    document: &'a Document<'input>,
    view: &SelectedView<'_>,
    source: SourceOccurrenceId,
    tag: &str,
) -> Result<Node<'a, 'input>, EvaluationError> {
    let range = view
        .build()
        .occurrence(source)
        .map_err(|error| binding_error(error.to_string()))?
        .range();
    document
        .descendants()
        .find(|node| node.is_element() && node.range() == range && node.has_tag_name(tag))
        .ok_or_else(|| {
            binding_error("Selected source occurrence is absent from the parsed document")
        })
}

/// Closed numerical adapters may consume only the same skill state they admitted
/// from source. Injected loader definitions can clamp levels or resolve another
/// effect; those results belong to the general stage, not stale profile numbers.
pub(crate) fn validate_loaded_skill_projection(
    build: &poe_optimizer_import::build_instance::ImportedBuildInstance,
    data: &crate::CompiledGameData,
    skills: &crate::skills::PreparedSkills,
) -> Result<(), EvaluationError> {
    use crate::skills::{SkillIdentityStatus, SkillPreparationStatus};
    let stage = skills.report();
    if stage.status != SkillPreparationStatus::Complete {
        return Err(unsupported(
            "Native numerical adapter requires completed authored skill loading",
        ));
    }
    let [selected] = stage.selected_groups.as_slice() else {
        return Err(unsupported(
            "Native numerical adapter requires one loaded skill group",
        ));
    };
    let group = stage
        .groups
        .iter()
        .find(|group| group.instance == *selected && group.attached)
        .ok_or_else(|| binding_error("Loaded selected group is absent"))?;
    for (index, gem) in group.gems.iter().enumerate() {
        let expected_skill = build
            .attribute(gem.source, "skillId")
            .map_err(|e| binding_error(e.to_string()))?;
        let expected_skill = expected_skill.as_ref().map(|value| value.decoded());
        if !gem.processed
            || gem.identity_status != SkillIdentityStatus::ResolvedGem
            || gem.text("skillId") != expected_skill
        {
            return Err(unsupported(
                "Authored skill resolution differs from the closed numerical adapter",
            ));
        }
        for name in ["level", "quality"] {
            let expected = build
                .attribute(gem.source, name)
                .map_err(|e| binding_error(e.to_string()))?
                .and_then(|value| value.decoded().parse::<f64>().ok());
            if gem.number(name) != expected {
                return Err(unsupported(format!(
                    "Processed skill {name} differs from the closed numerical adapter"
                )));
            }
        }
        let definition = gem
            .gem_data
            .as_ref()
            .and_then(|key| data.snapshot().skill_identities().gem_by_key(key))
            .ok_or_else(|| binding_error("Processed gem definition is absent"))?;
        if definition.effect_list.len() != 1
            || definition.effect_list.first().map(String::as_str) != expected_skill
        {
            return Err(unsupported(
                "Additional processed gem effects require the general numerical evaluator",
            ));
        }
        let effect = expected_skill
            .and_then(|id| data.snapshot().skill_identities().skill_by_id(id))
            .ok_or_else(|| binding_error("Processed effect definition is absent"))?;
        if (effect.support == Some(true)) != (index > 0) || effect.from_tree == Some(true) {
            return Err(unsupported(
                "Processed support/provider role differs from the closed numerical adapter",
            ));
        }
    }
    Ok(())
}

fn binding_error(message: impl Into<String>) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::BackendContract, message)
}

fn validate_view(
    request: &EvaluationRequest,
    data: &crate::CompiledGameData,
    view: &SelectedView<'_>,
) -> Result<(), EvaluationError> {
    view.validate_binding(view.build(), data.snapshot())
        .map_err(|error| binding_error(error.to_string()))?;
    if request.build.content != view.build().source_xml() {
        return Err(binding_error(
            "Native request XML differs from selected view source",
        ));
    }
    let report = view.report();
    if let Some(problem) = report.load_problems.first().or_else(|| {
        [
            &report.skills,
            &report.items,
            &report.passives,
            &report.configuration,
        ]
        .into_iter()
        .find_map(|selection| selection.problem.as_ref())
    }) {
        return Err(unsupported(format!(
            "Native selected view requires source preparation: {}: {}",
            problem.code, problem.message
        )));
    }
    if let Some(problem) = &report.skill_identities.problem {
        return Err(unsupported(format!(
            "Native selected skill identity preparation: {problem}"
        )));
    }
    if report.weapon_state.use_second_weapon_set != Some(false) {
        return Err(unsupported(
            "Native equipment requires the first selected weapon set",
        ));
    }
    Ok(())
}

fn only(node: Node<'_, '_>, attrs: &[&str], children: &[&str]) -> Result<(), EvaluationError> {
    if node.tag_name().namespace().is_some()
        || node
            .attributes()
            .any(|a| a.namespace().is_some() || !attrs.contains(&a.name()))
        || node
            .children()
            .filter(Node::is_element)
            .any(|n| n.tag_name().namespace().is_some() || !children.contains(&n.tag_name().name()))
        || node
            .children()
            .any(|n| n.is_text() && !n.text().unwrap_or("").trim().is_empty())
    {
        return Err(unsupported(format!(
            "Unsupported native build fields or content in {}",
            node.tag_name().name()
        )));
    }
    Ok(())
}
fn fixed(node: Node<'_, '_>, pairs: &[(&str, &str)]) -> Result<(), EvaluationError> {
    for (key, value) in pairs {
        if node.attribute(*key) != Some(*value) {
            return Err(unsupported(format!(
                "Native build profile requires {}.{key}={value}",
                node.tag_name().name()
            )));
        }
    }
    Ok(())
}
fn scalar(node: Node<'_, '_>) -> Result<Scalar, EvaluationError> {
    poe_optimizer_import::configuration::read_input_scalar(node)
        .map_err(|error| unsupported(error.to_string()))
}
fn validate_config(
    name: &str,
    value: &Scalar,
    is_mace: bool,
    data: &crate::CompiledGameData,
) -> Result<(), EvaluationError> {
    let valid = match (name, value) {
        ("enemyIsBoss", Scalar::Text(v)) => {
            if is_mace {
                v == "None"
            } else {
                ["None", "Boss", "Pinnacle"].contains(&v.as_str())
            }
        }
        ("enemyArmour" | "enemyEvasion", Scalar::Number(v)) if is_mace => {
            (0.0..=1_000_000.0).contains(v)
        }
        ("enemyDamageType", Scalar::Text(v)) => v == "Melee",
        (
            "conditionEnemyShocked"
            | "conditionEnemyChilled"
            | "conditionEnemyIgnited"
            | "conditionCritRecently"
            | "conditionBeenHitRecently",
            Scalar::Boolean(v),
        ) => !v,
        ("enemyLevel", Scalar::Number(v)) => (1.0..=85.0).contains(v) && v.fract() == 0.0,
        (
            "enemyFireResist" | "enemyColdResist" | "enemyLightningResist" | "enemyChaosResist",
            Scalar::Number(v),
        ) => (-200.0..=200.0).contains(v),
        (
            "enemyPhysicalDamage"
            | "enemyFireDamage"
            | "enemyColdDamage"
            | "enemyLightningDamage"
            | "enemyChaosDamage",
            Scalar::Number(v),
        ) => (0.0..=1_000_000.0).contains(v),
        (
            "enemyPhysicalOverwhelm" | "enemyFirePen" | "enemyColdPen" | "enemyLightningPen",
            Scalar::Number(v),
        ) => (0.0..=100.0).contains(v),
        ("enemyCritChance", Scalar::Number(v)) => *v == 0.0,
        ("enemySpeed", Scalar::Number(v)) => (1.0..=60_000.0).contains(v),
        ("multiplierNearbyEnemies", Scalar::Number(v)) => *v == 1.0,
        ("multiplierNearbyRareOrUniqueEnemies", Scalar::Number(v)) => *v == 0.0 || *v == 1.0,
        ("resistancePenalty", Scalar::Number(v)) => (-60.0..=0.0).contains(v),
        (name, Scalar::Boolean(_))
            if data
                .snapshot()
                .package()
                .quests
                .config_keys
                .iter()
                .any(|key| key == name)
                || data
                    .snapshot()
                    .package()
                    .actor
                    .spirit_quests
                    .iter()
                    .any(|quest| quest.config_key == name) =>
        {
            true
        }
        _ => false,
    };
    if !valid {
        return Err(unsupported(format!(
            "Unsupported native build configuration {name}={value:?}"
        )));
    }
    Ok(())
}
fn number(map: &BTreeMap<String, Scalar>, name: &str) -> f64 {
    match &map[name] {
        Scalar::Number(n) => *n,
        _ => unreachable!("validated configuration type"),
    }
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}

/// Fixed encounter/quest inputs validated through the complete source parser.
/// No selected character, resource, or local weapon calculation is performed.
pub(crate) struct ScenarioProfile {
    pub input: NativeInput,
    pub config: BTreeMap<String, Scalar>,
    pub authored_config: BTreeMap<String, Scalar>,
    pub authored_blocks: Vec<poe_optimizer_import::actor_modifiers::ActorModifierBlock>,
    pub actor_quests: poe_optimizer_engine::actor::ActorQuestSelection,
}
// This short-lived projection stays on the stack rather than adding a heap
// allocation to every full document preparation.
#[allow(clippy::large_enum_variant)]
enum ProfileProjection {
    Complete(Profile),
    Scenario(ScenarioProfile),
}
pub(crate) fn parse(
    request: &EvaluationRequest,
    data: &crate::CompiledGameData,
    view: &SelectedView<'_>,
) -> Result<Profile, EvaluationError> {
    match parse_projection(request, data, view, true)? {
        ProfileProjection::Complete(profile) => Ok(profile),
        ProfileProjection::Scenario(_) => unreachable!("full projection requested"),
    }
}
pub(crate) fn prepare_scenario(
    request: &EvaluationRequest,
    data: &crate::CompiledGameData,
    view: &SelectedView<'_>,
) -> Result<ScenarioProfile, EvaluationError> {
    let known = crate::metric_catalog();
    let mut queries = BTreeSet::new();
    for query in &request.metrics {
        if !queries.insert(query) {
            return Err(EvaluationError::new(
                EvaluationErrorKind::InvalidRequest,
                "Duplicate metric query",
            ));
        }
        if !known
            .iter()
            .any(|metric| metric.id == query.id && metric.actors.contains(&query.actor))
        {
            return Err(EvaluationError::new(
                EvaluationErrorKind::UnsupportedCapability,
                format!(
                    "Native backend does not implement {:?}.{}",
                    query.actor, query.id
                ),
            ));
        }
    }
    match parse_projection(request, data, view, false)? {
        ProfileProjection::Scenario(profile) => Ok(profile),
        ProfileProjection::Complete(_) => unreachable!("scenario projection requested"),
    }
}
fn parse_projection(
    request: &EvaluationRequest,
    data: &crate::CompiledGameData,
    view: &SelectedView<'_>,
    prepare_numeric: bool,
) -> Result<ProfileProjection, EvaluationError> {
    validate_view(request, data, view)?;
    request
        .options
        .validate()
        .map_err(|e| EvaluationError::new(EvaluationErrorKind::InvalidRequest, e))?;
    if request.build.content.len() > poe_optimizer_import::MAX_XML_BYTES {
        return Err(unsupported("Native XML exceeds byte limit"));
    }
    let doc = Document::parse_with_options(
        view.build().source_xml(),
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: poe_optimizer_import::MAX_XML_NODES,
            entity_resolver: None,
        },
    )
    .map_err(|e| EvaluationError::new(EvaluationErrorKind::InvalidRequest, e.to_string()))?;
    poe_optimizer_import::root_admission::validate_main(&doc)
        .map_err(|error| unsupported(format!("Native {error}")))?;
    let root = doc.root_element();
    let build = child(root, "Build")?;
    let skills = child(root, "Skills")?;
    let skill_set = selected_set(
        skills,
        view,
        &view.report().skills,
        SelectionDomain::Skills,
        "SkillSet",
    )?;
    let main_group = selected_group(skill_set, view)?;
    let main_gem = main_group
        .children()
        .find(|node| node.has_tag_name("Gem"))
        .ok_or_else(|| unsupported("Native profile requires a main active gem"))?;
    let is_mace = match main_gem.attribute("skillId") {
        Some(id) if id == data.snapshot().package().spark.skill_id => false,
        Some(id) if id == data.snapshot().package().mace.skill_id => true,
        _ => {
            return Err(unsupported(
                "Native profiles currently support Spark or Mace Strike",
            ));
        }
    };
    only(
        build,
        &[
            "level",
            "className",
            "ascendClassName",
            "targetVersion",
            "characterLevelAutoMode",
            "mainSocketGroup",
            "viewMode",
        ],
        &["PlayerStat", "MinionStat"],
    )?;
    fixed(
        build,
        &[
            ("targetVersion", "0_1"),
            ("characterLevelAutoMode", "false"),
            ("mainSocketGroup", "1"),
        ],
    )?;
    for cached in build.children().filter(Node::is_element) {
        only(cached, &["stat", "value"], &[])?;
    }
    let level = build
        .attribute("level")
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|n| (1..=100).contains(n))
        .ok_or_else(|| unsupported("Character level must be 1..100"))?;
    let tree = child(root, "Tree")?;
    only(tree, &["activeSpec"], &["Spec"])?;
    fixed(tree, &[("activeSpec", "1")])?;
    let spec = selected_set(
        tree,
        view,
        &view.report().passives,
        SelectionDomain::Passives,
        "Spec",
    )?;
    only(
        spec,
        &[
            "title",
            "classId",
            "classInternalId",
            "ascendClassId",
            "ascendancyInternalId",
            "treeVersion",
            "nodes",
            "masteryEffects",
        ],
        &["Overrides"],
    )?;
    fixed(
        spec,
        &[
            ("treeVersion", &data.snapshot().tree().source.tree_version),
            ("masteryEffects", ""),
        ],
    )?;
    let resolved_tree = if prepare_numeric {
        Some(crate::tree::NativeTree::resolve(build, spec, data)?)
    } else {
        crate::tree::validate_calculation_source(&data.snapshot().tree().source)?;
        poe_optimizer_import::controlled_build::parse_passive_allocation(
            build,
            spec,
            data.snapshot().package(),
        )
        .map_err(|error| unsupported(error.to_string()))?
        .resolve(data.snapshot())
        .map_err(|error| unsupported(error.to_string()))?;
        None
    };
    only(
        skills,
        &["activeSkillSet", "defaultGemLevel", "defaultGemQuality"],
        &["SkillSet"],
    )?;
    fixed(
        skills,
        &[
            ("activeSkillSet", "1"),
            ("defaultGemLevel", "normalMaximum"),
            ("defaultGemQuality", "0"),
        ],
    )?;
    only(skill_set, &["id", "title"], &["Skill"])?;
    fixed(skill_set, &[("id", "1")])?;
    let skill = main_group;
    only(
        skill,
        &[
            "enabled",
            "includeInFullDPS",
            "groupCount",
            "label",
            "mainActiveSkill",
            "mainActiveSkillCalcs",
        ],
        &["Gem"],
    )?;
    fixed(
        skill,
        &[
            ("enabled", "true"),
            ("includeInFullDPS", "true"),
            ("groupCount", "1"),
            ("mainActiveSkill", "1"),
            ("mainActiveSkillCalcs", "1"),
        ],
    )?;
    let gems: Vec<_> = skill.children().filter(Node::is_element).collect();
    if gems.is_empty() || gems.len() > if is_mace { 3 } else { 1 } {
        return Err(unsupported(
            "Native profile requires one active skill and zero to two reviewed Mace supports",
        ));
    }
    let package = data.snapshot().package();
    if is_mace {
        let gem = &package.mace;
        validate_gem(
            gems[0],
            &gem.name,
            &gem.skill_id,
            &gem.game_id,
            &gem.variant_id,
        )?;
    } else {
        let gem = &package.spark;
        validate_gem(
            gems[0],
            &gem.name,
            &gem.skill_id,
            &gem.game_id,
            &gem.variant_id,
        )?;
    }
    let mut support_order = Vec::new();
    for node in &gems[1..] {
        let gem = package
            .supports
            .iter()
            .find(|gem| Some(gem.skill_id.as_str()) == node.attribute("skillId"))
            .ok_or_else(|| unsupported("Unknown native Mace support"))?;
        validate_gem(
            *node,
            &gem.name,
            &gem.skill_id,
            &gem.game_id,
            &gem.variant_id,
        )?;
        support_order.push(gem.id.clone());
    }
    let mut support_keys = support_order.clone();
    support_keys.sort();
    let prepared_supports = if is_mace {
        Some(
            data.mace_support_loadout(&support_keys)
                .map_err(|error| unsupported(error.to_string()))?
                .clone(),
        )
    } else {
        None
    };
    let items = child(root, "Items")?;
    only(items, &["activeItemSet"], &["Item", "ItemSet"])?;
    fixed(items, &[("activeItemSet", "1")])?;
    let item_set = selected_set(
        items,
        view,
        &view.report().items,
        SelectionDomain::Items,
        "ItemSet",
    )?;
    only(item_set, &["id", "title", "useSecondWeaponSet"], &["Slot"])?;
    fixed(item_set, &[("id", "1")])?;
    if item_set
        .attribute("useSecondWeaponSet")
        .is_some_and(|value| value != "false")
        || (is_mace && item_set.attribute("useSecondWeaponSet") != Some("false"))
    {
        return Err(unsupported(
            "Native equipment requires the first weapon set",
        ));
    }
    let mut supplied = BTreeMap::new();
    for item in items.children().filter(|node| node.has_tag_name("Item")) {
        let parsed = poe_optimizer_import::equipment::parse_equipment_item_xml(item, package)
            .map_err(|error| unsupported(error.to_string()))?;
        if supplied.insert(parsed.pob_item_id(), parsed).is_some() {
            return Err(unsupported("Duplicate physical item ID"));
        }
    }
    let mut equipment = BTreeMap::new();
    let mut selected_slots = BTreeSet::new();
    for slot in item_set.children().filter(Node::is_element) {
        only(slot, &["name", "itemId"], &[])?;
        let name = slot
            .attribute("name")
            .ok_or_else(|| unsupported("Missing equipment slot name"))?;
        if !poe_optimizer_import::equipment::EQUIPMENT_SOURCE_ORDER.contains(&name)
            || !selected_slots.insert(name)
        {
            return Err(unsupported("Unsupported or duplicate equipment slot"));
        }
        let raw_id = slot
            .attribute("itemId")
            .ok_or_else(|| unsupported("Missing equipment slot itemId"))?;
        let id = raw_id
            .parse::<u32>()
            .ok()
            .filter(|id| raw_id == id.to_string())
            .ok_or_else(|| {
                unsupported("Equipment slot itemId must be canonical unsigned integer")
            })?;
        if id == 0 {
            continue;
        }
        let item = supplied
            .remove(&id)
            .ok_or_else(|| unsupported("Unknown or multiply equipped physical item"))?;
        if !item.allowed_slots().iter().any(|slot| slot == name)
            || (!is_mace && item.weapon().is_some())
            || equipment.insert(name.to_owned(), item).is_some()
        {
            return Err(unsupported(
                "Unsupported, duplicate or incompatible equipment slot",
            ));
        }
    }
    // Known unequipped inventory is preserved in source and contributes no modifiers.
    let weapon = equipment
        .get("Weapon 1")
        .and_then(|item| item.weapon())
        .cloned();
    if is_mace && weapon.is_none() {
        return Err(unsupported("Native Mace requires one main-hand weapon"));
    }
    let config_node = child(root, "Config")?;
    only(config_node, &["activeConfigSet"], &["ConfigSet"])?;
    fixed(config_node, &[("activeConfigSet", "1")])?;
    let config_set = selected_set(
        config_node,
        view,
        &view.report().configuration,
        SelectionDomain::Configuration,
        "ConfigSet",
    )?;
    only(
        config_set,
        &["id", "title"],
        &["Input", "CustomModifierBlock"],
    )?;
    fixed(config_set, &[("id", "1")])?;
    let actor_modifiers =
        poe_optimizer_import::actor_modifiers::parse_actor_configuration(config_set, package)
            .map_err(|error| unsupported(error.to_string()))?;
    let mut config = BTreeMap::new();
    let mut ranges = BTreeMap::new();
    for input in config_set
        .children()
        .filter(|node| node.has_tag_name("Input"))
    {
        // The shared actor parser validates legacy text and current modifier blocks.
        // Like PoB's migration, it keeps actor sources outside the scalar encounter map.
        if input.attribute("name") == Some("customMods") {
            continue;
        }
        let name = input
            .attribute("name")
            .ok_or_else(|| unsupported("Missing configuration name"))?;
        let value = scalar(input)?;
        validate_config(name, &value, is_mace, data)?;
        if config.insert(name.to_owned(), value).is_some() {
            return Err(unsupported("Duplicate configuration input"));
        }
        ranges.insert(name.to_owned(), input.range());
    }
    let expected: BTreeSet<_> = [
        "enemyIsBoss",
        "enemyLevel",
        "enemyFireResist",
        "enemyColdResist",
        "enemyLightningResist",
        "enemyChaosResist",
        "enemyDamageType",
        "enemyPhysicalDamage",
        "enemyFireDamage",
        "enemyColdDamage",
        "enemyLightningDamage",
        "enemyChaosDamage",
        "enemyPhysicalOverwhelm",
        "enemyFirePen",
        "enemyColdPen",
        "enemyLightningPen",
        "enemyCritChance",
        "enemySpeed",
        "conditionEnemyShocked",
        "conditionEnemyChilled",
        "conditionEnemyIgnited",
        "conditionCritRecently",
        "conditionBeenHitRecently",
        "multiplierNearbyEnemies",
        "multiplierNearbyRareOrUniqueEnemies",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    if package
        .quests
        .config_keys
        .iter()
        .map(String::as_str)
        .chain(
            package
                .actor
                .spirit_quests
                .iter()
                .map(|quest| quest.config_key.as_str()),
        )
        .any(|key| {
            expected.contains(key)
                || [
                    "resistancePenalty",
                    "enemyArmour",
                    "enemyEvasion",
                    "customMods",
                ]
                .contains(&key)
        })
    {
        return Err(unsupported(
            "Quest configuration keys conflict with native encounter or actor inputs",
        ));
    }
    if !expected.is_subset(&config.keys().cloned().collect::<BTreeSet<_>>()) {
        return Err(unsupported(
            "Native build profile requires all explicit encounter inputs",
        ));
    }
    if is_mace && !config.contains_key("enemyArmour") {
        return Err(unsupported("Native Mace requires explicit enemyArmour"));
    }
    let notes: Vec<_> = root
        .children()
        .filter(|n| n.has_tag_name("Notes"))
        .collect();
    if notes.len() > 1
        || notes
            .iter()
            .any(|n| n.attributes().len() != 0 || n.children().any(|c| c.is_element()))
    {
        return Err(unsupported("Unsupported Notes structure"));
    }
    if let Some(selection) = &request.options.selection
        && (selection.socket_group != 1
            || selection.active_skill.is_some_and(|n| n != 1)
            || selection.minion_skill.is_some())
    {
        return Err(unsupported(
            "Native build profiles support only group 1 action 1 and no minion",
        ));
    }
    let original = config.clone();
    if let Some(encounter) = &request.options.encounter {
        if let Some(level) = encounter.enemy_level {
            config.insert("enemyLevel".into(), Scalar::Number(level as f64));
        }
        if let Some(boss) = encounter.boss {
            config.insert(
                "enemyIsBoss".into(),
                Scalar::Text(
                    match boss {
                        BossKind::Normal => "None",
                        BossKind::Standard => "Boss",
                        BossKind::Pinnacle => "Pinnacle",
                        BossKind::Uber => {
                            return Err(unsupported(
                                "Uber encounter is unsupported by native build profiles",
                            ));
                        }
                    }
                    .into(),
                ),
            );
        }
        if let Some(hit) = &encounter.incoming_hit {
            for (key, value) in [
                ("enemyPhysicalDamage", hit.physical),
                ("enemyFireDamage", hit.fire),
                ("enemyColdDamage", hit.cold),
                ("enemyLightningDamage", hit.lightning),
                ("enemyChaosDamage", hit.chaos),
            ] {
                config.insert(key.into(), Scalar::Number(value));
            }
        }
    }
    for (key, value) in &config {
        validate_config(key, value, is_mace, data)?;
    }
    if [
        "enemyPhysicalDamage",
        "enemyFireDamage",
        "enemyColdDamage",
        "enemyLightningDamage",
        "enemyChaosDamage",
    ]
    .into_iter()
    .all(|name| number(&config, name) == 0.0)
    {
        return Err(unsupported(
            "Incoming hit must contain a positive component",
        ));
    }
    // Input cached outputs are not calculations and must not survive a fresh export.
    let mut replacements: Vec<_> = build
        .children()
        .filter(Node::is_element)
        .map(|n| (n.range(), String::new()))
        .collect();
    for (name, value) in &config {
        if original.get(name) != Some(value) {
            let attr = match value {
                Scalar::Number(n) => format!("number=\"{n}\""),
                Scalar::Boolean(b) => format!("boolean=\"{b}\""),
                Scalar::Text(s) => format!("string=\"{}\"", escape(s)),
            };
            replacements.push((
                ranges[name].clone(),
                format!("<Input name=\"{}\" {attr}/>", escape(name)),
            ));
        }
    }
    let export_xml = apply_source_edits(view.build().source_xml(), replacements)?;
    let quest =
        |index: usize| match config.get(&data.snapshot().package().quests.config_keys[index]) {
            Some(Scalar::Boolean(v)) => *v,
            None => data.snapshot().package().quests.default_enabled[index],
            _ => unreachable!(),
        };
    let penalty = match config.get("resistancePenalty") {
        Some(Scalar::Number(n)) => *n,
        None => {
            data.snapshot()
                .package()
                .encounters
                .default_resistance_penalty
        }
        _ => unreachable!(),
    };
    let quests = SparkQuestRewards {
        candlemass: quest(0),
        molten_shrine: quest(1),
        silent_hall: quest(2),
        beira: quest(3),
        garukhan: quest(4),
        blackjaw: quest(5),
    };
    let mut actor_quests = data.actor_quest_selection(quests);
    for (index, quest) in package.actor.spirit_quests.iter().enumerate() {
        if let Some(Scalar::Boolean(enabled)) = config.get(&quest.config_key) {
            actor_quests.spirit[index] = *enabled;
        }
    }
    let enemy_level = number(&config, "enemyLevel") as u32;
    let input = if let Some(record) = &weapon {
        let weapon = weapon_slot(record.weapon_key())?;
        let item_level = record.item_level();
        let quality = record.quality();
        let enemy_evasion = match config.get("enemyEvasion") {
            Some(Scalar::Number(value)) => *value,
            None => data
                .monster_evasion(enemy_level)
                .map_err(|error| unsupported(error.to_string()))?,
            _ => unreachable!("validated evasion type"),
        };
        NativeInput::Mace(MaceInput {
            character_level: level,
            weapon,
            quality,
            item_level,
            brutality: false,
            resistance_penalty: penalty,
            quests,
            enemy_armour: number(&config, "enemyArmour"),
            enemy_evasion,
            enemy_fire_resistance: number(&config, "enemyFireResist"),
        })
    } else {
        NativeInput::Spark(SparkInput {
            character_level: level,
            resistance_penalty: penalty,
            enemy_lightning_resistance: number(&config, "enemyLightningResist"),
            quests,
        })
    };
    if !prepare_numeric {
        return Ok(ProfileProjection::Scenario(ScenarioProfile {
            input,
            config,
            authored_config: original,
            authored_blocks: actor_modifiers.blocks().to_vec(),
            actor_quests,
        }));
    }
    let resolved_tree = resolved_tree.expect("full numeric tree requested");
    let mut prepared_armour = BTreeMap::new();
    for (slot, item) in &equipment {
        if let Some(records) = item.armour_modifiers() {
            let armour = data
                .prepare_armour_with_source(
                    item.base_id(),
                    item.quality(),
                    item.item_level(),
                    &item.modifier_source(),
                    records,
                )
                .map_err(|error| unsupported(error.to_string()))?;
            if armour.source_global_records() != item.actor_modifiers() {
                return Err(unsupported(
                    "armour global record projection differs from selected preparation",
                ));
            }
            prepared_armour.insert(slot.clone(), armour);
        }
    }
    // PoB constructs one local actor layer: configuration, source slot order, passives.
    let mut actor_records = actor_modifiers.records().to_vec();
    for slot in poe_optimizer_import::equipment::EQUIPMENT_SOURCE_ORDER {
        if let Some(item) = equipment.get(slot) {
            actor_records.extend_from_slice(
                prepared_armour
                    .get(slot)
                    .map_or(item.actor_modifiers(), |armour| armour.global_records()),
            );
        }
    }
    actor_records.extend(resolved_tree.actor_modifiers().cloned());
    let actor_layers = vec![actor_records];
    let prepared_actor = data
        .prepare_actor_with_armour(
            level,
            actor_quests,
            data.receiving_scenario(quests, penalty),
            &resolved_tree.character,
            &actor_layers,
            poe_optimizer_engine::armour::ArmourSlots {
                helmet: prepared_armour.get("Helmet"),
                gloves: prepared_armour.get("Gloves"),
                boots: prepared_armour.get("Boots"),
                body_armour: prepared_armour.get("Body Armour"),
            },
        )
        .map_err(|error| unsupported(error.to_string()))?;
    let prepared_weapon = weapon
        .as_ref()
        .map(|record| {
            data.prepare_mace_weapon(
                weapon_slot(record.weapon_key())?,
                record.quality(),
                record.item_level(),
                record.local_modifiers(),
            )
            .map_err(|error| unsupported(error.to_string()))
        })
        .transpose()?;
    Ok(ProfileProjection::Complete(Profile {
        input,
        support_keys,
        support_order,
        prepared_supports,
        weapon_record: weapon,
        prepared_weapon,
        actor_modifiers,
        equipment,
        actor_quests,
        prepared_actor,
        prepared_armour,
        tree: resolved_tree,
        enemy_level,
        config,
        authored_config: original,
        export_xml,
        group_label: skill.attribute("label").map(str::to_owned),
    }))
}

fn validate_gem(
    node: Node<'_, '_>,
    name: &str,
    skill_id: &str,
    game_id: &str,
    variant_id: &str,
) -> Result<(), EvaluationError> {
    only(
        node,
        &[
            "nameSpec",
            "skillId",
            "gemId",
            "variantId",
            "level",
            "quality",
            "enabled",
            "enableGlobal1",
            "enableGlobal2",
            "count",
        ],
        &[],
    )?;
    fixed(
        node,
        &[
            ("nameSpec", name),
            ("skillId", skill_id),
            ("gemId", game_id),
            ("variantId", variant_id),
            ("level", "1"),
            ("quality", "0"),
            ("enabled", "true"),
            ("enableGlobal1", "true"),
            ("enableGlobal2", "true"),
            ("count", "1"),
        ],
    )
}

fn weapon_slot(key: &str) -> Result<MaceWeapon, EvaluationError> {
    match key {
        "wooden_club" => Ok(MaceWeapon::WoodenClub),
        "smithing_hammer" => Ok(MaceWeapon::SmithingHammer),
        _ => Err(unsupported("Unknown native Mace weapon capability slot")),
    }
}
/// Copy each untouched source span once, including potentially large trailing Notes.
/// All edit spans refer to the original XML and must be disjoint UTF-8 boundaries.
fn apply_source_edits(
    source: &str,
    mut edits: Vec<(std::ops::Range<usize>, String)>,
) -> Result<String, EvaluationError> {
    let invalid = || {
        EvaluationError::new(
            EvaluationErrorKind::BackendContract,
            "Native export edits contain invalid, overlapping or overflowing source spans",
        )
    };
    edits.sort_unstable_by_key(|(range, _)| range.start);
    let mut length = source.len();
    let mut previous_end = 0;
    for (range, value) in &edits {
        if range.start < previous_end || source.get(range.clone()).is_none() {
            return Err(invalid());
        }
        length = length
            .checked_sub(range.len())
            .and_then(|length| length.checked_add(value.len()))
            .ok_or_else(invalid)?;
        previous_end = range.end;
    }
    if length > poe_optimizer_import::MAX_XML_BYTES {
        return Err(unsupported("Native export exceeds XML byte limit"));
    }
    let mut exported = String::with_capacity(length);
    let mut cursor = 0;
    for (range, value) in edits {
        exported.push_str(&source[cursor..range.start]);
        exported.push_str(&value);
        cursor = range.end;
    }
    exported.push_str(&source[cursor..]);
    debug_assert_eq!(exported.len(), length);
    Ok(exported)
}

#[cfg(test)]
mod scenario_tests {
    use super::*;
    use poe_optimizer_core::{
        build_identity::BuildLineage,
        build_view::{SelectionRequest, ViewRequest, WeaponStateRequest},
    };
    use poe_optimizer_import::{
        build_instance::{ImportedBuildInstance, InstanceImportLimits},
        selected_view::resolve_view,
    };

    fn owner(request: &EvaluationRequest) -> Result<ImportedBuildInstance, EvaluationError> {
        let decoded = poe_optimizer_import::decode_build(request.build.content.as_bytes())
            .map_err(|error| {
                EvaluationError::new(EvaluationErrorKind::InvalidRequest, error.to_string())
            })?;
        ImportedBuildInstance::from_decoded(
            decoded,
            BuildLineage::from_bytes([47; 16]),
            InstanceImportLimits::default(),
        )
        .map_err(|error| {
            EvaluationError::new(EvaluationErrorKind::InvalidRequest, error.to_string())
        })
    }
    fn with_saved_view<T>(
        request: &EvaluationRequest,
        data: &crate::CompiledGameData,
        operation: impl FnOnce(&SelectedView<'_>) -> Result<T, EvaluationError>,
    ) -> Result<T, EvaluationError> {
        let owner = owner(request)?;
        let view = resolve_view(
            &owner,
            data.snapshot(),
            &ViewRequest::default(),
            Default::default(),
        )
        .map_err(|error| unsupported(error.to_string()))?;
        operation(&view)
    }
    fn request(source: &str) -> EvaluationRequest {
        EvaluationRequest {
            build: BuildDocument {
                format: BuildFormat::PathOfBuilding2Xml,
                content: source.into(),
            },
            options: EvaluationOptions::default(),
            metrics: vec![],
        }
    }

    const SPARK: &str = include_str!("../../../tests/fixtures/builds/spark-passive-equipment.xml");
    const MACE: &str = include_str!("../../../tests/fixtures/builds/mace-passive-equipment.xml");
    #[test]
    fn fixed_scenario_accepts_same_inert_root_source_as_full_profile() {
        let backend = crate::NativeBackend::new();
        let source = MACE.replace("</PathOfBuilding2>", "<Party/><Import exportParty=\"false\"/><TreeView/><Calcs><Input name=\"skill_number\" number=\"14\"/><Input name=\"misc_buffMode\" string=\"BUFFED\"/></Calcs></PathOfBuilding2>");
        let request = EvaluationRequest {
            build: BuildDocument {
                format: BuildFormat::PathOfBuilding2Xml,
                content: source.clone(),
            },
            options: EvaluationOptions::default(),
            metrics: vec![],
        };
        let full = with_saved_view(&request, backend.data(), |view| {
            parse(&request, backend.data(), view)
        })
        .unwrap();
        with_saved_view(&request, backend.data(), |view| {
            prepare_scenario(&request, backend.data(), view)
        })
        .unwrap();
        assert_eq!(full.export_xml, source);
    }
    #[test]
    fn fixed_scenario_and_full_projection_reject_the_same_unsupported_source() {
        let data = crate::NativeBackend::new();
        for source in [
            MACE.replace(
                "name=\"enemyCritChance\" number=\"0\"",
                "name=\"enemyCritChance\" number=\"1\"",
            ),
            MACE.replace("<Gem nameSpec=", "<Gem unsupported=\"ignored\" nameSpec="),
            MACE.replace("<PathOfBuilding2>", "<PathOfBuilding2 xmlns=\"foreign\">"),
            MACE.replace(
                "</PathOfBuilding2>",
                "<Party><ImportedBuffs/></Party></PathOfBuilding2>",
            ),
        ] {
            let request = EvaluationRequest {
                build: BuildDocument {
                    format: BuildFormat::PathOfBuilding2Xml,
                    content: source,
                },
                options: EvaluationOptions::default(),
                metrics: vec![],
            };
            let full = with_saved_view(&request, data.data(), |view| {
                parse(&request, data.data(), view)
            })
            .err()
            .expect("invalid full source");
            let scenario = with_saved_view(&request, data.data(), |view| {
                prepare_scenario(&request, data.data(), view)
            })
            .err()
            .expect("invalid scenario source");
            assert_eq!(scenario.kind, full.kind);
            assert_eq!(scenario.message, full.message);
        }
    }

    #[test]
    fn full_and_scenario_consume_the_same_selected_profile_inputs() {
        let backend = crate::NativeBackend::new();
        for source in [SPARK, MACE] {
            let request = request(source);
            let owner = owner(&request).unwrap();
            let mut explicit = ViewRequest::default();
            for binding in owner.instances() {
                match binding.instance() {
                    AuthoredInstanceId::SkillSet(id) => {
                        explicit.skills = SelectionRequest::Instance(id)
                    }
                    AuthoredInstanceId::ItemSet(id) => {
                        explicit.items = SelectionRequest::Instance(id)
                    }
                    AuthoredInstanceId::PassiveSpec(id) => {
                        explicit.passives = SelectionRequest::Instance(id)
                    }
                    AuthoredInstanceId::ConfigSet(id) => {
                        explicit.configuration = SelectionRequest::Instance(id)
                    }
                    _ => {}
                }
            }
            let view = resolve_view(
                &owner,
                backend.data().snapshot(),
                &explicit,
                Default::default(),
            )
            .unwrap();
            let full = parse(&request, backend.data(), &view).unwrap();
            let scenario = prepare_scenario(&request, backend.data(), &view).unwrap();
            match (full.input, scenario.input) {
                (NativeInput::Spark(full), NativeInput::Spark(scenario)) => {
                    assert_eq!(full, scenario)
                }
                (NativeInput::Mace(full), NativeInput::Mace(scenario)) => {
                    assert_eq!(full, scenario)
                }
                _ => panic!("selected full/scenario profiles disagree"),
            }
            assert_eq!(full.config, scenario.config);
            assert_eq!(full.actor_quests, scenario.actor_quests);
            assert_eq!(full.export_xml, source);
        }
    }

    #[test]
    fn selected_profile_rejects_mismatched_source_and_definition_owner() {
        let backend = crate::NativeBackend::new();
        let mut request = request(MACE);
        let owner = owner(&request).unwrap();
        let view = resolve_view(
            &owner,
            backend.data().snapshot(),
            &Default::default(),
            Default::default(),
        )
        .unwrap();
        request.build.content.push(' ');
        let error = parse(&request, backend.data(), &view).err().unwrap();
        assert_eq!(error.kind, EvaluationErrorKind::BackendContract);
        assert!(error.message.contains("XML differs"));
        request.build.content.pop();
        let cloned_snapshot = backend.data().snapshot().clone();
        let foreign = resolve_view(
            &owner,
            &cloned_snapshot,
            &Default::default(),
            Default::default(),
        )
        .unwrap();
        let error = parse(&request, backend.data(), &foreign).err().unwrap();
        assert_eq!(error.kind, EvaluationErrorKind::BackendContract);
    }

    #[test]
    fn explicit_secondary_weapon_state_cannot_evaluate_primary_source() {
        let backend = crate::NativeBackend::new();
        let request = request(MACE);
        let owner = owner(&request).unwrap();
        let selection = ViewRequest {
            weapon_state: WeaponStateRequest::Secondary,
            ..Default::default()
        };
        let view = resolve_view(
            &owner,
            backend.data().snapshot(),
            &selection,
            Default::default(),
        )
        .unwrap();
        for error in [
            parse(&request, backend.data(), &view).err().unwrap(),
            prepare_scenario(&request, backend.data(), &view)
                .err()
                .unwrap(),
        ] {
            assert_eq!(error.kind, EvaluationErrorKind::UnsupportedCapability);
            assert!(error.message.contains("first selected weapon set"));
        }
    }

    #[test]
    fn selected_views_do_not_broaden_closed_source_admission() {
        let backend = crate::NativeBackend::new();
        let source = MACE.replace("</Skills>", "<SkillSet id=\"2\"/></Skills>");
        let request = request(&source);
        with_saved_view(&request, backend.data(), |view| {
            assert!(view.report().skills.selected.is_some());
            let error = parse(&request, backend.data(), view).err().unwrap();
            assert_eq!(error.kind, EvaluationErrorKind::UnsupportedCapability);
            assert!(error.message.contains("exactly one SkillSet"));
            Ok(())
        })
        .unwrap();
        let source = MACE.replace("activeSpec=\"1\"", "activeSpec=\"0\"");
        let request = self::request(&source);
        let owner = owner(&request).unwrap();
        let spec = owner
            .instances()
            .iter()
            .find_map(|binding| match binding.instance() {
                AuthoredInstanceId::PassiveSpec(id) => Some(id),
                _ => None,
            })
            .unwrap();
        let selection = ViewRequest {
            passives: SelectionRequest::Instance(spec),
            ..Default::default()
        };
        let view = resolve_view(
            &owner,
            backend.data().snapshot(),
            &selection,
            Default::default(),
        )
        .unwrap();
        assert!(view.report().passives.selected.is_some());
        let error = parse(&request, backend.data(), &view).err().unwrap();
        assert_eq!(error.kind, EvaluationErrorKind::UnsupportedCapability);
    }
}
