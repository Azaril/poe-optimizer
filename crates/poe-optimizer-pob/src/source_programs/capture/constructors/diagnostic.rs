//! Opt-in, same-host diagnostic witnesses. Queries do not grant execution admission;
//! the separate reserved-template adapter validates and attaches its closed family.
use super::*;
use serde::Serialize;
use std::{rc::Rc, sync::Arc};

const MAX_TARGETS: usize = 32;

#[derive(Debug, Clone)]
pub(super) struct Binding {
    identity: Arc<()>,
    callback: SourceCallbackId,
}
pub(in crate::source_programs::capture) struct Target {
    identity: Arc<()>,
    function: Function,
    prototype: usize,
    reflection: Reflection,
    constant: Function,
    next: Function,
}
/// An exact live function retained before graph observation. It cannot be
/// rebound by a name, source range, equivalent body, or prototype-only match.
#[derive(Clone)]
pub struct ConstructorDiagnosticTarget(Rc<Target>);

#[derive(Debug, Clone, Copy)]
pub struct ConstructorDiagnosticLimits {
    pub max_template_rows: usize,
    pub max_text_bytes: usize,
    pub max_window_instructions: usize,
    pub max_control_flow_instructions: usize,
}
impl Default for ConstructorDiagnosticLimits {
    fn default() -> Self {
        Self {
            max_template_rows: 256,
            max_text_bytes: 65_536,
            max_window_instructions: 512,
            max_control_flow_instructions: 1024,
        }
    }
}
impl ConstructorDiagnosticLimits {
    fn validate(self) -> Result<Self> {
        if self.max_template_rows > 4096
            || self.max_text_bytes > 1024 * 1024
            || self.max_window_instructions == 0
            || self.max_window_instructions > 4096
            || self.max_control_flow_instructions > 4096
        {
            return Err(error("constructor diagnostic limit bound"));
        }
        Ok(self)
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ConstructorDiagnosticRequest {
    pub callback: SourceCallbackId,
    pub expression: SourceProgramLocation,
    /// First original instruction on this source line, not a post-instruction
    /// hook claim. The window includes allocation through that PC and neighbors.
    pub continuation_line: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConstructorDiagnosticInstruction {
    pub pc: u32,
    pub word: u32,
    pub mode: u32,
    pub line: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ConstructorTemplateValue {
    Nil,
    Boolean(bool),
    NumberBits(u64),
    Bytes(Vec<u8>),
    /// Exact template self-reference, used by pinned TDUP for a reserved nil row.
    SelfMarker,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConstructorTemplateRow {
    pub key: ConstructorTemplateValue,
    pub value: ConstructorTemplateValue,
}
/// Data only. Pointer identity is available separately through retained handles;
/// this report cannot authenticate a function or template after serialization.
#[derive(Debug, Clone, Serialize)]
pub struct ConstructorDiagnosticReport {
    pub callback: SourceCallbackId,
    pub provenance: SourceProgramProvenance,
    pub expression: SourceProgramLocation,
    pub bytecode_sha256: String,
    pub instruction: ConstructorDiagnosticInstruction,
    pub template_constant_index: Option<i32>,
    pub template_rows: Vec<ConstructorTemplateRow>,
    pub continuation_pc: Option<u32>,
    /// Every actual source instruction carrying the requested continuation line.
    pub continuation_pcs: Vec<u32>,
    pub instruction_window: Vec<ConstructorDiagnosticInstruction>,
    /// Complete original jump/test inventory, including entries outside the
    /// window. Consumers must interpret pinned modes/opcodes before CFG claims.
    pub control_flow_instructions: Vec<ConstructorDiagnosticInstruction>,
    pub post_store_proof_unavailable: String,
}
pub enum ConstructorDiagnostic {
    Observed(Box<ConstructorDiagnosticWitness>),
    /// Missing callback/range or non-bijective complete lexical mapping.
    Unavailable(String),
}
/// Host-bound observation, deliberately separate from the serializable report.
/// Template rows are query-time contents, not an allocation-completion snapshot.
pub struct ConstructorDiagnosticWitness {
    target: ConstructorDiagnosticTarget,
    catalog: SourceProgramCatalog,
    report: ConstructorDiagnosticReport,
    template: Option<Table>,
    limits: ConstructorDiagnosticLimits,
}
impl SourceClosureObserver {
    /// Requires the existing combined closure/constructor observation mode so
    /// original `funck` was retained before source loading. No extra diagnostic
    /// function handles or bindings are retained unless this method is called.
    pub fn retain_constructor_diagnostic_target(
        &self,
        lua: &Lua,
        function: &Function,
    ) -> Result<ConstructorDiagnosticTarget> {
        if lua.globals().to_pointer() != self.globals.to_pointer()
            || function
                .environment()
                .is_none_or(|environment| environment.to_pointer() != self.globals.to_pointer())
        {
            return Err(error(
                "constructor diagnostic target belongs to another host/environment",
            ));
        }
        let reflection = self
            .constructor_reflection
            .as_ref()
            .ok_or_else(|| error("constructor diagnostic observation is disabled"))?;
        let closures = self
            .closure_reflection
            .as_ref()
            .ok_or_else(|| error("constructor diagnostic requires retained closure reflection"))?;
        let mut targets = self.constructor_diagnostic_targets.borrow_mut();
        targets.retain(|target| target.strong_count() != 0);
        if targets.len() >= MAX_TARGETS {
            return Err(error("constructor diagnostic target count bound"));
        }
        let prototype = closures.identity(function)?;
        let target = Rc::new(Target {
            identity: Arc::new(()),
            function: function.clone(),
            prototype,
            reflection: Reflection {
                profile: reflection.profile.clone(),
                info: reflection.info.clone(),
                bytecode: reflection.bytecode.clone(),
            },
            constant: closures.constant.clone(),
            next: self.iterator_primitives.next.clone(),
        });
        targets.push(Rc::downgrade(&target));
        Ok(ConstructorDiagnosticTarget(target))
    }
}
impl Graph<'_> {
    pub(in crate::source_programs::capture) fn bind_constructor_diagnostic_targets(
        &mut self,
        callback: SourceCallbackId,
        function: &Function,
    ) -> Result<()> {
        for target in self
            .observer
            .constructor_diagnostic_targets
            .borrow()
            .iter()
            .filter_map(|target| target.upgrade())
        {
            if target.function.to_pointer() != function.to_pointer() {
                continue;
            }
            let bindings = &mut self.constructor_observations.diagnostic_bindings;
            if let Some(binding) = bindings
                .iter()
                .find(|binding| Arc::ptr_eq(&binding.identity, &target.identity))
            {
                if binding.callback != callback {
                    return Err(error("constructor diagnostic callback identity changed"));
                }
                continue;
            }
            if self.values >= MAX_VALUES || bindings.len() >= MAX_TARGETS {
                return Err(error("constructor diagnostic binding count bound"));
            }
            self.values += 1;
            bindings.push(Binding {
                identity: target.identity.clone(),
                callback,
            });
        }
        Ok(())
    }
}
impl ConstructorDiagnosticTarget {
    pub fn function(&self) -> &Function {
        &self.0.function
    }
    /// Authentication comes from the ticket binding observed in this owner.
    /// Mapping a source position does not certify arbitrary authored IR semantics.
    pub fn inspect(
        &self,
        sources: &BTreeMap<String, String>,
        observations: &ObservedSourceConstructors,
        catalog: &SourceProgramCatalog,
        request: ConstructorDiagnosticRequest,
        limits: ConstructorDiagnosticLimits,
    ) -> Result<ConstructorDiagnostic> {
        let limits = limits.validate()?;
        observations.validate_owner(catalog.owner())?;
        if !observations.diagnostic_bindings.iter().any(|binding| {
            Arc::ptr_eq(&binding.identity, &self.0.identity) && binding.callback == request.callback
        }) {
            return Err(error(
                "constructor diagnostic exact function was not observed for this callback",
            ));
        }
        let Some(observation) = observations.callbacks.get(&request.callback) else {
            return Ok(ConstructorDiagnostic::Unavailable(
                "callback has no original constructor inventory".into(),
            ));
        };
        self.0.verify_identity()?;
        let Some(program) = catalog.for_callback(request.callback) else {
            return Ok(ConstructorDiagnostic::Unavailable(
                "callback has no complete lowered body".into(),
            ));
        };
        let selected = match crate::source_programs::constructors::diagnostic_site(
            sources,
            catalog.owner(),
            program,
            observation,
            request.expression,
        )? {
            Ok(selected) => selected,
            Err(reason) => return Ok(ConstructorDiagnostic::Unavailable(reason)),
        };
        let metadata_bytes = program.provenance.source.path.len()
            + program.provenance.source.sha256.len()
            + program.provenance.function_sha256.len()
            + observation.sha256.len()
            + 256;
        if metadata_bytes > limits.max_text_bytes {
            return Err(error("constructor diagnostic retained text bound"));
        }
        let instructions = self.0.instructions()?;
        self.0.verify_observation(observation, &instructions)?;
        let instruction = instructions
            .iter()
            .find(|instruction| instruction.pc == selected.pc)
            .ok_or_else(|| error("constructor diagnostic selected PC disappeared"))?
            .clone();
        let continuation_pcs = request
            .continuation_line
            .map(|line| {
                instructions
                    .iter()
                    .filter(|instruction| instruction.line == line)
                    .map(|instruction| instruction.pc)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let continuation_pc = continuation_pcs.first().copied();
        if request.continuation_line.is_some() && continuation_pc.is_none_or(|pc| pc <= selected.pc)
        {
            return Ok(ConstructorDiagnostic::Unavailable(
                "requested continuation has no forward first-line boundary".into(),
            ));
        }
        let first = selected.pc.saturating_sub(2).max(1);
        let last = continuation_pcs
            .last()
            .copied()
            .unwrap_or(selected.pc)
            .saturating_add(2)
            .min(instructions.len() as u32);
        if (last - first + 1) as usize > limits.max_window_instructions {
            return Err(error("constructor diagnostic instruction window bound"));
        }
        if instructions
            .iter()
            .filter(|instruction| control_flow(instruction))
            .count()
            > limits.max_control_flow_instructions
        {
            return Err(error("constructor diagnostic control-flow inventory bound"));
        }
        let template_constant_index =
            (selected.word & 255 == TDUP).then_some(-1 - (selected.word >> 16) as i32);
        let template = self.0.template(template_constant_index)?;
        let template_rows = self.0.rows(template.as_ref(), limits, metadata_bytes)?;
        let report = ConstructorDiagnosticReport {
            callback: request.callback, provenance: program.provenance.clone(), expression: request.expression,
            bytecode_sha256: observation.sha256.clone(), instruction, template_constant_index, template_rows,
            continuation_pc,
            continuation_pcs,
            instruction_window: instructions.iter().filter(|instruction| (first..=last).contains(&instruction.pc)).cloned().collect(),
            control_flow_instructions: instructions.into_iter().filter(control_flow).collect(),
            post_store_proof_unavailable: "raw instruction evidence; dominance, destination register and alternate entries require a separate pinned control-flow check".into(),
        };
        let witness = ConstructorDiagnosticWitness {
            target: self.clone(),
            catalog: catalog.clone(),
            report,
            template,
            limits,
        };
        witness.verify_unchanged()?;
        Ok(ConstructorDiagnostic::Observed(Box::new(witness)))
    }
}
impl ConstructorDiagnosticWitness {
    pub fn report(&self) -> &ConstructorDiagnosticReport {
        &self.report
    }
    pub fn function(&self) -> &Function {
        self.target.function()
    }
    pub fn template(&self) -> Option<&Table> {
        self.template.as_ref()
    }
    pub fn catalog(&self) -> &SourceProgramCatalog {
        &self.catalog
    }
    pub(crate) fn source_profile(&self) -> &SourceTableRuntimeProfile {
        &self.target.0.reflection.profile
    }
    /// Read the exact string constant referenced by a retained GGET instruction.
    /// This is a bounded query against the original Function, not a lookup in its
    /// mutable environment. Each returned name is owned by the diagnostic caller.
    pub fn global_name(&self, pc: u32, maximum_bytes: usize) -> Result<Vec<u8>> {
        if maximum_bytes > self.limits.max_text_bytes {
            return Err(error("constructor diagnostic global-name byte bound"));
        }
        let instruction = self
            .report
            .instruction_window
            .iter()
            .find(|i| i.pc == pc)
            .ok_or_else(|| error("global-name query is outside the retained instruction window"))?;
        if instruction.word & 255 != 54 {
            return Err(error(
                "global-name query requires an original GGET instruction",
            ));
        }
        self.verify_unchanged()?;
        let constant_index = -1 - (instruction.word >> 16) as i32;
        let value: Value = self
            .target
            .0
            .constant
            .call((self.function().clone(), constant_index))?;
        let Value::String(name) = value else {
            return Err(error("original GGET constant is not a string"));
        };
        let bytes = name.as_bytes();
        if bytes.len() > maximum_bytes {
            return Err(error("constructor diagnostic global-name byte bound"));
        }
        Ok(bytes.to_vec())
    }
    pub fn verify_unchanged(&self) -> Result<()> {
        self.target.0.verify_identity()?;
        let instructions = self.target.0.instructions()?;
        let (current, _) = self.target.0.reflection.observe(
            self.function(),
            &self.report.provenance.source,
            MAX_BYTECODES as usize,
            true,
        )?;
        if current.sha256 != self.report.bytecode_sha256 {
            return Err(error("constructor diagnostic bytecode differs from query"));
        }
        self.target.0.verify_observation(&current, &instructions)?;
        if let Some(pc) = self.report.continuation_pc {
            let line = self
                .report
                .instruction_window
                .iter()
                .find(|i| i.pc == pc)
                .ok_or_else(|| error("constructor diagnostic missing retained continuation"))?
                .line;
            if instructions
                .iter()
                .filter(|i| i.line == line)
                .map(|i| i.pc)
                .ne(self.report.continuation_pcs.iter().copied())
            {
                return Err(error(
                    "constructor diagnostic complete continuation-line inventory changed",
                ));
            }
        }
        for original in &self.report.instruction_window {
            if instructions.get(original.pc as usize - 1) != Some(original) {
                return Err(error("constructor diagnostic instruction/line changed"));
            }
        }
        if instructions.into_iter().filter(control_flow).ne(self
            .report
            .control_flow_instructions
            .iter()
            .cloned())
        {
            return Err(error(
                "constructor diagnostic control-flow inventory changed",
            ));
        }
        let current = self
            .target
            .0
            .template(self.report.template_constant_index)?;
        if current.as_ref().map(Table::to_pointer) != self.template.as_ref().map(Table::to_pointer)
        {
            return Err(error(
                "constructor diagnostic original template identity changed",
            ));
        }
        let metadata_bytes = self.report.provenance.source.path.len()
            + self.report.provenance.source.sha256.len()
            + self.report.provenance.function_sha256.len()
            + self.report.bytecode_sha256.len()
            + 256;
        if self
            .target
            .0
            .rows(current.as_ref(), self.limits, metadata_bytes)?
            != self.report.template_rows
        {
            return Err(error("constructor diagnostic template rows/order changed"));
        }
        Ok(())
    }
}
fn control_flow(instruction: &ConstructorDiagnosticInstruction) -> bool {
    // Pinned BCMjump = 13 in the original reflected mode. Include comparison/
    // test skips, return/tail operations and JIT entries that require extra care.
    (instruction.mode >> 7) & 15 == 13
        || matches!(instruction.word & 255, 0..=17 | 67..=68 | 73..=76 | 81 | 84 | 87)
}
impl Target {
    fn verify_identity(&self) -> Result<()> {
        let info: Table = self.reflection.info.call(self.function.clone())?;
        let proto: Value = info.raw_get("proto")?;
        if !matches!(proto, Value::Other(_)) || proto.to_pointer() as usize != self.prototype {
            return Err(error(
                "constructor diagnostic original function/prototype changed",
            ));
        }
        Ok(())
    }
    fn instructions(&self) -> Result<Vec<ConstructorDiagnosticInstruction>> {
        let info: Table = self.reflection.info.call(self.function.clone())?;
        let count: u32 = info.raw_get("bytecodes")?;
        if !(1..=MAX_BYTECODES).contains(&count) {
            return Err(error("constructor diagnostic bytecode count bound"));
        }
        let mut instructions = Vec::with_capacity(count as usize - 1);
        for pc in 1..count {
            let values: MultiValue = self.reflection.bytecode.call((self.function.clone(), pc))?;
            if values.len() != 2 {
                return Err(error(
                    "constructor diagnostic incomplete original instruction",
                ));
            }
            let word = integer(&values[0])?;
            let mode = integer(&values[1])?;
            let info: Table = self.reflection.info.call((self.function.clone(), pc))?;
            instructions.push(ConstructorDiagnosticInstruction {
                pc,
                word,
                mode,
                line: info.raw_get("currentline")?,
            });
        }
        if !self
            .reflection
            .bytecode
            .call::<MultiValue>((self.function.clone(), count))?
            .is_empty()
        {
            return Err(error(
                "constructor diagnostic original instruction inventory changed",
            ));
        }
        Ok(instructions)
    }
    fn verify_observation(
        &self,
        observation: &CallbackObservation,
        instructions: &[ConstructorDiagnosticInstruction],
    ) -> Result<()> {
        let (current, _) = self.reflection.observe(
            &self.function,
            &observation.source,
            MAX_BYTECODES as usize,
            true,
        )?;
        if &current != observation {
            return Err(error(
                "constructor diagnostic bytecode differs from owner observation",
            ));
        }
        let mut digest = Sha256::new();
        digest.update(b"poe-source-bytecode-v1\0");
        digest.update((instructions.len() as u32 + 1).to_le_bytes());
        for instruction in instructions {
            digest.update(instruction.word.to_le_bytes());
        }
        if format!("{:x}", digest.finalize()) != observation.sha256 {
            return Err(error("constructor diagnostic instruction scan changed"));
        }
        Ok(())
    }
    fn template(&self, index: Option<i32>) -> Result<Option<Table>> {
        let Some(index) = index else {
            return Ok(None);
        };
        let value: Value = self.constant.call((self.function.clone(), index))?;
        let Value::Table(table) = value else {
            return Err(error(
                "constructor diagnostic TDUP constant is not an actual table",
            ));
        };
        plain(&table, "constructor template")?;
        Ok(Some(table))
    }
    fn rows(
        &self,
        table: Option<&Table>,
        limits: ConstructorDiagnosticLimits,
        mut bytes: usize,
    ) -> Result<Vec<ConstructorTemplateRow>> {
        let Some(table) = table else {
            return Ok(Vec::new());
        };
        let mut rows = Vec::new();
        let mut previous = Value::Nil;
        loop {
            let mut pack: MultiValue = self.next.call((table.clone(), previous))?;
            if pack.len() == 1 && matches!(pack.front(), Some(Value::Nil)) {
                break;
            }
            if pack.len() != 2 || matches!(pack.front(), Some(Value::Nil)) {
                return Err(error("constructor diagnostic original next result"));
            }
            if rows.len() >= limits.max_template_rows {
                return Err(error("constructor diagnostic template row bound"));
            }
            let key = pack.pop_front().expect("checked key");
            let value = pack.pop_front().expect("checked value");
            let observed_key =
                template_value(&key, table, false, &mut bytes, limits.max_text_bytes)?;
            let observed_value =
                template_value(&value, table, true, &mut bytes, limits.max_text_bytes)?;
            rows.push(ConstructorTemplateRow {
                key: observed_key,
                value: observed_value,
            });
            previous = key;
        }
        Ok(rows)
    }
}
fn integer(value: &Value) -> Result<u32> {
    match value {
        Value::Integer(value) => Ok(i32::try_from(*value).map_err(error)? as u32),
        Value::Number(value)
            if value.is_finite()
                && value.fract() == 0.0
                && (i32::MIN as f64..=i32::MAX as f64).contains(value) =>
        {
            Ok(*value as i32 as u32)
        }
        _ => Err(error("constructor diagnostic invalid reflected integer")),
    }
}
fn template_value(
    value: &Value,
    table: &Table,
    allow_self: bool,
    bytes: &mut usize,
    maximum: usize,
) -> Result<ConstructorTemplateValue> {
    Ok(match value {
        Value::Nil => ConstructorTemplateValue::Nil,
        Value::Boolean(value) => ConstructorTemplateValue::Boolean(*value),
        Value::Integer(value) => ConstructorTemplateValue::NumberBits((*value as f64).to_bits()),
        Value::Number(value) => ConstructorTemplateValue::NumberBits(value.to_bits()),
        Value::String(value) => {
            *bytes = bytes
                .checked_add(value.as_bytes().len())
                .ok_or_else(|| error("constructor diagnostic text overflow"))?;
            if *bytes > maximum {
                return Err(error("constructor diagnostic retained text bound"));
            }
            ConstructorTemplateValue::Bytes(value.as_bytes().to_vec())
        }
        Value::Table(value) if allow_self && value.to_pointer() == table.to_pointer() => {
            ConstructorTemplateValue::SelfMarker
        }
        _ => {
            return Err(error(
                "constructor diagnostic template key/value kind unavailable",
            ));
        }
    })
}

#[cfg(test)]
mod tests;
