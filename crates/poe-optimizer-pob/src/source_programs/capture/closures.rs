//! Actual prototype graph observation. No factory or child closure is executed.
use super::*;
use mlua::MultiValue;
use std::sync::Arc;
mod dump;
pub(crate) use dump::Proto;
const MAX_DUMP: usize = 16 * 1024 * 1024;
const FNEW: u32 = 51;

pub(super) struct Reflection {
    profile: SourceTableRuntimeProfile,
    info: Function,
    bytecode: Function,
    constant: Function,
    uvname: Function,
}
#[derive(Default)]
pub(super) struct Pending {
    roots: BTreeMap<SourceCallbackId, usize>,
    nodes: BTreeMap<usize, Node>,
    callbacks: BTreeMap<usize, SourceCallbackId>,
}
#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub source: ItemSourceSpan,
    pub metadata: Arc<Proto>,
    pub children: BTreeMap<i32, usize>,
}
/// Opaque evidence tied to an observed owner, including never-instantiated child
/// code. Host pointers are discarded before this value crosses the adapter seam.
#[derive(Debug, Clone)]
pub struct ObservedSourceClosureCreations {
    pub(crate) owner: SourceProgramOwner,
    pub(crate) profile: SourceTableRuntimeProfile,
    pub(crate) nodes: BTreeMap<SourceCallbackId, Node>,
}
impl Reflection {
    fn capture(lua: &Lua, profile: SourceTableRuntimeProfile) -> Result<Self> {
        // Reuse the complete fresh-host observable/profile checks; its table
        // evidence remains separately optional and is never inferred here.
        let _ = constructors::Reflection::capture(lua, profile.clone())?;
        let preload: Table = lua.named_registry_value("_PRELOAD")?;
        let loader: Function = preload.raw_get("jit.util")?;
        let module: Table = loader.call(())?;
        let get = |name| -> Result<Function> {
            let function: Function = module.raw_get(name)?;
            if function.info().what != "C" {
                return Err(error("closure reflection must retain original C functions"));
            }
            Ok(function)
        };
        Ok(Self {
            profile,
            info: get("funcinfo")?,
            bytecode: get("funcbc")?,
            constant: get("funck")?,
            uvname: get("funcuvname")?,
        })
    }
    pub(super) fn identity(&self, function: &Function) -> Result<usize> {
        let info: Table = self.info.call(function.clone())?;
        let proto: Value = info.raw_get("proto")?;
        if !matches!(proto, Value::Other(_)) {
            return Err(error("closure reflection has no actual prototype identity"));
        }
        Ok(proto.to_pointer() as usize)
    }
    fn preflight(
        &self,
        value: Value,
        depth: usize,
        budget: &mut usize,
        seen: &mut std::collections::BTreeSet<usize>,
        records: &mut usize,
        limits: (usize, usize),
    ) -> Result<()> {
        if depth > 32 {
            return Err(error("closure prototype graph depth bound"));
        }
        if *records >= limits.1 || *budget >= limits.0 {
            return Err(error("remaining closure observation budget exhausted"));
        }
        let info: Table = self.info.call(value.clone())?;
        let proto: Value = info.raw_get("proto")?;
        if !seen.insert(proto.to_pointer() as usize) {
            return Err(error("duplicate/cyclic original child prototype"));
        }
        if seen.len() > MAX_CALLBACKS {
            return Err(error("closure prototype count bound"));
        }
        let bc: usize = info.raw_get("bytecodes")?;
        let kgc: u32 = info.raw_get("gcconsts")?;
        let kn: usize = info.raw_get("nconsts")?;
        if bc == 0 || bc > 65_536 || kgc as usize > MAX_VALUES || kn > MAX_VALUES {
            return Err(error("closure reflection instruction/constant bound"));
        }
        *records = records
            .checked_add(8 * bc + 4 * kgc as usize + kn + 256)
            .ok_or_else(|| error("closure metadata count overflow"))?;
        if *records > limits.1 {
            return Err(error("remaining closure metadata record bound"));
        }
        // Includes a conservative debug-name/lifetime allowance per instruction.
        *budget = budget
            .checked_add(128 * bc + 16 * kn + 256)
            .ok_or_else(|| error("closure scratch estimate overflow"))?;
        for index in 0..kgc {
            let value: Value = self.constant.call((proto.clone(), -1 - i64::from(index)))?;
            match value {
                Value::Other(_) => {
                    self.preflight(value, depth + 1, budget, seen, records, limits)?
                }
                Value::String(text) => {
                    *budget = budget
                        .checked_add(text.as_bytes().len() + 8)
                        .ok_or_else(|| error("closure constant text overflow"))?;
                }
                Value::Table(table) => {
                    let mut count = 0;
                    for pair in table.pairs::<Value, Value>() {
                        let (key, value) = pair?;
                        count += 1;
                        if count > 50_000 {
                            return Err(error("closure template constant row bound"));
                        }
                        for (position, value) in [key, value].into_iter().enumerate() {
                            let bytes = match value {
                                Value::String(text) => text.as_bytes().len() + 8,
                                Value::Nil
                                | Value::Boolean(_)
                                | Value::Integer(_)
                                | Value::Number(_) => 16,
                                // Pinned expr_table uses the template itself as
                                // the nil/dynamic keyed-value sentinel; bcwrite_ktabk
                                // emits exactly nil for this marker. Never follow it.
                                Value::Table(marker)
                                    if position == 1
                                        && marker.to_pointer() == table.to_pointer() =>
                                {
                                    1
                                }
                                _ => return Err(error("closure template constant type frontier")),
                            };
                            *budget = budget
                                .checked_add(bytes)
                                .ok_or_else(|| error("closure constant estimate overflow"))?;
                        }
                    }
                }
                _ => return Err(error("closure GC constant type frontier")),
            }
            if *budget > limits.0.min(MAX_DUMP) {
                return Err(error("closure dump scratch/byte bound"));
            }
        }
        if *budget > limits.0.min(MAX_DUMP) {
            return Err(error("closure dump scratch/byte bound"));
        }
        Ok(())
    }
    fn observe(
        &self,
        lua: &Lua,
        function: &Function,
        source: &ItemSourceSpan,
        sources: &BTreeMap<String, String>,
        remaining_text: usize,
        remaining_values: usize,
    ) -> Result<(usize, BTreeMap<usize, Node>, usize)> {
        if remaining_text < 256 || remaining_values < 256 {
            return Err(error("remaining closure observation budget exhausted"));
        }
        let span_bytes = sources[&source.path]
            .split_inclusive('\n')
            .skip(source.line as usize - 1)
            .take((source.end_line - source.line + 1) as usize)
            .map(str::len)
            .sum::<usize>();
        let mut estimate = span_bytes
            .checked_mul(2)
            .ok_or_else(|| error("closure debug source estimate overflow"))?;
        let mut records = span_bytes;
        let mut seen = Default::default();
        self.preflight(
            Value::Function(function.clone()),
            0,
            &mut estimate,
            &mut seen,
            &mut records,
            (remaining_text, remaining_values),
        )?;
        let dump_estimate = estimate;
        estimate = estimate
            .checked_add(
                seen.len()
                    .checked_mul(source.path.len() + 128)
                    .ok_or_else(|| error("closure source metadata estimate overflow"))?,
            )
            .ok_or_else(|| error("closure source metadata estimate overflow"))?;
        if estimate > remaining_text.min(MAX_DUMP) {
            return Err(error("remaining closure source metadata byte bound"));
        }
        let bytes = dump::write(lua, function, dump_estimate)?;
        let protos = dump::decode(&bytes)?
            .into_iter()
            .map(Arc::new)
            .collect::<Vec<_>>();
        let mut nodes = BTreeMap::new();
        let root = self.verify_node(
            Value::Function(function.clone()),
            protos.len() - 1,
            &protos,
            source,
            sources,
            &mut nodes,
            0,
        )?;
        if nodes.len() != protos.len() {
            return Err(error("dump/reflection child graph inventory mismatch"));
        }
        let retained_text = nodes.values().try_fold(bytes.len(), |total, node| {
            total
                .checked_add(
                    node.source.path.len() + node.source.sha256.len() + node.metadata.sha256.len(),
                )
                .ok_or_else(|| error("closure retained metadata text overflow"))
        })?;
        if retained_text > remaining_text {
            return Err(error("closure retained metadata text bound"));
        }
        Ok((root, nodes, retained_text))
    }
    #[allow(clippy::too_many_arguments)]
    fn verify_node(
        &self,
        value: Value,
        index: usize,
        protos: &[Arc<Proto>],
        source: &ItemSourceSpan,
        sources: &BTreeMap<String, String>,
        nodes: &mut BTreeMap<usize, Node>,
        depth: usize,
    ) -> Result<usize> {
        if depth > 32 {
            return Err(error("closure reflection depth bound"));
        }
        let metadata = &protos[index];
        let info: Table = self.info.call(value.clone())?;
        let proto: Value = info.raw_get("proto")?;
        if !matches!(proto, Value::Other(_)) {
            return Err(error("closure child is not an actual prototype"));
        }
        let pointer = proto.to_pointer() as usize;
        for (name, expected) in [
            ("linedefined", metadata.first),
            ("lastlinedefined", metadata.last),
            ("bytecodes", metadata.instructions.len() as u32 + 1),
            ("params", metadata.parameters as u32),
            ("stackslots", metadata.frame as u32),
            ("upvalues", metadata.names.len() as u32),
            ("gcconsts", metadata.gc_constants),
            ("nconsts", metadata.numeric_constants),
        ] {
            if info.raw_get::<u32>(name)? != expected {
                return Err(error(format!("closure dump/reflection {name} mismatch")));
            }
        }
        if info.raw_get::<bool>("isvararg")? != metadata.variadic {
            return Err(error("closure vararg mismatch"));
        }
        for (slot, name) in metadata.names.iter().enumerate() {
            if self.uvname.call::<String>((proto.clone(), slot))? != *name {
                return Err(error("closure ordered upvalue name mismatch"));
            }
        }
        for (offset, word) in metadata.instructions.iter().enumerate() {
            let values: MultiValue = self.bytecode.call((proto.clone(), offset + 1))?;
            let current = match values.front() {
                Some(Value::Integer(n)) => *n as i32 as u32,
                Some(Value::Number(n)) if n.is_finite() && n.fract() == 0.0 => *n as i32 as u32,
                _ => return Err(error("invalid reflected closure instruction")),
            };
            // The trusted dump normalizes patched loops. Their exact branch D
            // comes from the original trace start instruction, not live J* D.
            let old = word & 255;
            let op = current & 255;
            let patched = matches!(
                (old, op),
                (77, 78) | (79, 80) | (79, 81) | (82, 83) | (82, 84) | (85, 86) | (85, 87)
            );
            if current != *word && !(patched && (current & 0xff00) == (word & 0xff00)) {
                return Err(error(
                    "closure bytecode differs outside pinned loop normalization",
                ));
            }
            if old == FNEW
                && !metadata
                    .children
                    .contains_key(&(-1 - ((*word >> 16) as i32)))
            {
                return Err(error("FNEW lacks exact decoded child constant"));
            }
        }
        let text = sources
            .get(&source.path)
            .ok_or_else(|| error("closure child source file absent"))?;
        if metadata.first < source.line || metadata.last > source.end_line || metadata.first == 0 {
            return Err(error("child prototype outside authenticated parent source"));
        }
        let body = source_slice(text, metadata.first, metadata.last)?;
        let span = ItemSourceSpan {
            path: source.path.clone(),
            line: metadata.first,
            end_line: metadata.last,
            sha256: hash(body.as_bytes()),
        };
        let mut children = BTreeMap::new();
        for (constant, child) in &metadata.children {
            let value: Value = self.constant.call((proto.clone(), *constant))?;
            let child =
                self.verify_node(value, *child, protos, &span, sources, nodes, depth + 1)?;
            children.insert(*constant, child);
        }
        if nodes
            .insert(
                pointer,
                Node {
                    source: span,
                    metadata: metadata.clone(),
                    children,
                },
            )
            .is_some()
        {
            return Err(error("duplicate reflected child identity"));
        }
        Ok(pointer)
    }
}
impl SourceClosureObserver {
    /// Opt into uninstantiated prototype discovery using a caller-attested host
    /// build. Also retains the existing independent table-constructor evidence.
    pub fn capture_before_source_with_closures(
        lua: &Lua,
        build_attestation: SourceTableRuntimeProfile,
    ) -> Result<Self> {
        let mut observer =
            Self::capture_before_source_with_constructors(lua, build_attestation.clone())?;
        observer.closure_reflection = Some(Reflection::capture(lua, build_attestation)?);
        Ok(observer)
    }
    pub(super) fn bind_closure_creations(
        &self,
        owner: &SourceProgramOwner,
        pending: Pending,
    ) -> Option<ObservedSourceClosureCreations> {
        self.closure_reflection.as_ref().map(|reflection| {
            let mut nodes = BTreeMap::new();
            for (callback, pointer) in pending.roots {
                let mut node = pending.nodes[&pointer].clone();
                node.children = node
                    .children
                    .into_iter()
                    .map(|(index, pointer)| (index, pending.callbacks[&pointer].0 as usize))
                    .collect();
                nodes.insert(callback, node);
            }
            ObservedSourceClosureCreations {
                owner: owner.clone(),
                profile: reflection.profile.clone(),
                nodes,
            }
        })
    }
}
impl Graph<'_> {
    pub(super) fn observe_closure_creation(
        &mut self,
        id: SourceCallbackId,
        function: &Function,
        source: &ItemSourceSpan,
    ) -> Result<()> {
        let Some(reflection) = &self.observer.closure_reflection else {
            return Ok(());
        };
        let (root, nodes, bytes) = reflection
            .observe(
                &self.observer.lua,
                function,
                source,
                self.sources,
                MAX_TEXT_BYTES.saturating_sub(self.text_bytes),
                MAX_VALUES.saturating_sub(self.values),
            )
            .map_err(|e| {
                error(format!(
                    "closure prototype {id:?} {}:{}..{}: {e}",
                    source.path, source.line, source.end_line
                ))
            })?;
        self.text(bytes)?;
        for (pointer, node) in nodes {
            self.values = self
                .values
                .checked_add(
                    2 * node.metadata.instructions.len()
                        + node.metadata.locals.len()
                        + 2 * node.metadata.names.len()
                        + 2 * node.children.len()
                        + 1,
                )
                .ok_or_else(|| error("closure graph count overflow"))?;
            if self.values > MAX_VALUES {
                return Err(error("closure graph count bound"));
            }
            if let Some(previous) = self.closure_observations.nodes.get(&pointer) {
                if previous.metadata != node.metadata || previous.source != node.source {
                    return Err(error("actual prototype changed between observations"));
                }
            } else {
                self.closure_observations.nodes.insert(pointer, node);
            }
        }
        self.closure_observations.roots.insert(id, root);
        Ok(())
    }
    pub(super) fn finish_closure_creations(
        &mut self,
        prototypes: &mut SourceClosurePrototypes,
    ) -> Result<()> {
        if self.observer.closure_reflection.is_none() {
            return Ok(());
        }
        // Precharge every retained source/name/map copy before publishing owner
        // child callbacks or bound evidence. Proto vectors themselves are shared.
        let mut copy_text = 0usize;
        let mut copy_values = 0usize;
        for node in self.closure_observations.nodes.values() {
            copy_text = copy_text
                .checked_add(
                    3 * (node.source.path.len() + node.source.sha256.len())
                        + 128
                        + node.metadata.names.iter().map(String::len).sum::<usize>(),
                )
                .ok_or_else(|| error("closure retained text overflow"))?;
            copy_values = copy_values
                .checked_add(
                    node.children.len()
                        + node.metadata.names.len()
                        + node
                            .metadata
                            .instructions
                            .iter()
                            .filter(|word| matches!(**word & 255, 52 | 53))
                            .count()
                        + 3,
                )
                .ok_or_else(|| error("closure retained record overflow"))?;
        }
        for pointer in self.closure_observations.roots.values() {
            let node = &self.closure_observations.nodes[pointer];
            copy_text = copy_text
                .checked_add(node.source.path.len() + node.source.sha256.len())
                .ok_or_else(|| error("closure root text overflow"))?;
            copy_values = copy_values
                .checked_add(node.children.len() + 1)
                .ok_or_else(|| error("closure root record overflow"))?;
        }
        self.text(copy_text)?;
        self.values = self
            .values
            .checked_add(copy_values)
            .ok_or_else(|| error("closure retained record overflow"))?;
        if self.values > MAX_VALUES {
            return Err(error("closure retained record bound"));
        }
        for prototype in &prototypes.prototypes {
            if let Some(pointer) = self.closure_observations.roots.get(&prototype.callback) {
                self.closure_observations
                    .callbacks
                    .insert(*pointer, prototype.callback);
            }
        }
        // Only a child edge requires an all-live prototype. Immutable observed
        // parent callbacks remain distinct from their possible created instances.
        let children = self
            .closure_observations
            .nodes
            .values()
            .flat_map(|node| node.children.values().copied())
            .collect::<std::collections::BTreeSet<_>>();
        for pointer in children {
            if self.closure_observations.callbacks.contains_key(&pointer) {
                continue;
            }
            if self.callbacks.len() >= MAX_CALLBACKS {
                return Err(error("source child callback bound"));
            }
            let node = &self.closure_observations.nodes[&pointer];
            let callback = SourceCallbackId(self.callbacks.len() as u32 + 1);
            self.callbacks.push(SourceCallback {
                kind: SourceCallbackKind::Lua {
                    source: node.source.clone(),
                },
                upvalues: node
                    .metadata
                    .names
                    .iter()
                    .map(|name| SourceUpvalue {
                        name: name.clone(),
                        value: SourceValue::LiveCapture {},
                    })
                    .collect(),
                environment: SourceEnvironment::OriginalGlobals,
            });
            prototypes
                .prototypes
                .push(SourceClosurePrototype { callback });
            self.closure_observations
                .callbacks
                .insert(pointer, callback);
            self.closure_observations.roots.insert(callback, pointer);
            self.constructor_observations.callbacks.insert(
                callback,
                constructors::CallbackObservation {
                    source: node.source.clone(),
                    sha256: node.metadata.sha256.clone(),
                    constructors: node
                        .metadata
                        .instructions
                        .iter()
                        .enumerate()
                        .filter(|(_, word)| matches!(**word & 255, 52 | 53))
                        .map(|(pc, word)| constructors::Instruction {
                            pc: pc as u32 + 1,
                            word: *word,
                            line: node.metadata.lines[pc],
                        })
                        .collect(),
                    unsupported: None,
                },
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

pub(crate) fn source_slice(text: &str, first: u32, last: u32) -> Result<&str> {
    if first == 0 || last < first {
        return Err(error("invalid closure source line range"));
    }
    let start = text
        .split_inclusive('\n')
        .take(first as usize - 1)
        .map(str::len)
        .sum::<usize>();
    let length = text
        .split_inclusive('\n')
        .skip(first as usize - 1)
        .take((last - first + 1) as usize)
        .map(str::len)
        .sum::<usize>();
    text.get(start..start + length)
        .ok_or_else(|| error("closure source line range outside file"))
}
