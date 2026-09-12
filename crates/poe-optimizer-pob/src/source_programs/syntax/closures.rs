use super::*;
use poe_optimizer_data::source_program::*;

#[derive(Clone)]
pub(super) struct Declaration {
    pub name: String,
    pub debug_index: usize,
    pub location: SourceProgramLocation,
    pub kind: SourceProgramClosureLocalKind,
    pub recursive: bool,
}
#[derive(Clone)]
pub(super) struct Creation {
    expression: SourceProgramLocation,
    child: SourceCallbackId,
    pc: u32,
    captures: Vec<SourceProgramCaptureOrigin>,
    /// Typed local -> actual parent register.
    locals: BTreeMap<u16, u8>,
}
impl Lowerer<'_, '_> {
    pub(super) fn closure_expr(&mut self, depth: usize) -> LowerResult<Info> {
        if depth > 32 {
            return Err("nested function depth bound".into());
        }
        let start = self.offset();
        let bound = self
            .authorization
            .closure_functions
            .get(&self.callback_id)
            .ok_or("nested function requires actual prototype observations")?;
        let (child, pc) = *bound
            .children
            .get(&((start - self.function_start) as u32))
            .ok_or("function expression lacks an authenticated FNEW site")?;
        let child_bound = &self.authorization.closure_functions[&child];
        let end = complete_function_token_end(&self.tokens, self.at)?;
        let end_offset = self.tokens[end - 1].end;
        let expected = &child_bound.provenance;
        if hash(&self.body.as_bytes()[start..end_offset]) != expected.function_sha256 {
            return Err("child source expression differs from observed prototype".into());
        }
        let mut captures = Vec::new();
        let mut locals = BTreeMap::new();
        for (slot, descriptor) in child_bound.node.metadata.descriptors.iter().enumerate() {
            let name = &child_bound.node.metadata.names[slot];
            if descriptor & 0x8000 != 0 {
                let register = (descriptor & 255) as u8;
                let active = bound
                    .node
                    .metadata
                    .locals
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| entry.start <= pc && pc < entry.end)
                    .collect::<Vec<_>>();
                let debug_index = if let Some((index, _)) = active.get(register as usize) {
                    *index
                } else {
                    // local function's own slot becomes debug-visible immediately
                    // after FNEW; only that exact declaration/store can use it.
                    let instruction = bound.node.metadata.instructions[pc as usize - 1];
                    let found = self.declarations.iter().enumerate().find(|(_, decl)| {
                        decl.recursive
                            && decl.name == *name
                            && bound.node.metadata.locals[decl.debug_index].start == pc + 1
                            && ((instruction >> 8) & 255) == u32::from(register)
                    });
                    found
                        .map(|(_, decl)| decl.debug_index)
                        .ok_or("captured local is not active at original FNEW")?
                };
                let (local, declaration) = self
                    .declarations
                    .iter()
                    .enumerate()
                    .find(|(_, decl)| decl.debug_index == debug_index)
                    .ok_or("source capture register has no lexical declaration")?;
                if declaration.name != *name || self.local(name) != Some(local as u16) {
                    return Err("source capture register/name/lexical visibility mismatch".into());
                }
                captures.push(SourceProgramCaptureOrigin::Local {
                    local: local as u16,
                });
                locals.insert(local as u16, register);
            } else {
                if !self
                    .authorization
                    .closure_prototypes
                    .contains_key(&self.callback_id)
                {
                    return Err(
                        "inherited child captures require an actual session parent closure".into(),
                    );
                }
                let upvalue = *descriptor;
                if descriptor & 0xc000 != 0
                    || self
                        .callback
                        .upvalues
                        .get(upvalue as usize)
                        .is_none_or(|capture| capture.name != *name)
                {
                    return Err("inherited source capture descriptor mismatch".into());
                }
                captures.push(SourceProgramCaptureOrigin::ParentCapture { upvalue });
            }
        }
        self.at = end;
        let expression = self.location(start, end_offset);
        self.creations.push(Creation {
            expression,
            child,
            pc,
            captures: captures.clone(),
            locals,
        });
        let prototype = *self
            .authorization
            .closure_prototypes
            .get(&child)
            .ok_or("child prototype absent from shared owner")?;
        self.node(
            ParserProgramExprKind::CreateClosure {
                prototype,
                captures,
            },
            start,
            end_offset,
            1,
        )
    }
    pub(super) fn declare_observed(&mut self, name: &str, local: u16) -> LowerResult<()> {
        let Some(bound) = self.authorization.closure_functions.get(&self.callback_id) else {
            return Ok(());
        };
        let (debug_index, entry) = bound
            .node
            .metadata
            .locals
            .iter()
            .enumerate()
            .filter(|(_, entry)| !entry.hidden)
            .nth(local as usize)
            .ok_or("typed declaration missing from actual source debug inventory")?;
        if entry.name != name {
            return Err(format!(
                "source lexical declaration order mismatch: expected {}, got {name}",
                entry.name
            ));
        }
        self.declarations.push(Declaration {
            name: name.into(),
            debug_index,
            location: self.location(self.offset(), self.offset().max(self.function_start + 1)),
            kind: SourceProgramClosureLocalKind::Declare,
            recursive: false,
        });
        Ok(())
    }
    pub(super) fn mark_declarations(
        &mut self,
        locals: &[u16],
        location: SourceProgramLocation,
        kind: SourceProgramClosureLocalKind,
    ) {
        for local in locals {
            if let Some(declaration) = self.declarations.get_mut(*local as usize) {
                declaration.location = location;
                declaration.kind = kind;
            }
        }
    }
    pub(super) fn finish_creations(
        &self,
        provenance: &SourceProgramProvenance,
    ) -> LowerResult<Vec<SourceProgramClosureCreation>> {
        let Some(bound) = self.authorization.closure_functions.get(&self.callback_id) else {
            return Ok(vec![]);
        };
        if *provenance != bound.provenance
            || self.parameters != bound.node.metadata.parameters as u16
            || self.variadic != bound.node.metadata.variadic
            || self.declarations.len()
                != bound
                    .node
                    .metadata
                    .locals
                    .iter()
                    .filter(|entry| !entry.hidden)
                    .count()
        {
            return Err("complete function lexical/prototype inventory mismatch".into());
        }
        self.creations
            .iter()
            .map(|site| {
                let child = &self.authorization.closure_functions[&site.child];
                Ok(SourceProgramClosureCreation {
                    callback: self.callback_id,
                    provenance: provenance.clone(),
                    expression: site.expression,
                    prototype: self.authorization.closure_prototypes[&site.child],
                    child_provenance: child.provenance.clone(),
                    bytecode_sha256: bound.node.metadata.sha256.clone(),
                    bytecode_pc: site.pc,
                    instruction: bound.node.metadata.instructions[site.pc as usize - 1],
                    child_bytecode_sha256: child.node.metadata.sha256.clone(),
                    captures: site.captures.clone(),
                    capture_descriptors: child.node.metadata.descriptors.clone(),
                    local_bindings: site
                        .locals
                        .iter()
                        .map(|(local, register)| {
                            let declaration = &self.declarations[*local as usize];
                            SourceProgramClosureLocalBinding {
                                local: *local,
                                register: *register,
                                declaration: declaration.location,
                                kind: declaration.kind,
                            }
                        })
                        .collect(),
                })
            })
            .collect()
    }
}
