//! Source-preserving, bounded weapon/support mutations for a structural Mace profile.
//! Pure Rust source projection shared by native and optional reference evaluators.
//! Experimental diagnostic domain, not a general build-legality claim.
use poe_optimizer_core::{
    candidate::*,
    coverage::{SkillActor, SkillOrigin, SkillResolution},
    data::DataIdentity,
    evaluation::{BackendIdentity, BuildDocument, BuildFormat, EvaluationResult},
    options::{EvaluationContext, EvaluationOptions, Scalar},
};
use poe_optimizer_data::class_tree::{self, ClassTreeSelection, ResolvedClassTree};
use poe_optimizer_data::game_data::{
    self, GameDataPackage, GameDataSnapshot, RequirementData, SupportColor,
};
use roxmltree::{Document, Node, ParsingOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::Arc;
use thiserror::Error;
/// Rules/data pin for this finite format projection; not a runtime dependency.
pub const PINNED_RULES_REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const SLOT: &str = "pob-group-1";
const SOURCE: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalMaceAlternative {
    pub id: String,
    pub item_text: String,
}
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaceSupportChoice {
    None,
    BrutalityI,
}
#[derive(Debug, Clone, Serialize)]
pub struct ControlledMaceAlternative {
    pub id: String,
    pub weapon_id: String,
    pub support: MaceSupportChoice,
    pub tree: ClassTreeSelection,
    pub xml_sha256: String,
    pub candidate: Candidate,
}
/// Closed-profile requirement evidence, independent of diagnostic numerical evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MaceRequirementValues {
    pub level: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
}
impl MaceRequirementValues {
    fn include(&mut self, other: &RequirementData) {
        self.level = self.level.max(other.level);
        self.strength = self.strength.max(other.attributes.strength);
        self.dexterity = self.dexterity.max(other.attributes.dexterity);
        self.intelligence = self.intelligence.max(other.attributes.intelligence);
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaceRequirementViolation {
    pub requirement: String,
    pub required: u32,
    pub available: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaceRequirementAssessment {
    pub available: MaceRequirementValues,
    pub required: MaceRequirementValues,
    pub violations: Vec<MaceRequirementViolation>,
}
impl MaceRequirementAssessment {
    pub fn is_legal(&self) -> bool {
        self.violations.is_empty()
    }
}
#[derive(Debug, Error)]
pub enum ControlledMutationError {
    #[error("unsupported controlled Mace input: {0}")]
    Unsupported(String),
    #[error("candidate is not in this immutable controlled Mace domain")]
    UnknownCandidate,
    #[error("controlled Mace realization mismatch: {0}")]
    Realization(String),
}
type Result<T> = std::result::Result<T, ControlledMutationError>;
#[derive(Debug)]
struct Profile {
    level: u32,
    config: BTreeMap<String, Scalar>,
    item_range: Range<usize>,
    support_range: Range<usize>,
    weapon: String,
    active_attributes: BTreeMap<String, String>,
    has_support: bool,
    tree: Arc<ResolvedClassTree>,
    tree_attribute_ranges: BTreeMap<String, Range<usize>>,
    ascendancy_insert: usize,
}
#[derive(Debug, Clone)]
struct Choice {
    weapon: String,
    support: MaceSupportChoice,
    tree: Arc<ResolvedClassTree>,
    patch_tree: bool,
}
/// Fresh validated template evidence; baseline-derived, not an independent golden.
#[derive(Debug)]
pub struct VerifiedMaceScenario {
    identity: CatalogIdentity,
    context: EvaluationContext,
    backend: BackendIdentity,
    exported_scenario: String,
}
/// Fresh native template evidence, bound to caller-authenticated backend identity.
/// This establishes this finite source projection, not complete game legality.
#[derive(Debug)]
pub struct VerifiedNativeMaceScenario {
    identity: CatalogIdentity,
    backend: BackendIdentity,
    context: EvaluationContext,
}
#[derive(Debug)]
pub struct ControlledMaceCatalog {
    data: Arc<GameDataSnapshot>,
    support_xml: String,
    template: String,
    profile: Profile,
    catalog: CandidateCatalog,
    alternatives: Vec<ControlledMaceAlternative>,
    choices: BTreeMap<Candidate, Choice>,
    tree_choices: Vec<ClassTreeSelection>,
    candidate_index:
        BTreeMap<ClassTreeSelection, BTreeMap<String, BTreeMap<MaceSupportChoice, usize>>>,
}
impl ControlledMaceCatalog {
    pub fn new(
        template_xml: String,
        weapons: Vec<NormalMaceAlternative>,
        supports: Vec<MaceSupportChoice>,
    ) -> Result<Self> {
        let data =
            game_data::bundled_snapshot().map_err(|error| unsupported(&error.to_string()))?;
        Self::with_data(Arc::new(data), template_xml, weapons, supports)
    }
    /// Retain one immutable selected dataset for materialization, identity and legality.
    /// The admitted structure remains Warrior without an ascendancy or paid passives.
    pub fn with_data(
        data: Arc<GameDataSnapshot>,
        template_xml: String,
        weapons: Vec<NormalMaceAlternative>,
        supports: Vec<MaceSupportChoice>,
    ) -> Result<Self> {
        Self::build(data, template_xml, weapons, supports, None)
    }
    /// Compose finite class/ascendancy/ordinary-entrance/ascendancy-passive choices with equipment and support.
    /// The snapshot is shared with requirements and evaluator admission; no Lua runs here.
    pub fn with_tree_choices(
        data: Arc<GameDataSnapshot>,
        template_xml: String,
        weapons: Vec<NormalMaceAlternative>,
        supports: Vec<MaceSupportChoice>,
        tree_choices: Vec<ClassTreeSelection>,
    ) -> Result<Self> {
        Self::build(data, template_xml, weapons, supports, Some(tree_choices))
    }
    fn build(
        data: Arc<GameDataSnapshot>,
        template_xml: String,
        weapons: Vec<NormalMaceAlternative>,
        supports: Vec<MaceSupportChoice>,
        requested_trees: Option<Vec<ClassTreeSelection>>,
    ) -> Result<Self> {
        let package = data.package();
        if package
            .quests
            .config_keys
            .iter()
            .any(|key| INPUT_NAMES.contains(&key.as_str()) || key == "resistancePenalty")
        {
            return Err(unsupported(
                "selected quest keys overlap controlled encounter configuration",
            ));
        }
        let profile = profile(&template_xml, package)?;
        let support_xml = support_xml(package);
        let patch_tree = requested_trees.is_some();
        if !patch_tree && profile.tree.selection != fixed_warrior() {
            return Err(unsupported(
                "fixed catalog requires Warrior without ascendancy or paid passives",
            ));
        }
        let mut tree_choices = requested_trees.unwrap_or_else(|| vec![fixed_warrior()]);
        if tree_choices.is_empty() || tree_choices.len() > 105 {
            return Err(unsupported("provide 1..105 distinct tree choices"));
        }
        tree_choices.sort();
        if tree_choices.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(unsupported("duplicate tree alternatives"));
        }
        let resolved_trees = tree_choices
            .iter()
            .map(|selection| {
                selection
                    .resolve(&package.tree)
                    .map(Arc::new)
                    .map_err(|error| unsupported(&error.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        if weapons.is_empty() || weapons.len() > 64 || supports.is_empty() || supports.len() > 2 {
            return Err(unsupported(
                "provide 1..64 weapons and 1..2 distinct support choices",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut payloads = BTreeSet::new();
        let mut parsed = Vec::new();
        for weapon in weapons {
            if weapon.id.trim().is_empty()
                || weapon.id.len() > 128
                || !ids.insert(weapon.id.clone())
            {
                return Err(unsupported(
                    "weapon IDs must be distinct nonblank strings of at most 128 bytes",
                ));
            }
            let text = normal_mace(&weapon.item_text, package)?;
            if !payloads.insert(text.clone()) {
                return Err(unsupported("duplicate exact weapon alternatives"));
            }
            parsed.push((weapon.id, text));
        }
        let support_set: BTreeSet<_> = supports.iter().copied().collect();
        if support_set.len() != supports.len() {
            return Err(unsupported("duplicate support alternatives"));
        }
        parsed.sort();
        let preparations = parsed
            .len()
            .checked_mul(support_set.len())
            .and_then(|count| count.checked_mul(tree_choices.len()))
            .and_then(|count| count.checked_mul(template_xml.len().saturating_add(4096)));
        if preparations.is_none_or(|bytes| bytes > 256 * 1024 * 1024) {
            return Err(unsupported(
                "controlled materialization hashing exceeds 256 MiB preparation budget",
            ));
        }
        let graph = class_tree::candidate_catalog(&data)
            .map_err(|error| unsupported(&error.to_string()))?;
        let identity = CatalogIdentity {
            schema_version: 1,
            game: "path_of_exile_2".into(),
            rules_revision: PINNED_RULES_REVISION.into(),
            content_fingerprint: hash(
                &if patch_tree {
                    serde_json::to_string(&(
                        "pob-controlled-mace-v4",
                        data.identity(),
                        SOURCE,
                        hash(&template_xml),
                        &parsed,
                        &support_set,
                        &tree_choices,
                        &graph.identity,
                    ))
                } else {
                    serde_json::to_string(&(
                        "pob-controlled-mace-v2",
                        data.identity(),
                        SOURCE,
                        hash(&template_xml),
                        &parsed,
                        &support_set,
                    ))
                }
                .map_err(|error| unsupported(&error.to_string()))?,
            ),
        };
        let active_payload = payload(
            "pob2-gem-attributes-json-v1",
            serde_json::to_string(&profile.active_attributes).unwrap(),
        );
        let active_id = format!("pob-gem:{}", active_payload.sha256);
        let support_document =
            Document::parse(&support_xml).map_err(|error| unsupported(&error.to_string()))?;
        let support_payload = payload(
            "pob2-gem-attributes-json-v1",
            serde_json::to_string(&attributes(support_document.root_element())).unwrap(),
        );
        let support_id = format!("pob-gem:{}", support_payload.sha256);
        let legacy_root = profile.tree.class.start_node_id;
        let mut catalog = CandidateCatalog {
            identity: identity.clone(),
            classes: if patch_tree {
                graph.classes
            } else {
                BTreeMap::from([(
                    "6".into(),
                    ClassDefinition {
                        start_node_id: legacy_root,
                    },
                )])
            },
            ascendancies: if patch_tree {
                graph.ascendancies
            } else {
                BTreeMap::new()
            },
            passive_nodes: if patch_tree {
                graph.passive_nodes
            } else {
                BTreeMap::from([(
                    legacy_root,
                    PassiveNode {
                        kind: PassiveKind::ClassStart {
                            class_ids: BTreeSet::from(["6".into()]),
                        },
                        ..Default::default()
                    },
                )])
            },
            equipment_slots: BTreeSet::from(["Weapon 1".into()]),
            skill_slots: BTreeSet::from([SLOT.into()]),
            items: BTreeMap::new(),
            active_skills: BTreeMap::from([(
                active_id.clone(),
                ActiveSkillInstance {
                    definition_id: package.mace.skill_id.clone(),
                    payload: active_payload,
                    ..Default::default()
                },
            )]),
            supports: BTreeMap::new(),
            support_definition_limits: BTreeMap::new(),
            unsupported_mechanics: BTreeSet::new(),
        };
        if support_set.contains(&MaceSupportChoice::BrutalityI) {
            catalog.supports.insert(
                support_id.clone(),
                SupportInstance {
                    definition_id: package.mace.brutality.skill_id.clone(),
                    payload: support_payload,
                    compatible_active_skill_ids: BTreeSet::from([package.mace.skill_id.clone()]),
                    ..Default::default()
                },
            );
            catalog
                .support_definition_limits
                .insert(package.mace.brutality.skill_id.clone(), 1);
        }
        let mut choices = BTreeMap::new();
        let mut alternatives = Vec::new();
        let mut hashed_bytes = 0usize;
        let mut candidate_index: BTreeMap<
            ClassTreeSelection,
            BTreeMap<String, BTreeMap<MaceSupportChoice, usize>>,
        > = BTreeMap::new();
        for (id, weapon) in parsed {
            let item_payload = payload("pob2-item-text-v1", weapon.clone());
            let item_id = format!("pob-item-1:{}", item_payload.sha256);
            catalog.items.insert(
                item_id.clone(),
                ItemInstance {
                    definition_id: weapon.lines().nth(1).unwrap().into(),
                    payload: item_payload,
                    compatible_slots: BTreeSet::from(["Weapon 1".into()]),
                    ..Default::default()
                },
            );
            for support in &support_set {
                for tree in &resolved_trees {
                    let candidate = Candidate {
                        catalog: identity.clone(),
                        class_id: tree.class.integer_id.to_string(),
                        ascendancy_id: tree.ascendancy.as_ref().map(|asc| asc.internal_id.clone()),
                        passives: tree
                            .selection
                            .entrance_node_id
                            .into_iter()
                            .chain(tree.selection.ascendancy_node_id)
                            .collect(),
                        equipment: BTreeMap::from([("Weapon 1".into(), item_id.clone())]),
                        skills: BTreeMap::from([(
                            SLOT.into(),
                            SkillAssignment {
                                active_instance_id: active_id.clone(),
                                support_instance_ids: if *support == MaceSupportChoice::None {
                                    BTreeSet::new()
                                } else {
                                    BTreeSet::from([support_id.clone()])
                                },
                            },
                        )]),
                    };
                    let choice = Choice {
                        weapon: weapon.clone(),
                        support: *support,
                        tree: tree.clone(),
                        patch_tree,
                    };
                    let xml = patch(&template_xml, &profile, &choice, &support_xml);
                    hashed_bytes = hashed_bytes.checked_add(xml.len()).ok_or_else(|| {
                        unsupported(
                            "controlled materialization hashing exceeds 256 MiB preparation budget",
                        )
                    })?;
                    if hashed_bytes > 256 * 1024 * 1024 {
                        return Err(unsupported(
                            "controlled materialization hashing exceeds 256 MiB preparation budget",
                        ));
                    }
                    candidate_index
                        .entry(tree.selection.clone())
                        .or_default()
                        .entry(id.clone())
                        .or_default()
                        .insert(*support, alternatives.len());
                    alternatives.push(ControlledMaceAlternative {
                        id: format!(
                            "{}{id}/{}",
                            if patch_tree {
                                format!(
                                    "class/{}/asc/{}/entrance/{}/{ascendancy_passive}",
                                    tree.selection.class_id,
                                    tree.selection.ascendancy_id.as_deref().unwrap_or("none"),
                                    tree.selection
                                        .entrance_node_id
                                        .map_or_else(|| "none".into(), |id| id.to_string()),
                                    ascendancy_passive = tree
                                        .selection
                                        .ascendancy_node_id
                                        .map_or_else(String::new, |id| format!(
                                            "ascendancy-passive/{id}/"
                                        ))
                                )
                            } else {
                                String::new()
                            },
                            if *support == MaceSupportChoice::None {
                                "none"
                            } else {
                                "brutality_i"
                            }
                        ),
                        weapon_id: id.clone(),
                        support: *support,
                        tree: tree.selection.clone(),
                        xml_sha256: hash(&xml),
                        candidate: candidate.clone(),
                    });
                    choices.insert(candidate, choice);
                }
            }
        }
        Ok(Self {
            data,
            support_xml,
            template: template_xml,
            profile,
            catalog,
            alternatives,
            choices,
            tree_choices,
            candidate_index,
        })
    }
    pub fn snapshot(&self) -> &Arc<GameDataSnapshot> {
        &self.data
    }
    pub fn data_identity(&self) -> &DataIdentity {
        self.data.identity()
    }
    pub fn required_skill_id(&self) -> &str {
        &self.data.package().mace.skill_id
    }
    pub fn catalog(&self) -> &CandidateCatalog {
        &self.catalog
    }
    pub fn alternatives(&self) -> &[ControlledMaceAlternative] {
        &self.alternatives
    }
    pub fn tree_choices(&self) -> &[ClassTreeSelection] {
        &self.tree_choices
    }
    pub fn resolve_tree_candidate(
        &self,
        tree: &ClassTreeSelection,
        weapon_id: &str,
        support: MaceSupportChoice,
    ) -> Option<&Candidate> {
        let index = self
            .candidate_index
            .get(tree)?
            .get(weapon_id)?
            .get(&support)?;
        self.alternatives.get(*index).map(|value| &value.candidate)
    }
    /// Resolve a weapon/support choice under the imported template's tree identity.
    pub fn resolve_candidate(
        &self,
        weapon_id: &str,
        support: MaceSupportChoice,
    ) -> Option<&Candidate> {
        self.resolve_tree_candidate(&self.profile.tree.selection, weapon_id, support)
    }
    /// Admitted passive records do not change attributes; equipment dependencies are excluded.
    /// Aggregate support-color costs compete with individual requirements by maximum;
    /// they are never added to weapon or active-gem requirements.
    pub fn requirements(&self, candidate: &Candidate) -> Result<MaceRequirementAssessment> {
        let choice = self
            .choices
            .get(candidate)
            .ok_or(ControlledMutationError::UnknownCandidate)?;
        let data = self.data.package();
        let class = &choice.tree.class;
        let available = MaceRequirementValues {
            level: self.profile.level,
            strength: class.base_strength,
            dexterity: class.base_dexterity,
            intelligence: class.base_intelligence,
        };
        let mut required = MaceRequirementValues {
            level: 0,
            strength: 0,
            dexterity: 0,
            intelligence: 0,
        };
        let name = choice.weapon.lines().nth(1).expect("validated weapon name");
        let weapon = data
            .weapons
            .iter()
            .find(|weapon| weapon.name == name)
            .expect("validated selected weapon");
        required.include(&weapon.requirements);
        required.include(&data.mace.requirements);
        if choice.support == MaceSupportChoice::BrutalityI {
            required.include(&data.mace.brutality.requirements);
            // Exactly one support socket is admitted by this profile.
            let costs = &data.mace.support_attribute_costs;
            match data.mace.brutality.color {
                SupportColor::Red => required.strength = required.strength.max(costs.strength),
                SupportColor::Green => required.dexterity = required.dexterity.max(costs.dexterity),
                SupportColor::Blue => {
                    required.intelligence = required.intelligence.max(costs.intelligence)
                }
            }
        }
        let mut violations = Vec::new();
        for (name, required, available) in [
            ("level", required.level, available.level),
            ("strength", required.strength, available.strength),
            ("dexterity", required.dexterity, available.dexterity),
            (
                "intelligence",
                required.intelligence,
                available.intelligence,
            ),
        ] {
            if required > available {
                violations.push(MaceRequirementViolation {
                    requirement: name.into(),
                    required,
                    available,
                });
            }
        }
        Ok(MaceRequirementAssessment {
            available,
            required,
            violations,
        })
    }
    pub fn validate_requirements(&self, candidate: &Candidate) -> Result<()> {
        let assessment = self.requirements(candidate)?;
        if assessment.is_legal() {
            return Ok(());
        }
        let reasons = assessment
            .violations
            .iter()
            .map(|violation| {
                format!(
                    "{} requires {}, available {}",
                    violation.requirement, violation.required, violation.available
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        Err(unsupported(&format!(
            "controlled candidate requirements not met: {reasons}"
        )))
    }
    pub fn template_build(&self) -> BuildDocument {
        BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: self.template.clone(),
        }
    }
    pub fn materialize(&self, candidate: &Candidate) -> Result<BuildDocument> {
        let choice = self
            .choices
            .get(candidate)
            .ok_or(ControlledMutationError::UnknownCandidate)?;
        Ok(BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: patch(&self.template, &self.profile, choice, &self.support_xml),
        })
    }
    /// Caller budgets this fresh template attempt; it does not replace final verification.
    pub fn bind_baseline(&self, result: &EvaluationResult) -> Result<VerifiedMaceScenario> {
        let choice = Choice {
            weapon: self.profile.weapon.clone(),
            tree: self.profile.tree.clone(),
            patch_tree: false,
            support: if self.profile.has_support {
                MaceSupportChoice::BrutalityI
            } else {
                MaceSupportChoice::None
            },
        };
        self.check_realization(&choice, result)?;
        Ok(VerifiedMaceScenario {
            identity: self.catalog.identity.clone(),
            context: result.context.clone(),
            backend: result.backend.clone(),
            exported_scenario: exported_scenario(
                &result.exports[0].content,
                &self.data.package().mace.brutality.skill_id,
            )?,
        })
    }
    pub fn validate_realization(
        &self,
        candidate: &Candidate,
        result: &EvaluationResult,
        scenario: &VerifiedMaceScenario,
    ) -> Result<()> {
        let choice = self
            .choices
            .get(candidate)
            .ok_or(ControlledMutationError::UnknownCandidate)?;
        if scenario.identity != self.catalog.identity {
            return Err(mismatch("baseline belongs to another catalog"));
        }
        self.check_realization(choice, result)?;
        let expected = &scenario.context;
        let actual = &result.context;
        if result.backend != scenario.backend
            || actual.requested != expected.requested
            || actual.calculation_mode != expected.calculation_mode
            || actual.enemy_level != expected.enemy_level
            || actual.config_inputs != expected.config_inputs
            || actual.config_placeholders != expected.config_placeholders
        {
            return Err(mismatch(
                "effective immutable scenario changed from the verified template",
            ));
        }
        let signature = exported_scenario(
            &result.exports[0].content,
            &self.data.package().mace.brutality.skill_id,
        )?;
        if signature != scenario.exported_scenario {
            let at = signature
                .chars()
                .zip(scenario.exported_scenario.chars())
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            return Err(mismatch(&format!(
                "exported scenario changed at {at}: actual {:?}; expected {:?}",
                signature.chars().skip(at).take(160).collect::<String>(),
                scenario
                    .exported_scenario
                    .chars()
                    .skip(at)
                    .take(160)
                    .collect::<String>()
            )));
        }
        Ok(())
    }
    /// Validate a freshly evaluated native template without PoB-normalized export assumptions.
    /// `expected_backend` must come from the actual selected backend implementation, not the result.
    pub fn bind_native_baseline(
        &self,
        result: &EvaluationResult,
        expected_backend: &BackendIdentity,
    ) -> Result<VerifiedNativeMaceScenario> {
        if !same_backend(&result.backend, expected_backend) {
            return Err(mismatch(
                "native baseline backend identity differs from the selected implementation",
            ));
        }
        if expected_backend.data.as_ref() != Some(self.data.identity()) {
            return Err(mismatch(
                "native backend data differs from the selected controlled catalog",
            ));
        }
        let choice = Choice {
            weapon: self.profile.weapon.clone(),
            tree: self.profile.tree.clone(),
            patch_tree: false,
            support: if self.profile.has_support {
                MaceSupportChoice::BrutalityI
            } else {
                MaceSupportChoice::None
            },
        };
        self.check_native_realization(&choice, result, &self.template)?;
        Ok(VerifiedNativeMaceScenario {
            identity: self.catalog.identity.clone(),
            backend: expected_backend.clone(),
            context: result.context.clone(),
        })
    }
    /// Check the exact requested materialization, resolved item/skill evidence and fixed encounter.
    pub fn validate_native_realization(
        &self,
        candidate: &Candidate,
        result: &EvaluationResult,
        scenario: &VerifiedNativeMaceScenario,
    ) -> Result<()> {
        let choice = self
            .choices
            .get(candidate)
            .ok_or(ControlledMutationError::UnknownCandidate)?;
        if scenario.identity != self.catalog.identity
            || !same_backend(&result.backend, &scenario.backend)
        {
            return Err(mismatch(
                "native scenario belongs to another catalog or backend",
            ));
        }
        self.check_native_realization(choice, result, &self.materialize(candidate)?.content)?;
        let expected = &scenario.context;
        let actual = &result.context;
        // Conditions may derive from the selected candidate; only external encounter inputs are frozen.
        if actual.requested != expected.requested
            || actual.calculation_mode != expected.calculation_mode
            || actual.enemy_level != expected.enemy_level
            || actual.config_inputs != expected.config_inputs
            || actual.config_placeholders != expected.config_placeholders
        {
            return Err(mismatch("native immutable configured scenario changed"));
        }
        Ok(())
    }
    fn check_native_realization(
        &self,
        choice: &Choice,
        result: &EvaluationResult,
        expected_xml: &str,
    ) -> Result<()> {
        result
            .validate_recorded()
            .map_err(|error| mismatch(&error.to_string()))?;
        if result.backend.id != "native-poe2"
            || result.backend.rules_revision != PINNED_RULES_REVISION
            || result.backend.data.as_ref() != Some(self.data.identity())
            || !result.diagnostic_only
        {
            return Err(mismatch(
                "wrong native backend identity or missing diagnostic marker",
            ));
        }
        self.check_common_realization(choice, result)?;
        if result.context.config_inputs != self.profile.config {
            return Err(mismatch(
                "native configured inputs differ from the exact source profile",
            ));
        }
        let mut expected_defaults: BTreeMap<String, Scalar> = self
            .data
            .package()
            .quests
            .config_keys
            .iter()
            .zip(self.data.package().quests.default_enabled)
            .filter(|(name, _)| !self.profile.config.contains_key(*name))
            .map(|(name, enabled)| (name.clone(), Scalar::Boolean(enabled)))
            .collect();
        if !self.profile.config.contains_key("resistancePenalty") {
            expected_defaults.insert(
                "resistancePenalty".into(),
                Scalar::Number(self.data.package().encounters.default_resistance_penalty),
            );
        }
        if result.context.config_placeholders != expected_defaults {
            return Err(mismatch(
                "native quest or resistance defaults differ from selected data",
            ));
        }
        if result.exports.len() != 1
            || result.exports[0].format != BuildFormat::PathOfBuilding2Xml
            || result.exports[0].content != expected_xml
        {
            return Err(mismatch(
                "native export must match the exact materialized source XML",
            ));
        }
        let evidence: Vec<_> = result
            .attachments
            .iter()
            .filter(|attachment| {
                attachment.media_type
                    == "application/vnd.poe-optimizer.native-profile+json;version=1"
            })
            .collect();
        if evidence.len() != 1 || evidence[0].content.len() > 64 * 1024 {
            return Err(mismatch(
                "one bounded native resolved-profile attachment is required",
            ));
        }
        let evidence: serde_json::Value = serde_json::from_str(&evidence[0].content)
            .map_err(|error| mismatch(&error.to_string()))?;
        let lines: Vec<_> = choice.weapon.lines().collect();
        let item_level = lines[2]
            .strip_prefix("Item Level: ")
            .and_then(|text| text.parse::<u64>().ok())
            .ok_or_else(|| mismatch("invalid registered item level"))?;
        let quality = lines[3]
            .strip_prefix("Quality: ")
            .and_then(|text| text.parse::<u64>().ok())
            .ok_or_else(|| mismatch("invalid registered item quality"))?;
        if evidence["weapon_base"].as_str() != Some(lines[1])
            || evidence["weapon_quality"].as_u64() != Some(quality)
            || evidence["weapon_item_level"].as_u64() != Some(item_level)
            || evidence["brutality_i"].as_bool()
                != Some(choice.support == MaceSupportChoice::BrutalityI)
        {
            return Err(mismatch(
                "native resolved weapon or support differs from candidate payload",
            ));
        }
        self.check_native_tree(choice, result)?;
        Ok(())
    }
    fn check_native_tree(&self, choice: &Choice, result: &EvaluationResult) -> Result<()> {
        let attachments: Vec<_> = result
            .attachments
            .iter()
            .filter(|attachment| {
                attachment.media_type == "application/vnd.poe-optimizer.native-tree+json;version=2"
            })
            .collect();
        if attachments.len() != 1 || attachments[0].content.len() > 64 * 1024 {
            return Err(mismatch(
                "one bounded native resolved-tree attachment is required",
            ));
        }
        let actual: serde_json::Value = serde_json::from_str(&attachments[0].content)
            .map_err(|error| mismatch(&error.to_string()))?;
        let tree = &choice.tree;
        let ascendancy = tree.ascendancy.as_ref().map(|asc| {
            serde_json::json!({
                "index":asc.class_index,"internal_id":asc.internal_id,"catalog_id":asc.catalog_id,
                "name":asc.name,"start_node_id":asc.start_node_id,
            })
        });
        let paid_nodes: Vec<_> = tree.paid_node.iter().map(|node| ("ordinary", node))
            .chain(tree.ascendancy_node.iter().map(|node| ("ascendancy", node)))
            .map(|(kind, node)| serde_json::json!({
            "allocation_kind":kind,
            "physical_node_id":node.physical_node_id,"effective_node_id":node.effective_source_id,
            "name":node.name,"stats":node.stats,"override_provenance":node.provenance,
        })).collect();
        let expected = serde_json::json!({
            "schema_version":2,
            "class":{"index":tree.class.integer_id,"internal_id":tree.class.integer_id,
                "source_index":tree.class.source_index,"name":tree.class.name,"start_node_id":tree.class.start_node_id},
            "ascendancy":ascendancy,"allocated_nodes":tree.allocated_nodes,
            "ordinary_allocated_count":usize::from(tree.paid_node.is_some()),
            "ascendancy_allocated_count":usize::from(tree.ascendancy_node.is_some()),"paid_nodes":paid_nodes,
            "source":{"upstream_revision":self.data.tree().source.upstream_revision,
                "tree_version":self.data.tree().source.tree_version,
                "bundled_content_sha256":poe_optimizer_data::bundled::content_sha256()},
            "data_identity":self.data.identity(),
            "configured_effects":self.data.package().passive_effects.iter().filter(|entry|
                entry.class_id == tree.class.integer_id && (
                    (entry.ascendancy_id.is_none() && tree.paid_node.as_ref().is_some_and(|node| entry.physical_node_id == node.physical_node_id)) ||
                    (entry.ascendancy_id.as_deref() == tree.selection.ascendancy_id.as_deref() && tree.ascendancy_node.as_ref().is_some_and(|node| entry.physical_node_id == node.physical_node_id))
                )).collect::<Vec<_>>(),
            "point_budget_verified":false,
            "evidence_kind":"native_source_resolution",
            "scope":"class_identity_and_zero_or_one_ordinary_and_ascendancy_passive",
        });
        if actual != expected {
            return Err(mismatch(
                "native tree resolution differs from selected class, ascendancy, allocation or configured effects",
            ));
        }
        Ok(())
    }
    fn check_common_realization(&self, choice: &Choice, result: &EvaluationResult) -> Result<()> {
        if result.context.requested != EvaluationOptions::default()
            || result.context.calculation_mode != "MAIN"
            || result.context.enemy_level
                != scalar_number(&self.profile.config["enemyLevel"]).unwrap() as u32
        {
            return Err(mismatch("encounter overrides or calculation mode changed"));
        }
        for (name, value) in &self.profile.config {
            if result.context.config_inputs.get(name) != Some(value) {
                return Err(mismatch(&format!("explicit configuration {name} changed")));
            }
        }
        let build = &result.build;
        if build.level != self.profile.level
            || build.class_name != choice.tree.class.name
            || build.ascendancy_name
                != choice
                    .tree
                    .ascendancy
                    .as_ref()
                    .map_or("None", |asc| asc.name.as_str())
            || build.tree_version != self.data.package().tree.source.tree_version
            || build.main_socket_group != 1
            || build.skill_groups != 1
            || build
                .allocated_nodes
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != choice.tree.allocated_nodes
            || build.allocated_nodes.len() != choice.tree.allocated_nodes.len()
        {
            return Err(mismatch("class, ascendancy, level, tree or group changed"));
        }
        let coverage = &result.coverage;
        let count = if choice.support == MaceSupportChoice::None {
            1
        } else {
            2
        };
        if coverage.active_skill_set_id != Some(1)
            || coverage.groups.len() != 1
            || coverage.unresolved_entry_count != 0
            || coverage.selected_minion.is_some()
        {
            return Err(mismatch(
                "unexpected skill set, unresolved or minion entries",
            ));
        }
        let selected = coverage
            .selected_player
            .as_ref()
            .ok_or_else(|| mismatch("missing selected player"))?;
        if selected.skill_id.as_deref() != Some(self.required_skill_id())
            || selected.actor != SkillActor::Player
            || selected.group_index != Some(1)
            || selected.gem_index != Some(1)
            || selected.actor_skill_index != Some(1)
            || selected.synthesized_default_attack
            || selected.minion_id.is_some()
            || selected.part_index.is_some()
            || selected.stat_set_index != Some(1)
            || selected.show_average
        {
            return Err(mismatch("selected action changed or fell back"));
        }
        let group = &coverage.groups[0];
        if group.index != 1
            || !group.enabled
            || !group.include_in_full_dps
            || group.group_count != Some(1.0)
            || group.main_active_skill != Some(1)
            || group.provenance.kind != SkillOrigin::Manual
            || group.slot.is_some()
            || group.provenance.source.is_some()
            || group.provenance.item_id.is_some()
            || group.provenance.node_id.is_some()
            || group.gems.len() != count
        {
            return Err(mismatch("manual skill group changed"));
        }
        for (index, gem) in group.gems.iter().enumerate() {
            let data = self.data.package();
            let (skill, game, variant) = if index == 0 {
                (
                    &data.mace.skill_id,
                    &data.mace.game_id,
                    &data.mace.variant_id,
                )
            } else {
                (
                    &data.mace.brutality.skill_id,
                    &data.mace.brutality.game_id,
                    &data.mace.brutality.variant_id,
                )
            };
            if gem.index != index + 1
                || !gem.enabled
                || gem.resolution != SkillResolution::ResolvedGem
                || gem.skill_id.as_deref() != Some(skill.as_str())
                || gem.gem_game_id.as_deref() != Some(game.as_str())
                || gem.variant_id.as_deref() != Some(variant.as_str())
                || gem.level != Some(1.0)
                || gem.quality != Some(0.0)
                || gem.count != Some(1.0)
                || gem.is_support != Some(index > 0)
            {
                return Err(mismatch("exact gem identity or configuration changed"));
            }
        }
        Ok(())
    }
    fn check_realization(&self, choice: &Choice, result: &EvaluationResult) -> Result<()> {
        result
            .validate_recorded()
            .map_err(|error| mismatch(&error.to_string()))?;
        if self.data.identity().content_sha256 != game_data::bundled_package_sha256() {
            return Err(mismatch(
                "PoB reference binding requires the reviewed default data content",
            ));
        }
        if result.backend.id != "pob-poe2-mlua"
            || result.backend.rules_revision != PINNED_RULES_REVISION
            || result.backend.source_fingerprint != SOURCE
            || !result.diagnostic_only
        {
            return Err(mismatch(
                "wrong backend identity or missing diagnostic marker",
            ));
        }
        self.check_common_realization(choice, result)?;
        if result.exports.len() != 1 || result.exports[0].format != BuildFormat::PathOfBuilding2Xml
        {
            return Err(mismatch("exactly one fresh PoB XML export is required"));
        }
        check_export(
            &result.exports[0].content,
            choice,
            self.profile.level,
            &result.context,
            self.data.package(),
        )?;
        check_template_frame(
            &patch(&self.template, &self.profile, choice, &self.support_xml),
            &result.exports[0].content,
        )?;
        Ok(())
    }
}
fn fixed_warrior() -> ClassTreeSelection {
    ClassTreeSelection {
        class_id: 6,
        ascendancy_id: None,
        entrance_node_id: None,
        ascendancy_node_id: None,
    }
}
fn same_backend(left: &BackendIdentity, right: &BackendIdentity) -> bool {
    left == right
}
fn profile(xml: &str, data: &GameDataPackage) -> Result<Profile> {
    crate::xml_compat::validate_native(xml).map_err(|error| unsupported(&error.to_string()))?;
    let document = parse(xml)?;
    let root = document.root_element();
    only(
        root,
        &[],
        &["Build", "Tree", "Skills", "Items", "Config", "Notes"],
    )?;
    let build = child(root, "Build")?;
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
        &[],
    )?;
    fixed(
        build,
        &[
            ("targetVersion", "0_1"),
            ("characterLevelAutoMode", "false"),
            ("mainSocketGroup", "1"),
            ("viewMode", "CALCS"),
        ],
    )?;
    let level = integer(build.attribute("level"), 1, 100, "character level")?;
    let tree = child(root, "Tree")?;
    only(tree, &["activeSpec"], &["Spec"])?;
    fixed(tree, &[("activeSpec", "1")])?;
    let spec = child(tree, "Spec")?;
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
        &[],
    )?;
    fixed(
        spec,
        &[
            ("treeVersion", data.tree.source.tree_version.as_str()),
            ("masteryEffects", ""),
        ],
    )?;
    let class_id = integer(
        spec.attribute("classInternalId"),
        0,
        u32::MAX,
        "classInternalId",
    )?;
    let class = data
        .tree
        .class(class_id)
        .map_err(|error| unsupported(&error.to_string()))?;
    let source_class = integer(spec.attribute("classId"), 0, u32::MAX, "classId")?;
    if (source_class != class_id && source_class != class.source_index)
        || build.attribute("className") != Some(class.name.as_str())
    {
        return Err(unsupported("class identity fields disagree"));
    }
    let ascendancy_index = integer(
        spec.attribute("ascendClassId"),
        0,
        u32::MAX,
        "ascendClassId",
    )?;
    let ascendancy_id = spec.attribute("ascendancyInternalId").unwrap_or("");
    let selection = ClassTreeSelection {
        class_id,
        ascendancy_id: if ascendancy_index == 0 {
            None
        } else {
            Some(ascendancy_id.into())
        },
        entrance_node_id: None,
        ascendancy_node_id: None,
    };
    let base = selection
        .resolve(&data.tree)
        .map_err(|error| unsupported(&error.to_string()))?;
    if base.ascendancy.as_ref().map_or(0, |asc| asc.class_index) != ascendancy_index
        || base
            .ascendancy
            .as_ref()
            .map_or("", |asc| asc.internal_id.as_str())
            != ascendancy_id
        || build.attribute("ascendClassName")
            != Some(
                base.ascendancy
                    .as_ref()
                    .map_or("None", |asc| asc.name.as_str()),
            )
    {
        return Err(unsupported("ascendancy identity fields disagree"));
    }
    let nodes = spec
        .attribute("nodes")
        .ok_or_else(|| unsupported("explicit nodes required"))?;
    let mut requested = BTreeSet::new();
    if !nodes.is_empty() {
        for node in nodes.split(',') {
            if !requested.insert(integer(Some(node), 0, u32::MAX, "allocated node")?) {
                return Err(unsupported("duplicate allocated node"));
            }
        }
    }
    let (ascendancy_ids, ordinary_ids): (Vec<_>, Vec<_>) = requested
        .difference(&base.implicit_roots)
        .copied()
        .partition(|id| data.tree.ascendancy_nodes.contains_key(id));
    if ordinary_ids.len() > 1 || ascendancy_ids.len() > 1 {
        return Err(unsupported(
            "at most one ordinary entrance and one reviewed ascendancy passive are supported",
        ));
    }
    let tree = ClassTreeSelection {
        entrance_node_id: ordinary_ids.first().copied(),
        ascendancy_node_id: ascendancy_ids.first().copied(),
        ..selection
    }
    .resolve(&data.tree)
    .map(Arc::new)
    .map_err(|error| unsupported(&error.to_string()))?;
    let mut tree_attribute_ranges = BTreeMap::new();
    for (node, fields) in [
        (build, &["className", "ascendClassName"][..]),
        (
            spec,
            &[
                "classId",
                "classInternalId",
                "ascendClassId",
                "ascendancyInternalId",
                "nodes",
            ][..],
        ),
    ] {
        for name in fields {
            if let Some(attribute) = node
                .attributes()
                .find(|attribute| attribute.name() == *name)
            {
                tree_attribute_ranges.insert((*name).into(), attribute.range_value());
            }
        }
    }
    // Insert a missing optional ascendancy identity before the first existing attribute.
    let ascendancy_insert = spec
        .attributes()
        .next()
        .expect("validated Spec attributes")
        .range()
        .start;
    let skills = child(root, "Skills")?;
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
    let set = child(skills, "SkillSet")?;
    only(set, &["id", "title"], &["Skill"])?;
    fixed(set, &[("id", "1")])?;
    let group = child(set, "Skill")?;
    only(
        group,
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
        group,
        &[
            ("enabled", "true"),
            ("includeInFullDPS", "true"),
            ("groupCount", "1"),
            ("mainActiveSkill", "1"),
            ("mainActiveSkillCalcs", "1"),
        ],
    )?;
    let gems: Vec<_> = group.children().filter(Node::is_element).collect();
    if !(1..=2).contains(&gems.len()) {
        return Err(unsupported(
            "one Mace Strike and zero or one Brutality I required",
        ));
    }
    check_gem(gems[0], false, false, data)?;
    if gems.len() == 2 {
        check_gem(gems[1], true, false, data)?;
    }
    let support_range = if gems.len() == 2 {
        gems[1].range()
    } else {
        gems[0].range().end..gems[0].range().end
    };
    let items = child(root, "Items")?;
    only(items, &["activeItemSet"], &["Item", "ItemSet"])?;
    fixed(items, &[("activeItemSet", "1")])?;
    let item = child(items, "Item")?;
    only(item, &["id"], &[])?;
    fixed(item, &[("id", "1")])?;
    if item.children().count() != 1 || !item.first_child().is_some_and(|node| node.is_text()) {
        return Err(unsupported("item must contain one unsplit text payload"));
    }
    let weapon = normal_mace(item.text().unwrap_or_default(), data)?;
    let item_set = child(items, "ItemSet")?;
    only(item_set, &["id", "title", "useSecondWeaponSet"], &["Slot"])?;
    fixed(item_set, &[("id", "1"), ("useSecondWeaponSet", "false")])?;
    let slot = child(item_set, "Slot")?;
    only(slot, &["name", "itemId"], &[])?;
    fixed(slot, &[("name", "Weapon 1"), ("itemId", "1")])?;
    let config = child(root, "Config")?;
    only(config, &["activeConfigSet"], &["ConfigSet"])?;
    fixed(config, &[("activeConfigSet", "1")])?;
    let config_set = child(config, "ConfigSet")?;
    only(config_set, &["id", "title"], &["Input"])?;
    fixed(config_set, &[("id", "1")])?;
    let mut inputs = BTreeMap::new();
    for node in config_set.children().filter(Node::is_element) {
        let name = node
            .attribute("name")
            .ok_or_else(|| unsupported("configuration name missing"))?;
        let value = scalar(node)?;
        validate_input(name, &value, data)?;
        if inputs.insert(name.into(), value).is_some() {
            return Err(unsupported("duplicate configuration input"));
        }
    }
    for required in INPUT_NAMES {
        if !inputs.contains_key(*required) {
            return Err(unsupported(&format!(
                "explicit configuration {required} required"
            )));
        }
    }
    if [
        "enemyPhysicalDamage",
        "enemyFireDamage",
        "enemyColdDamage",
        "enemyLightningDamage",
        "enemyChaosDamage",
    ]
    .iter()
    .all(|name| scalar_number(&inputs[*name]) == Some(0.0))
    {
        return Err(unsupported(
            "at least one incoming damage component must be positive",
        ));
    }
    for notes in root.children().filter(|node| node.has_tag_name("Notes")) {
        only(notes, &[], &[])?;
    }
    Ok(Profile {
        level,
        config: inputs,
        item_range: item.range(),
        support_range,
        weapon,
        active_attributes: attributes(gems[0]),
        has_support: gems.len() == 2,
        tree,
        tree_attribute_ranges,
        ascendancy_insert,
    })
}
const INPUT_NAMES: &[&str] = &[
    "enemyIsBoss",
    "enemyLevel",
    "enemyArmour",
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
];
fn validate_input(name: &str, value: &Scalar, data: &GameDataPackage) -> Result<()> {
    let valid = match (name, value) {
        (name, Scalar::Boolean(_)) if data.quests.config_keys.iter().any(|key| key == name) => true,
        ("resistancePenalty", Scalar::Number(value)) => (-60.0..=0.0).contains(value),
        ("enemyIsBoss", Scalar::Text(value)) => {
            ["None", "Boss", "Pinnacle"].contains(&value.as_str())
        }
        ("enemyDamageType", Scalar::Text(value)) => value == "Melee",
        (
            "conditionEnemyShocked"
            | "conditionEnemyChilled"
            | "conditionEnemyIgnited"
            | "conditionCritRecently"
            | "conditionBeenHitRecently",
            Scalar::Boolean(value),
        ) => !value,
        ("enemyLevel", Scalar::Number(value)) => {
            (1.0..=85.0).contains(value) && value.fract() == 0.0
        }
        (
            "enemyFireResist" | "enemyColdResist" | "enemyLightningResist" | "enemyChaosResist",
            Scalar::Number(value),
        ) => (-200.0..=100.0).contains(value),
        (
            "enemyArmour"
            | "enemyPhysicalDamage"
            | "enemyFireDamage"
            | "enemyColdDamage"
            | "enemyLightningDamage"
            | "enemyChaosDamage",
            Scalar::Number(value),
        ) => (0.0..=1_000_000.0).contains(value),
        (
            "enemyPhysicalOverwhelm" | "enemyFirePen" | "enemyColdPen" | "enemyLightningPen",
            Scalar::Number(value),
        ) => (0.0..=100.0).contains(value),
        ("enemyCritChance", Scalar::Number(value)) => *value == 0.0,
        ("enemySpeed", Scalar::Number(value)) => (1.0..=60_000.0).contains(value),
        ("multiplierNearbyEnemies", Scalar::Number(value)) => *value == 1.0,
        ("multiplierNearbyRareOrUniqueEnemies", Scalar::Number(value)) => *value == 0.0,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(unsupported(&format!(
            "unsupported configuration {name}={value:?}"
        )))
    }
}
fn check_export(
    xml: &str,
    choice: &Choice,
    level: u32,
    context: &EvaluationContext,
    data: &GameDataPackage,
) -> Result<()> {
    let document = parse(xml)?;
    let root = document.root_element();
    let build = child(root, "Build")?;
    fixed(
        build,
        &[
            ("className", choice.tree.class.name.as_str()),
            (
                "ascendClassName",
                choice
                    .tree
                    .ascendancy
                    .as_ref()
                    .map_or("None", |asc| asc.name.as_str()),
            ),
            ("mainSocketGroup", "1"),
        ],
    )?;
    if integer(build.attribute("level"), 1, 100, "exported character level")? != level {
        return Err(mismatch("exported level changed"));
    }
    let tree = child(root, "Tree")?;
    fixed(tree, &[("activeSpec", "1")])?;
    let spec = child(tree, "Spec")?;
    fixed(
        spec,
        &[
            ("classId", &choice.tree.class.integer_id.to_string()),
            ("classInternalId", &choice.tree.class.integer_id.to_string()),
            (
                "ascendancyInternalId",
                choice
                    .tree
                    .ascendancy
                    .as_ref()
                    .map_or("", |asc| asc.internal_id.as_str()),
            ),
            (
                "ascendClassId",
                &choice
                    .tree
                    .ascendancy
                    .as_ref()
                    .map_or(0, |asc| asc.class_index)
                    .to_string(),
            ),
            ("treeVersion", &data.tree.source.tree_version),
            ("masteryEffects", ""),
        ],
    )?;
    let nodes = spec
        .attribute("nodes")
        .ok_or_else(|| mismatch("exported nodes missing"))?;
    let mut allocated = BTreeSet::new();
    for node in nodes.split(',') {
        if !allocated.insert(integer(Some(node), 0, u32::MAX, "exported node")?) {
            return Err(mismatch("duplicate exported allocation"));
        }
    }
    if allocated != choice.tree.allocated_nodes {
        return Err(mismatch("exported allocations differ from selected tree"));
    }
    if spec
        .attribute("secondaryAscendClassId")
        .is_some_and(|value| value != "nil")
    {
        return Err(mismatch("secondary ascendancy changed"));
    }
    for container in spec.children().filter(Node::is_element) {
        match container.tag_name().name() {
            "URL" => {}
            "Sockets" if !container.children().any(|node| node.is_element()) => {}
            "Overrides" => {
                for entry in container.children().filter(Node::is_element) {
                    if !entry.has_tag_name("AttributeOverride")
                        || entry.attributes().any(|attribute| {
                            !["strNodes", "dexNodes", "intNodes"].contains(&attribute.name())
                                || !attribute.value().is_empty()
                        })
                    {
                        return Err(mismatch("passive override added"));
                    }
                }
            }
            _ => return Err(mismatch("unsupported exported passive state")),
        }
    }
    let items = child(root, "Items")?;
    let item_set = child(items, "ItemSet")?;
    fixed(
        items,
        &[("activeItemSet", "1"), ("useSecondWeaponSet", "false")],
    )?;
    fixed(item_set, &[("id", "1"), ("useSecondWeaponSet", "false")])?;
    let equipped: Vec<_> = item_set
        .children()
        .filter(|node| node.has_tag_name("Slot") && node.attribute("itemId") != Some("0"))
        .collect();
    if equipped.len() != 1 {
        return Err(mismatch("equipped item count changed"));
    }
    fixed(equipped[0], &[("name", "Weapon 1"), ("itemId", "1")])?;
    if item_set
        .children()
        .filter(|node| node.has_tag_name("RuneSlot"))
        .any(|node| node.attribute("runeName") != Some("None"))
    {
        return Err(mismatch("equipment rune added"));
    }
    let item = child(items, "Item")?;
    only(item, &["id"], &[])?;
    fixed(item, &[("id", "1")])?;
    if item.children().count() != 1 || !item.first_child().is_some_and(|node| node.is_text()) {
        return Err(mismatch("exported item payload split"));
    }
    let actual: Vec<_> = item
        .text()
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let mut expected: Vec<_> = choice.weapon.lines().collect();
    expected.insert(4, "LevelReq: 0");
    if actual != expected {
        return Err(mismatch("exact equipped item payload changed"));
    }
    let skills = child(root, "Skills")?;
    fixed(skills, &[("activeSkillSet", "1")])?;
    let set = child(skills, "SkillSet")?;
    fixed(set, &[("id", "1")])?;
    let group = child(set, "Skill")?;
    only(
        group,
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
        group,
        &[
            ("enabled", "true"),
            ("includeInFullDPS", "true"),
            ("groupCount", "1"),
            ("mainActiveSkill", "1"),
            ("mainActiveSkillCalcs", "1"),
        ],
    )?;
    let gems: Vec<_> = group.children().filter(Node::is_element).collect();
    if gems.len()
        != if choice.support == MaceSupportChoice::None {
            1
        } else {
            2
        }
    {
        return Err(mismatch("exported support count changed"));
    }
    for (index, gem) in gems.iter().enumerate() {
        check_gem(*gem, index > 0, true, data)?;
    }
    let config = child(root, "Config")?;
    fixed(config, &[("activeConfigSet", "1")])?;
    let set = child(config, "ConfigSet")?;
    fixed(set, &[("id", "1")])?;
    let mut names = BTreeSet::new();
    for entry in set.children().filter(Node::is_element) {
        let kind = entry.tag_name().name();
        if !names.insert((kind, entry.attribute("name"))) {
            return Err(mismatch("duplicate exported configuration"));
        }
        match kind {
            "Input" | "Placeholder" => {
                let values = if kind == "Input" {
                    &context.config_inputs
                } else {
                    &context.config_placeholders
                };
                let actual = scalar(entry)?;
                if entry.attribute("name").and_then(|name| values.get(name)) != Some(&actual) {
                    return Err(mismatch(
                        "exported configuration differs from effective context",
                    ));
                }
            }
            "CustomModifierBlock"
                if entry.attribute("title") == Some("Default")
                    && entry.attribute("enabled") == Some("true")
                    && !entry.children().any(|node| {
                        node.is_element() || node.text().is_some_and(|text| !text.trim().is_empty())
                    }) => {}
            _ => return Err(mismatch("unsupported exported configuration")),
        }
    }
    Ok(())
}
fn check_gem(
    node: Node<'_, '_>,
    support: bool,
    exported: bool,
    data: &GameDataPackage,
) -> Result<()> {
    let mut allowed = vec![
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
    ];
    if exported {
        allowed.extend([
            "corrupted",
            "corruptLevel",
            "statSetIndex",
            "statSetIndexCalcs",
        ]);
    }
    only(node, &allowed, &[])?;
    let (name, skill, game, variant) = if support {
        (
            &data.mace.brutality.name,
            &data.mace.brutality.skill_id,
            &data.mace.brutality.game_id,
            &data.mace.brutality.variant_id,
        )
    } else {
        (
            &data.mace.name,
            &data.mace.skill_id,
            &data.mace.game_id,
            &data.mace.variant_id,
        )
    };
    fixed(
        node,
        &[
            ("nameSpec", name),
            ("skillId", skill),
            ("gemId", game),
            ("variantId", variant),
            ("level", "1"),
            ("quality", "0"),
            ("enabled", "true"),
            ("enableGlobal1", "true"),
            ("enableGlobal2", "true"),
            ("count", "1"),
        ],
    )?;
    for (name, expected) in [
        ("corrupted", "false"),
        ("corruptLevel", "0"),
        ("statSetIndex", "nil"),
        ("statSetIndexCalcs", "nil"),
    ] {
        if node.attribute(name).is_some_and(|value| value != expected) {
            return Err(mismatch("unsupported exported gem setting"));
        }
    }
    Ok(())
}
/// Canonical normalized XML except numeric Build outputs and validated mutable build state.
fn exported_scenario(xml: &str, support_id: &str) -> Result<String> {
    fn visit(node: Node<'_, '_>, support_id: &str) -> String {
        if !node.is_element() {
            return String::new();
        }
        let tag = node.tag_name().name();
        if (tag == "Gem" && node.attribute("skillId") == Some(support_id))
            || (["PlayerStat", "MinionStat", "FullDPSSkill"].contains(&tag)
                && node
                    .parent_element()
                    .is_some_and(|parent| parent.has_tag_name("Build")))
        {
            return String::new();
        }
        if tag == "URL"
            && node
                .parent_element()
                .is_some_and(|parent| parent.has_tag_name("Spec"))
        {
            return String::new();
        }
        let mut fields = attributes(node);
        if tag == "Build" {
            for name in ["className", "ascendClassName"] {
                fields.remove(name);
            }
        }
        if tag == "Spec" {
            for name in [
                "classId",
                "classInternalId",
                "ascendClassId",
                "ascendancyInternalId",
                "nodes",
            ] {
                fields.remove(name);
            }
        }
        let mut output = format!("<{tag}:{fields:?}>");
        if !(tag == "Item"
            && node
                .parent_element()
                .is_some_and(|parent| parent.has_tag_name("Items")))
        {
            // PoB serializes several source tables with Lua pairs(). Child order
            // is validated explicitly where it has semantics (active/support gems).
            let mut children: Vec<_> = node
                .children()
                .filter(Node::is_element)
                .map(|node| visit(node, support_id))
                .filter(|value| !value.is_empty())
                .collect();
            children.sort();
            for child in children {
                output.push_str(&child);
            }
            for text in node
                .children()
                .filter(Node::is_text)
                .filter_map(|child| child.text())
                .filter(|text| !text.trim().is_empty())
            {
                output.push_str(&format!("{:?}", text.trim()));
            }
        }
        output.push_str(&format!("</{tag}>"));
        output
    }
    let document = parse(xml)?;
    Ok(visit(document.root_element(), support_id))
}
fn normal_mace(input: &str, data: &GameDataPackage) -> Result<String> {
    if input.len() > 1024 {
        return Err(unsupported("normal mace text exceeds 1024 bytes"));
    }
    let text = input.replace("\r\n", "\n");
    let text = text.trim();
    let lines: Vec<_> = text.lines().collect();
    if lines.len() != 5
        || lines[0] != "Rarity: NORMAL"
        || !data.weapons.iter().any(|weapon| weapon.name == lines[1])
        || lines[4] != "Implicits: 0"
    {
        return Err(unsupported(
            "only exact normal mace payloads selected by the data package without modifiers are supported",
        ));
    }
    integer(lines[2].strip_prefix("Item Level: "), 1, 100, "item level")?;
    integer(lines[3].strip_prefix("Quality: "), 0, 20, "weapon quality")?;
    Ok(text.into())
}
fn patch(template: &str, profile: &Profile, choice: &Choice, support_xml: &str) -> String {
    let support = if choice.support == MaceSupportChoice::None {
        String::new()
    } else if profile.has_support {
        support_xml.into()
    } else {
        format!("\n        {support_xml}")
    };
    let mut patches = vec![
        (
            profile.item_range.clone(),
            format!("<Item id=\"1\">{}\n</Item>", escape(&choice.weapon)),
        ),
        (profile.support_range.clone(), support),
    ];
    if choice.patch_tree {
        let tree = &choice.tree;
        let fields = [
            ("className", tree.class.name.clone()),
            (
                "ascendClassName",
                tree.ascendancy
                    .as_ref()
                    .map_or("None", |asc| asc.name.as_str())
                    .into(),
            ),
            ("classId", tree.class.integer_id.to_string()),
            ("classInternalId", tree.class.integer_id.to_string()),
            (
                "ascendClassId",
                tree.ascendancy
                    .as_ref()
                    .map_or(0, |asc| asc.class_index)
                    .to_string(),
            ),
            (
                "ascendancyInternalId",
                tree.ascendancy
                    .as_ref()
                    .map_or("", |asc| asc.internal_id.as_str())
                    .into(),
            ),
            // Roots are implicit; preserve the selected paid physical nodes in authored XML.
            (
                "nodes",
                tree.selection
                    .entrance_node_id
                    .into_iter()
                    .chain(tree.selection.ascendancy_node_id)
                    .collect::<BTreeSet<_>>()
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ];
        for (name, value) in fields {
            if let Some(range) = profile.tree_attribute_ranges.get(name) {
                patches.push((range.clone(), escape_attribute(&value)));
            } else {
                debug_assert_eq!(name, "ascendancyInternalId");
                patches.push((
                    profile.ascendancy_insert..profile.ascendancy_insert,
                    format!("{name}=\"{}\" ", escape_attribute(&value)),
                ));
            }
        }
    }
    patches.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = template.to_owned();
    for (range, replacement) in patches {
        output.replace_range(range, &replacement);
    }
    output
}
fn support_xml(data: &GameDataPackage) -> String {
    let support = &data.mace.brutality;
    format!(
        r#"<Gem nameSpec="{}" skillId="{}" gemId="{}" variantId="{}" level="1" quality="0" enabled="true" enableGlobal1="true" enableGlobal2="true" count="1"/>"#,
        escape_attribute(&support.name),
        escape_attribute(&support.skill_id),
        escape_attribute(&support.game_id),
        escape_attribute(&support.variant_id)
    )
}
fn escape_attribute(text: &str) -> String {
    escape(text).replace('"', "&quot;").replace('\'', "&apos;")
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn payload(format: &str, content: String) -> ExactPayload {
    ExactPayload {
        format: format.into(),
        sha256: hash(&content),
        content,
    }
}
fn attributes(node: Node<'_, '_>) -> BTreeMap<String, String> {
    node.attributes()
        .map(|attribute| (attribute.name().into(), attribute.value().into()))
        .collect()
}
fn unsupported(message: &str) -> ControlledMutationError {
    ControlledMutationError::Unsupported(message.into())
}
fn mismatch(message: &str) -> ControlledMutationError {
    ControlledMutationError::Realization(message.into())
}
fn integer(value: Option<&str>, min: u32, max: u32, name: &str) -> Result<u32> {
    value
        .and_then(|text| {
            text.parse::<u32>()
                .ok()
                .filter(|value| value.to_string() == text)
        })
        .filter(|value| (min..=max).contains(value))
        .ok_or_else(|| unsupported(&format!("{name} must be an integer in {min}..={max}")))
}
fn parse(xml: &str) -> Result<Document<'_>> {
    crate::preflight::validate(xml).map_err(|error| unsupported(&error.to_string()))?;
    Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: crate::MAX_XML_NODES,
            entity_resolver: None,
        },
    )
    .map_err(|error| unsupported(&error.to_string()))
}
fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Result<Node<'a, 'input>> {
    let mut children = node.children().filter(|node| node.has_tag_name(name));
    let found = children
        .next()
        .ok_or_else(|| unsupported(&format!("missing {name}")))?;
    if children.next().is_some() {
        return Err(unsupported(&format!("duplicate {name}")));
    }
    Ok(found)
}
fn only(node: Node<'_, '_>, allowed_attributes: &[&str], allowed_children: &[&str]) -> Result<()> {
    if node.tag_name().namespace().is_some()
        || node.attributes().any(|attribute| {
            attribute.namespace().is_some() || !allowed_attributes.contains(&attribute.name())
        })
        || node.children().filter(Node::is_element).any(|child| {
            child.tag_name().namespace().is_some()
                || !allowed_children.contains(&child.tag_name().name())
        })
    {
        Err(unsupported(&format!(
            "unmapped attributes or children in {}",
            node.tag_name().name()
        )))
    } else {
        Ok(())
    }
}
fn fixed(node: Node<'_, '_>, expected: &[(&str, &str)]) -> Result<()> {
    for (name, value) in expected {
        if node.attribute(*name) != Some(*value) {
            return Err(unsupported(&format!(
                "{}.{} must be {value:?}",
                node.tag_name().name(),
                name
            )));
        }
    }
    Ok(())
}
fn scalar(node: Node<'_, '_>) -> Result<Scalar> {
    only(node, &["name", "number", "string", "boolean"], &[])?;
    let values: Vec<_> = ["number", "string", "boolean"]
        .into_iter()
        .filter_map(|name| node.attribute(name).map(|value| (name, value)))
        .collect();
    match values.as_slice() {
        [("number", text)] => text
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(Scalar::Number)
            .ok_or_else(|| unsupported("nonfinite configuration number")),
        [("string", text)] => Ok(Scalar::Text((*text).into())),
        [("boolean", "true")] => Ok(Scalar::Boolean(true)),
        [("boolean", "false")] => Ok(Scalar::Boolean(false)),
        _ => Err(unsupported("configuration must contain exactly one scalar")),
    }
}
fn scalar_number(value: &Scalar) -> Option<f64> {
    if let Scalar::Number(value) = value {
        Some(*value)
    } else {
        None
    }
}
/// Compare each explicitly supported source attribute outside mutated state.
/// Generated export defaults are checked by their specific validators and baseline guard.
fn check_template_frame(source_xml: &str, exported_xml: &str) -> Result<()> {
    let source = parse(source_xml)?;
    let exported = parse(exported_xml)?;
    for path in [
        &["Build"][..],
        &["Tree"],
        &["Tree", "Spec"],
        &["Skills"],
        &["Skills", "SkillSet"],
        &["Skills", "SkillSet", "Skill"],
        &["Items"],
        &["Items", "ItemSet"],
        &["Config"],
        &["Config", "ConfigSet"],
    ] {
        let mut left = source.root_element();
        let mut right = exported.root_element();
        for name in path {
            left = child(left, name)?;
            right = child(right, name)?;
        }
        for attribute in left.attributes() {
            // PoB explicitly adds the implicit class root to an empty allocation.
            if left.has_tag_name("Spec") && ["nodes", "classId"].contains(&attribute.name()) {
                continue;
            }
            if right.attribute(attribute.name()) != Some(attribute.value()) {
                return Err(mismatch(&format!(
                    "source {}.{} was not preserved",
                    left.tag_name().name(),
                    attribute.name()
                )));
            }
        }
    }
    let notes = |document: &Document<'_>| {
        document
            .root_element()
            .children()
            .find(|node| node.has_tag_name("Notes"))
            .map(|node| {
                node.children()
                    .filter(Node::is_text)
                    .filter_map(|child| child.text())
                    .collect::<String>()
                    .trim()
                    .to_owned()
            })
            .unwrap_or_default()
    };
    if notes(&source) != notes(&exported) {
        return Err(mismatch("source Notes were not preserved"));
    }
    let build = child(exported.root_element(), "Build")?;
    let buffs = child(build, "Buffs")?;
    only(buffs, &["combatList", "buffList", "curseList"], &[])?;
    fixed(
        buffs,
        &[("combatList", ""), ("buffList", ""), ("curseList", "")],
    )?;
    let timeless = child(build, "TimelessData")?;
    only(
        timeless,
        &[
            "searchList",
            "searchListFallback",
            "devotionVariant1",
            "devotionVariant2",
        ],
        &[],
    )?;
    fixed(
        timeless,
        &[
            ("searchList", ""),
            ("searchListFallback", ""),
            ("devotionVariant1", "1"),
            ("devotionVariant2", "1"),
        ],
    )?;
    if build.children().filter(Node::is_element).any(|node| {
        ![
            "PlayerStat",
            "MinionStat",
            "FullDPSSkill",
            "Buffs",
            "TimelessData",
        ]
        .contains(&node.tag_name().name())
    }) {
        return Err(mismatch(
            "exported build acquired unsupported persistent state",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "controlled_mace_tests.rs"]
mod tests;
