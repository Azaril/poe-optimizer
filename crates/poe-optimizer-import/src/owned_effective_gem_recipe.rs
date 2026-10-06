//! Offline lowering of reviewed effective Gem input policies into ordinary rules.
//! Outputs are pre-support active inputs or prepared support-origin inputs, never
//! final generated Skill parameters, source execution, or coverage certificates.
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, RuleLimits};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, io};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectiveGemRecipeInput {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    pub bindings: Vec<EffectiveGemRecipeBinding>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectiveGemRecipeBinding {
    pub gem: GemDefId,
    pub program: OwnedDefinitionKey,
    pub role: EffectiveGemRecipeRole,
    pub quality: QualityDefId,
    pub quality_absence: QualityAbsencePolicy,
    pub level_unit: UnitDefId,
    /// Explicit, eligible global channels only. Socket/property predicates must
    /// be established by authoring; this compiler cannot globalize scoped data.
    pub external_level: Vec<StatDefId>,
    pub external_quality: Vec<StatDefId>,
    pub level_output: StatDefId,
    pub quality_output: StatDefId,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EffectiveGemRecipeRole {
    ActivePreSupport {
        corruption: DeclaredSlot<ParameterSlotDefId>,
    },
    SupportPreparation {
        levels: DenseGemLevelPolicy,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityAbsencePolicy {
    /// A missing, absent or differently selected quality remains unresolved.
    RequireSelectedQuality,
    /// Zero is valid only after checking an explicitly closed singleton schema.
    ZeroWhenProvenSingleton,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DenseGemLevelPolicy {
    /// Reviewed table has every integral key 1..=maximum and no other keys.
    pub maximum: u16,
    /// Exact source fallback, which must itself be a valid dense table key.
    pub natural_maximum: u16,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectiveGemInputPhase {
    ActivePreSupport,
    SupportPreparation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectiveGemRecipeProgram {
    pub gem: GemDefId,
    pub phase: EffectiveGemInputPhase,
    pub program: RuleProgram,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectiveGemRecipeOutput {
    pub schema_version: u32,
    pub policy: OwnedContentDigest,
    pub definitions: DataIdentity,
    /// Bare program rows carry no owner/contributor completeness authority.
    pub programs: Vec<EffectiveGemRecipeProgram>,
    pub work_used: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct EffectiveGemRecipeLimits {
    pub max_policy_bytes: usize,
    pub max_output_bytes: usize,
    pub max_bindings: usize,
    pub max_channels_per_binding: usize,
    pub max_nodes: usize,
    pub max_work: usize,
}
impl Default for EffectiveGemRecipeLimits {
    fn default() -> Self {
        Self {
            max_policy_bytes: 2 * 1024 * 1024,
            max_output_bytes: 16 * 1024 * 1024,
            max_bindings: 1024,
            max_channels_per_binding: 256,
            max_nodes: 65536,
            max_work: 2_000_000,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum EffectiveGemRecipeError {
    #[error("effective Gem recipe limit: {0}")]
    Limit(&'static str),
    #[error("effective Gem recipe definition identity mismatch")]
    Binding,
    #[error("invalid effective Gem recipe: {0}")]
    Invalid(String),
}
type Result<T> = std::result::Result<T, EffectiveGemRecipeError>;
fn invalid(message: impl Into<String>) -> EffectiveGemRecipeError {
    EffectiveGemRecipeError::Invalid(message.into())
}
fn key(value: impl AsRef<str>) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value.as_ref()).expect("bounded compiler-owned symbol")
}
fn charge(work: &mut usize, amount: usize) -> Result<()> {
    *work = work
        .checked_sub(amount)
        .ok_or(EffectiveGemRecipeError::Limit("work"))?;
    Ok(())
}
fn add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b)
        .ok_or(EffectiveGemRecipeError::Limit("expansion"))
}
fn multiply(a: usize, b: usize) -> Result<usize> {
    a.checked_mul(b)
        .ok_or(EffectiveGemRecipeError::Limit("expansion"))
}
struct ByteCounter {
    used: usize,
    limit: usize,
}
impl io::Write for ByteCounter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.used = self
            .used
            .checked_add(bytes.len())
            .filter(|used| *used <= self.limit)
            .ok_or_else(|| io::Error::other("wire byte limit"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn wire_size(value: &impl Serialize, limit: usize, name: &'static str) -> Result<usize> {
    let mut writer = ByteCounter { used: 0, limit };
    serde_json::to_writer(&mut writer, value).map_err(|_| EffectiveGemRecipeError::Limit(name))?;
    Ok(writer.used)
}
fn quantity(value: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(
        FiniteQuantity::new(value, unit.clone()).expect("finite compiler scalar"),
    )
}
fn integer(value: u16) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(i64::from(value)).expect("u16 is bounded"))
}
fn known_stat(
    schema: &OwnedDefinitionSchemaPackage,
    id: &StatDefId,
    value: &ComputedValueType,
    target: RuleEntityKind,
) -> Result<()> {
    match schema.definition(id) {
        SchemaLookup::Known(stat) if &stat.value == value && stat.targets.contains(&target) => {
            Ok(())
        }
        _ => Err(invalid(
            "stat is missing, has another unit/type, or lacks its required target",
        )),
    }
}
fn phase(
    role: &EffectiveGemRecipeRole,
) -> (EffectiveGemInputPhase, RuleEntityKind, AuthoredGemRole) {
    match role {
        EffectiveGemRecipeRole::ActivePreSupport { .. } => (
            EffectiveGemInputPhase::ActivePreSupport,
            RuleEntityKind::Skill,
            AuthoredGemRole::SkillUse,
        ),
        EffectiveGemRecipeRole::SupportPreparation { .. } => (
            EffectiveGemInputPhase::SupportPreparation,
            RuleEntityKind::SupportOrigin,
            AuthoredGemRole::SupportAssignment,
        ),
    }
}
fn validate_binding(
    binding: &EffectiveGemRecipeBinding,
    schema: &OwnedDefinitionSchemaPackage,
    work: &mut usize,
) -> Result<UnitDefId> {
    let SchemaLookup::Known(gem) = schema.definition(&binding.gem) else {
        return Err(invalid("Gem schema must be known"));
    };
    charge(
        work,
        add(gem.roles.len(), gem.quality.allowed_kinds.members.len())?,
    )?;
    let (_, context, role) = phase(&binding.role);
    if !gem.roles.contains(&role) {
        return Err(invalid("Gem does not admit the requested physical role"));
    }
    // HasGemQuality(false) also represents another selected quality kind. It is
    // only an absence guard after this exact, closed singleton check.
    if !gem.quality.allowed_kinds.members.contains(&binding.quality)
        || gem.quality.presence == QualityPresence::Forbidden
        || (binding.quality_absence == QualityAbsencePolicy::ZeroWhenProvenSingleton
            && (!gem.quality.allowed_kinds.is_complete()
                || gem.quality.allowed_kinds.members.len() != 1))
    {
        return Err(invalid(
            "quality is not an explicit member or its zero-absence policy lacks a closed singleton",
        ));
    }
    let SchemaLookup::Known(quality) = schema.definition(&binding.quality) else {
        return Err(invalid("quality schema must be known"));
    };
    let quality_unit = quality.amount.minimum.unit();
    if !matches!(schema.definition(quality_unit), SchemaLookup::Known(unit) if unit.dimension == UnitDimension::PercentagePoints)
        || !matches!(schema.definition(&binding.level_unit), SchemaLookup::Known(unit) if unit.dimension == UnitDimension::Count)
    {
        return Err(invalid(
            "level/quality units must be Count/PercentagePoints",
        ));
    }
    if let EffectiveGemRecipeRole::SupportPreparation { levels } = binding.role
        && (levels.maximum == 0
            || levels.natural_maximum == 0
            || levels.natural_maximum > levels.maximum)
    {
        return Err(invalid(
            "dense level interval and natural fallback are inconsistent",
        ));
    }
    if binding.level_output == binding.quality_output {
        return Err(invalid("level and quality outputs must be distinct"));
    }
    let level_type = match binding.role {
        EffectiveGemRecipeRole::ActivePreSupport { .. } => ComputedValueType::Quantity {
            unit: binding.level_unit.clone(),
        },
        EffectiveGemRecipeRole::SupportPreparation { .. } => ComputedValueType::Integer,
    };
    known_stat(schema, &binding.level_output, &level_type, context)?;
    known_stat(
        schema,
        &binding.quality_output,
        &ComputedValueType::Quantity {
            unit: quality_unit.clone(),
        },
        context,
    )?;
    if let EffectiveGemRecipeRole::ActivePreSupport { corruption } = &binding.role {
        charge(work, gem.declarations.parameters.members.len())?;
        if corruption.declaration != SlotOwnerDefId::Gem(binding.gem.clone())
            || !gem.declarations.parameters.members.contains(corruption)
            || !matches!(schema.slot(corruption), SchemaLookup::Known(parameter)
                if matches!(&parameter.value, ValueSchema::Quantity(range)
                    if range.minimum.unit() == &binding.level_unit && range.maximum.unit() == &binding.level_unit))
        {
            return Err(invalid(
                "active corruption must be an exact declared Gem Count parameter",
            ));
        }
    }
    for (channels, unit) in [
        (&binding.external_level, &binding.level_unit),
        (&binding.external_quality, quality_unit),
    ] {
        let mut seen = BTreeSet::new();
        for stat in channels {
            charge(work, 1)?;
            if !seen.insert(stat) {
                return Err(invalid("duplicate external channel"));
            }
            known_stat(
                schema,
                stat,
                &ComputedValueType::Quantity { unit: unit.clone() },
                RuleEntityKind::Actor,
            )?;
        }
    }
    Ok(quality_unit.clone())
}
struct ProgramBuilder {
    reads: Vec<RuleRead>,
    nodes: Vec<RuleNode>,
}
impl ProgramBuilder {
    fn node(&mut self, id: &str, expression: RuleExpression) -> OwnedDefinitionKey {
        let id = key(id);
        self.nodes.push(RuleNode {
            id: id.clone(),
            expression,
        });
        id
    }
    fn read(
        &mut self,
        id: &str,
        value_type: ComputedValueType,
        source: RuleReadSource,
    ) -> OwnedDefinitionKey {
        self.reads.push(RuleRead {
            id: key(id),
            value_type,
            source,
        });
        self.node(id, RuleExpression::Read { input: key(id) })
    }
    fn literal(&mut self, id: &str, value: ParameterValue) -> OwnedDefinitionKey {
        self.node(id, RuleExpression::Literal { value })
    }
    fn external(
        &mut self,
        label: &str,
        channels: &[StatDefId],
        unit: &UnitDefId,
        mut value: OwnedDefinitionKey,
    ) -> OwnedDefinitionKey {
        for (i, stat) in channels.iter().enumerate() {
            let read = self.read(
                &format!("external-{label}-{i}"),
                ComputedValueType::Quantity { unit: unit.clone() },
                RuleReadSource::Contributions {
                    entity: RuleEntity::Player,
                    stat: stat.clone(),
                    contribution: ContributionKind::Add,
                    reduction: ContributionReduction::Sum,
                    empty: quantity(0.0, unit),
                },
            );
            value = self.node(
                &format!("adjusted-{label}-{i}"),
                RuleExpression::Add {
                    left: value,
                    right: read,
                },
            );
        }
        value
    }
}
fn lower(
    binding: &EffectiveGemRecipeBinding,
    quality_unit: &UnitDefId,
) -> EffectiveGemRecipeProgram {
    let mut b = ProgramBuilder {
        reads: Vec::new(),
        nodes: Vec::new(),
    };
    let raw = b.read(
        "raw-level",
        ComputedValueType::Integer,
        RuleReadSource::GemLevel,
    );
    let quantum = b.literal("level-quantum", quantity(1.0, &binding.level_unit));
    let mut level = b.node(
        "physical-level",
        RuleExpression::ScaleInteger {
            value: quantum.clone(),
            count: raw,
        },
    );
    if let EffectiveGemRecipeRole::ActivePreSupport { corruption } = &binding.role {
        let delta = b.read(
            "corruption",
            ComputedValueType::Quantity {
                unit: binding.level_unit.clone(),
            },
            RuleReadSource::Parameter {
                slot: corruption.clone(),
            },
        );
        let adjusted = b.node(
            "corrupted-level",
            RuleExpression::Add {
                left: level,
                right: delta,
            },
        );
        level = b.node(
            "active-initial-level",
            RuleExpression::Maximum {
                left: adjusted,
                right: quantum.clone(),
            },
        );
    }
    level = b.external("level", &binding.external_level, &binding.level_unit, level);
    if let EffectiveGemRecipeRole::SupportPreparation { levels } = binding.role {
        let minimum = b.node(
            "level-minimum",
            RuleExpression::Maximum {
                left: level,
                right: quantum.clone(),
            },
        );
        let maximum = b.literal(
            "level-maximum",
            quantity(f64::from(levels.maximum), &binding.level_unit),
        );
        let clamped = b.node(
            "level-clamped",
            RuleExpression::Minimum {
                left: minimum,
                right: maximum,
            },
        );
        // Quantization is only an integrality witness. Fractional source levels use
        // the declared natural fallback; they are never silently rounded to a row.
        let count = b.node(
            "level-count",
            RuleExpression::QuantizeInteger {
                value: clamped.clone(),
                quantum: FiniteQuantity::new(1.0, binding.level_unit.clone()).unwrap(),
                mode: RuleRounding::Truncate,
            },
        );
        let round_trip = b.node(
            "level-round-trip",
            RuleExpression::ScaleInteger {
                value: quantum,
                count: count.clone(),
            },
        );
        let integral = b.node(
            "level-is-integral",
            RuleExpression::Compare {
                operation: RuleComparison::Equal,
                left: clamped,
                right: round_trip,
            },
        );
        let fallback = b.literal("natural-level", integer(levels.natural_maximum));
        level = b.node(
            "effective-level",
            RuleExpression::Select {
                condition: integral,
                when_true: count,
                when_false: fallback,
            },
        );
    }
    let raw_quality = b.read(
        "raw-quality",
        ComputedValueType::Quantity {
            unit: quality_unit.clone(),
        },
        RuleReadSource::GemQualityAmount {
            quality: binding.quality.clone(),
        },
    );
    let raw_quality = if binding.quality_absence == QualityAbsencePolicy::ZeroWhenProvenSingleton {
        let has_quality = b.read(
            "has-quality",
            ComputedValueType::Boolean,
            RuleReadSource::HasGemQuality {
                quality: binding.quality.clone(),
            },
        );
        let zero = b.literal("absent-quality", quantity(0.0, quality_unit));
        b.node(
            "physical-quality",
            RuleExpression::Select {
                condition: has_quality,
                when_true: raw_quality,
                when_false: zero,
            },
        )
    } else {
        raw_quality
    };
    let quality = b.external(
        "quality",
        &binding.external_quality,
        quality_unit,
        raw_quality,
    );
    let (phase, context, _) = phase(&binding.role);
    EffectiveGemRecipeProgram {
        gem: binding.gem.clone(),
        phase,
        program: RuleProgram {
            id: binding.program.clone(),
            context,
            reads: b.reads,
            nodes: b.nodes,
            effects: vec![
                RuleEffect {
                    id: key("level"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: binding.level_output.clone(),
                        value: level,
                    },
                },
                RuleEffect {
                    id: key("quality"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: binding.quality_output.clone(),
                        value: quality,
                    },
                },
            ],
        },
    }
}
/// Compile without changing schemas, owner closures, supplied parameters, or
/// existing releases. A caller must separately prove global channel membership
/// and assign programs to pre-support stages in a checked evaluation package.
pub fn compile_effective_gem_recipe(
    input: &EffectiveGemRecipeInput,
    schema: &OwnedDefinitionSchemaPackage,
    limits: EffectiveGemRecipeLimits,
) -> Result<EffectiveGemRecipeOutput> {
    let hard = EffectiveGemRecipeLimits::default();
    for (name, value, maximum) in [
        (
            "policy bytes",
            limits.max_policy_bytes,
            hard.max_policy_bytes,
        ),
        (
            "output bytes",
            limits.max_output_bytes,
            hard.max_output_bytes,
        ),
        ("bindings", limits.max_bindings, hard.max_bindings),
        (
            "channels",
            limits.max_channels_per_binding,
            hard.max_channels_per_binding,
        ),
        ("nodes", limits.max_nodes, hard.max_nodes),
        ("work", limits.max_work, hard.max_work),
    ] {
        if value == 0 || value > maximum {
            return Err(EffectiveGemRecipeError::Limit(name));
        }
    }
    if input.schema_version != 1 {
        return Err(invalid("policy version"));
    }
    if &input.definitions != schema.identity() {
        return Err(EffectiveGemRecipeError::Binding);
    }
    if schema.input().schema_version < 4 {
        return Err(invalid("Skill/SupportOrigin outputs require schema v4"));
    }
    if input.bindings.is_empty() || input.bindings.len() > limits.max_bindings {
        return Err(EffectiveGemRecipeError::Limit("bindings"));
    }
    let bytes = wire_size(input, limits.max_policy_bytes, "policy bytes")?;
    let mut work = limits.max_work;
    charge(&mut work, add(bytes, input.bindings.len())?)?;
    // Charge conservative expansion BEFORE allocating/cloning generated rows.
    // Input references are repeated a bounded number of times; fixed per-row
    // overhead also covers generated nodes, typechecking clones, and ID strings.
    let mut nodes = 0;
    let mut output_bytes = 1024;
    for binding in &input.bindings {
        let channels = add(binding.external_level.len(), binding.external_quality.len())?;
        if channels > limits.max_channels_per_binding {
            return Err(EffectiveGemRecipeError::Limit("channels"));
        }
        nodes = add(nodes, add(32, multiply(channels, 2)?)?)?;
        let size = wire_size(binding, limits.max_policy_bytes, "policy bytes")?;
        output_bytes = add(output_bytes, add(32768, multiply(size, 16)?)?)?;
        if nodes > limits.max_nodes {
            return Err(EffectiveGemRecipeError::Limit("nodes"));
        }
        if output_bytes > limits.max_output_bytes {
            return Err(EffectiveGemRecipeError::Limit("output bytes"));
        }
    }
    charge(&mut work, add(multiply(nodes, 8)?, output_bytes / 64)?)?;
    // Reserve semantic compilation up front too. Its bounded allowance is part
    // of this call's conservative work accounting, not an unmetered second pass.
    let typecheck_work = add(multiply(nodes, 128)?, multiply(input.bindings.len(), 32)?)?;
    charge(&mut work, typecheck_work)?;
    let policy = digest_owned(
        "owned-effective-gem-recipe-v1",
        input,
        limits.max_policy_bytes,
    )
    .map_err(|error| invalid(error.to_string()))?;
    let mut seen = BTreeSet::new();
    let mut outputs = BTreeSet::new();
    let mut programs = Vec::with_capacity(input.bindings.len());
    for binding in &input.bindings {
        if !seen.insert((&binding.gem, &binding.program)) {
            return Err(invalid("duplicate Gem/program binding"));
        }
        let (_, context, _) = phase(&binding.role);
        for stat in [&binding.level_output, &binding.quality_output] {
            if !outputs.insert((&binding.gem, context, stat)) {
                return Err(invalid("competing output in the same Gem role"));
            }
        }
        let unit = validate_binding(binding, schema, &mut work)?;
        programs.push(lower(binding, &unit));
    }
    // Semantic typechecking uses explicitly Partial temporary owners. Nothing
    // exports that temporary package or treats it as contributor coverage.
    let mut owners = std::collections::BTreeMap::<GemDefId, Vec<RuleProgram>>::new();
    for row in &programs {
        owners
            .entry(row.gem.clone())
            .or_default()
            .push(row.program.clone());
    }
    let package = RulePackageInput {
        existing_actor_rules: None,
        ordered_contributions: None,
        effect_applications: None,
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: schema.namespace().clone(),
        release: input.version.clone(),
        semantics_version: key("owned-effective-gem-recipe-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_V13),
        definitions: schema.identity().clone(),
        tables: vec![],
        owners: owners
            .into_iter()
            .map(|(gem, programs)| {
                let owner = SchemaSubject::Definition(gem.address());
                DefinitionRules {
                    owner: owner.clone(),
                    programs: DeclaredSet::partial(
                        programs,
                        vec![SchemaGap {
                            subject: owner,
                            facet: SchemaFacet::GameRules,
                            code: key("recipe-has-no-coverage-authority"),
                        }],
                    ),
                }
            })
            .collect(),
        receivers: DeclaredSet::partial(
            vec![],
            vec![SchemaGap {
                subject: SchemaSubject::Definition(input.bindings[0].gem.address()),
                facet: SchemaFacet::GameRules,
                code: key("recipe-has-no-receiver-authority"),
            }],
        ),
    };
    CompiledRulePackage::compile(
        &package,
        schema,
        RuleLimits {
            max_work: typecheck_work,
            ..RuleLimits::default()
        },
    )
    .map_err(|error| invalid(error.to_string()))?;
    let output = EffectiveGemRecipeOutput {
        schema_version: 1,
        policy,
        definitions: schema.identity().clone(),
        programs,
        work_used: limits.max_work - work,
    };
    wire_size(&output, limits.max_output_bytes, "output bytes")?;
    Ok(output)
}
