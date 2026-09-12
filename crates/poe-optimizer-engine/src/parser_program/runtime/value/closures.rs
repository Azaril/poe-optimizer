//! Session-owned closure identities and shared capture cells on the existing heap.
use super::{
    Error, Heap, Result, TableBehavior, TableRef, V, append_staged, extend_staged_map, index,
    input_value, validate_input,
};
use poe_optimizer_data::modifier_parser::{ParserCallbackId, ParserProgramCaptureOrigin};
use poe_optimizer_data::source_program::{SourceClosurePrototypeId, SourceSessionInput};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::parser_program::runtime) struct ClosureRef(pub u32);
#[derive(Debug, Clone, Copy)]
pub(in crate::parser_program::runtime) struct CellRef(u32);

/// One lexical binding generation. Promoted values live in the session arena,
/// never behind a pointer into a running Rust frame.
#[derive(Clone)]
pub(in crate::parser_program::runtime) enum LocalSlot {
    Value(V),
    Cell(CellRef),
}
pub(super) struct Closure {
    callback: ParserCallbackId,
    captures: Vec<CellRef>,
}
#[derive(Clone, Copy)]
pub(super) struct ImportClosures {
    pub(super) count: usize,
    pub(super) offset: u32,
}
fn offset(existing: usize, additional: usize, name: &'static str) -> Result<u32> {
    let first = u32::try_from(existing).map_err(|_| Error::resource(name))?;
    first
        .checked_add(u32::try_from(additional).map_err(|_| Error::resource(name))?)
        .ok_or_else(|| Error::resource(name))?;
    Ok(first)
}
impl Heap<'_> {
    /// Instantiate validated source code on this same session arena. No pointer
    /// targets a Rust frame: promotion gives the live binding a stable heap cell.
    pub(in crate::parser_program::runtime) fn create_closure(
        &mut self,
        prototype: SourceClosurePrototypeId,
        parent: Option<ClosureRef>,
        locals: &mut [LocalSlot],
        origins: &[ParserProgramCaptureOrigin],
        work: &mut crate::lua_pattern::MatchBudget,
    ) -> Result<V> {
        let definition = self
            .owner()
            .closure_prototype(prototype)
            .ok_or_else(|| Error::input("missing created closure prototype"))?;
        let callback = definition.callback;
        let expected = self
            .owner()
            .callback(callback)
            .ok_or_else(|| Error::input("missing created closure source"))?
            .upvalues
            .len();
        if expected != origins.len() || origins.len() > 128 || locals.len() > 1024 {
            return Err(Error::input("created closure capture layout mismatch"));
        }
        let closure_offset = offset(self.closures.len(), 1, "created closure identity bound")?;
        // Fixed bounded scratch storage does not allocate or retain build state.
        // Zero marks a local needing promotion; existing positive IDs are reused.
        let mut local_cells = [None; 1024];
        let mut promotion_slots = [0u16; 128];
        let mut new_count = 0usize;
        for origin in origins {
            work.charge(1)?;
            match *origin {
                ParserProgramCaptureOrigin::Local { local } => {
                    let slot = locals
                        .get(usize::from(local))
                        .ok_or_else(|| Error::input("created closure local is missing"))?;
                    if local_cells[usize::from(local)].is_none() {
                        local_cells[usize::from(local)] = Some(match slot {
                            LocalSlot::Value(_) => {
                                promotion_slots[new_count] = local;
                                new_count += 1;
                                CellRef(0)
                            }
                            LocalSlot::Cell(cell) => *cell,
                        });
                    }
                }
                ParserProgramCaptureOrigin::ParentCapture { upvalue } => {
                    let parent = parent.ok_or_else(|| {
                        Error::unsupported("inherited capture creation requires a session closure")
                    })?;
                    self.capture_cell(parent, upvalue)?;
                }
            }
        }
        let cell_offset = offset(
            self.cells.len(),
            new_count,
            "created capture cell identity bound",
        )?;
        self.budget.values(1 + origins.len() + new_count)?;
        self.budget.bytes(
            std::mem::size_of::<Closure>()
                + origins.len() * std::mem::size_of::<CellRef>()
                + new_count * std::mem::size_of::<V>(),
        )?;
        // Reserve every arena/instance allocation before publishing identities or
        // changing any frame binding. Capacity changes alone have no Lua identity.
        self.closures
            .try_reserve_exact(1)
            .map_err(|_| Error::resource("created closure arena allocation"))?;
        self.cells
            .try_reserve_exact(new_count)
            .map_err(|_| Error::resource("created capture cell allocation"))?;
        let mut captures = Vec::new();
        captures
            .try_reserve_exact(origins.len())
            .map_err(|_| Error::resource("created capture layout allocation"))?;
        for (index, local) in promotion_slots.iter().take(new_count).enumerate() {
            work.charge(1)?;
            local_cells[usize::from(*local)] = Some(CellRef(cell_offset + index as u32 + 1));
        }
        for origin in origins {
            work.charge(1)?;
            captures.push(match *origin {
                ParserProgramCaptureOrigin::Local { local } => {
                    local_cells[usize::from(local)].expect("validated local capture")
                }
                ParserProgramCaptureOrigin::ParentCapture { upvalue } => {
                    self.capture_cell(parent.expect("validated live parent"), upvalue)?
                }
            });
        }
        // No fallible work after this point. Publish cells in their staged
        // first-capture order without scanning unrelated lexical slots.
        for local in promotion_slots.into_iter().take(new_count) {
            let cell = local_cells[usize::from(local)].expect("staged promotion");
            let slot = &mut locals[usize::from(local)];
            let LocalSlot::Value(value) = slot else {
                unreachable!("new promotion")
            };
            self.cells.push(value.clone());
            *slot = LocalSlot::Cell(cell);
        }
        self.closures.push(Closure { callback, captures });
        Ok(V::Closure(ClosureRef(closure_offset + 1)))
    }
    pub(in crate::parser_program::runtime) fn read_local(&self, local: &LocalSlot) -> Result<V> {
        match local {
            LocalSlot::Value(value) => Ok(value.clone()),
            LocalSlot::Cell(cell) => self
                .cells
                .get(index(cell.0)?)
                .cloned()
                .ok_or_else(|| Error::input("missing promoted local cell")),
        }
    }
    pub(in crate::parser_program::runtime) fn write_local(
        &mut self,
        local: &mut LocalSlot,
        value: V,
    ) -> Result<()> {
        match local {
            LocalSlot::Value(target) => *target = value,
            LocalSlot::Cell(cell) => {
                *self
                    .cells
                    .get_mut(index(cell.0)?)
                    .ok_or_else(|| Error::input("missing promoted local cell"))? = value
            }
        }
        Ok(())
    }
    pub(in crate::parser_program::runtime) fn import_session_input(
        &mut self,
        input: &SourceSessionInput,
    ) -> Result<Vec<V>> {
        if !self.owner().is_same_owner(&input.owner) {
            return Err(Error::input("session input belongs to another owner"));
        }
        // Bound the handle preflight itself before walking an attacker-sized
        // instance list; this does not publish state or charge foreign handles.
        let headers = input
            .closures
            .len()
            .checked_add(input.cells.len())
            .and_then(|value| value.checked_add(input.class_bindings.len()))
            .ok_or_else(|| Error::resource("session closure graph size"))?;
        if headers
            > self
                .budget
                .limits
                .max_values
                .saturating_sub(self.stats().values)
        {
            return Err(Error::resource("session closure graph size"));
        }
        // Resolve every bound prototype before touching foreign graph data.
        for closure in &input.closures {
            self.owner()
                .resolve_closure_prototype(&closure.prototype)
                .map_err(|error| Error::input(error.to_string()))?;
        }
        let closure_offset = offset(
            self.closures.len(),
            input.closures.len(),
            "closure identity bound",
        )?;
        let cell_offset = offset(
            self.cells.len(),
            input.cells.len(),
            "capture cell identity bound",
        )?;
        let table_offset = offset(
            self.tables.len(),
            input.state.tables.len(),
            "session table identity bound",
        )?;
        // Stage all class associations before the graph is published. Coverage
        // markers are accepted only in this coherent path, with exact one-to-one
        // bindings; no raw methods, aliases or parent proxies are manufactured.
        self.budget.values(
            input
                .class_bindings
                .len()
                .checked_mul(2)
                .ok_or_else(|| Error::resource("session class association metadata"))?,
        )?;
        self.budget.values(input.coverage.len())?;
        input
            .validate_class_bindings(
                self.budget
                    .limits
                    .max_tables
                    .saturating_sub(self.stats().tables),
            )
            .map_err(|error| match error.kind {
                poe_optimizer_data::source_program::SourceProgramErrorKind::ResourceLimit => {
                    Error::resource(error.to_string())
                }
                _ => Error::input(error.to_string()),
            })?;
        let mut checked = BTreeSet::new();
        let mut behaviors = BTreeMap::new();
        for (table, class) in &input.class_bindings {
            if !checked.contains(&class.id()) {
                self.budget.values(1)?;
                checked.insert(class.id());
                self.validate_instance_protocol(class.id())?;
            }
            let call_fallback = input
                .coverage
                .get(table)
                .expect("validated class coverage")
                .call_fallback;
            if call_fallback != self.class_call_fallback(class.id())? {
                return Err(Error::input(
                    "class call fallback conflicts with captured metatable",
                ));
            }
            behaviors.insert(
                TableRef::Heap(table_offset + table.0),
                TableBehavior::Instance {
                    class: class.id(),
                    call_fallback,
                },
            );
        }
        let space = Some(ImportClosures {
            count: input.closures.len(),
            offset: closure_offset,
        });
        self.budget.values(input.closures.len())?;
        self.budget.values(input.cells.len())?;
        let mut closures = Vec::with_capacity(input.closures.len());
        for closure in &input.closures {
            self.budget.values(closure.captures.len())?;
            if closure.captures.len() != closure.prototype.capture_count() {
                return Err(Error::input(
                    "closure capture layout differs from source prototype",
                ));
            }
            let mut captures = Vec::with_capacity(closure.captures.len());
            for cell in &closure.captures {
                if index(cell.0)? >= input.cells.len() {
                    return Err(Error::input("missing session capture cell"));
                }
                captures.push(CellRef(cell_offset + cell.0));
            }
            closures.push(Closure {
                callback: closure.prototype.definition().callback,
                captures,
            });
        }
        for value in &input.cells {
            validate_input(
                value,
                input.state.tables.len(),
                &self.catalog,
                &mut self.budget,
                space,
            )?;
        }
        // These vectors remain unpublished until the whole state graph has also
        // passed coverage/key/reference checks. References may form cycles.
        let cells: Vec<_> = input
            .cells
            .iter()
            .map(|value| input_value(value, true, table_offset, space))
            .collect();
        let roots = self.import_graph(&input.state, &input.coverage, true, space, Some(input))?;
        append_staged(&mut self.cells, cells);
        append_staged(&mut self.closures, closures);
        extend_staged_map(&mut self.behaviors, behaviors);
        Ok(roots)
    }
    pub(in crate::parser_program::runtime) fn closure_callback(
        &self,
        closure: ClosureRef,
    ) -> Result<ParserCallbackId> {
        Ok(self
            .closures
            .get(index(closure.0)?)
            .ok_or_else(|| Error::input("missing session closure"))?
            .callback)
    }
    fn capture_cell(&self, closure: ClosureRef, upvalue: u16) -> Result<CellRef> {
        self.closures
            .get(index(closure.0)?)
            .and_then(|closure| closure.captures.get(upvalue as usize))
            .copied()
            .ok_or_else(|| Error::input("missing live closure capture slot"))
    }
    pub(in crate::parser_program::runtime) fn closure_capture(
        &mut self,
        closure: ClosureRef,
        upvalue: u16,
    ) -> Result<V> {
        let cell = self.capture_cell(closure, upvalue)?;
        self.budget.values(1)?;
        self.cells
            .get(index(cell.0)?)
            .cloned()
            .ok_or_else(|| Error::input("missing live capture cell"))
    }
    pub(in crate::parser_program::runtime) fn set_closure_capture(
        &mut self,
        closure: ClosureRef,
        upvalue: u16,
        value: V,
    ) -> Result<()> {
        let cell = self.capture_cell(closure, upvalue)?;
        let target = self
            .cells
            .get_mut(index(cell.0)?)
            .ok_or_else(|| Error::input("missing live capture cell"))?;
        *target = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lua_pattern::{MatchBudget, MatchLimits};
    use crate::parser_program::runtime::{ProgramLimits, ProgramRuntimeErrorKind};
    use poe_optimizer_data::item_loading::{ItemLoadingSource, ItemSourceSpan};
    use poe_optimizer_data::source_program::*;
    fn owner() -> SourceProgramOwner {
        let path = "src/Factory.lua".to_owned();
        SourceProgramOwner::new_with_closures(
            SourceProgramDefinitions {
                schema_version: SOURCE_PROGRAM_DEFINITIONS_SCHEMA_VERSION,
                source: ItemLoadingSource {
                    upstream_revision: "a".repeat(40),
                    files: BTreeMap::from([(path.clone(), "b".repeat(64))]),
                    construction_spans: BTreeMap::new(),
                    module_order: vec![path.clone()],
                },
                tables: vec![],
                roots: vec![],
                intrinsics: BTreeMap::new(),
                callbacks: vec![SourceCallback {
                    kind: SourceCallbackKind::Lua {
                        source: ItemSourceSpan {
                            path,
                            line: 1,
                            end_line: 2,
                            sha256: "b".repeat(64),
                        },
                    },
                    environment: SourceEnvironment::OriginalGlobals,
                    upvalues: (0..2)
                        .map(|i| SourceUpvalue {
                            name: format!("c{i}"),
                            value: SourceValue::LiveCapture {},
                        })
                        .collect(),
                }],
            },
            None,
            None,
            SourceClosurePrototypes {
                schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
                prototypes: vec![SourceClosurePrototype {
                    callback: SourceCallbackId(1),
                }],
            },
        )
        .unwrap()
    }
    fn locals() -> Vec<LocalSlot> {
        vec![LocalSlot::Value(V::Number(7.0))]
    }
    fn origins() -> [ParserProgramCaptureOrigin; 2] {
        [ParserProgramCaptureOrigin::Local { local: 0 }; 2]
    }
    #[test]
    fn failed_creation_keeps_work_charges_without_publishing_cells_or_identity() {
        let owner = owner();
        for bound in 0..5 {
            let mut heap = Heap::owned(&owner, ProgramLimits::default());
            let mut locals = locals();
            let mut work = MatchBudget::new(MatchLimits {
                max_steps: bound,
                ..MatchLimits::default()
            });
            let error = heap
                .create_closure(
                    SourceClosurePrototypeId(1),
                    None,
                    &mut locals,
                    &origins(),
                    &mut work,
                )
                .unwrap_err();
            assert_eq!(error.kind, ProgramRuntimeErrorKind::ResourceBound);
            assert!(heap.cells.is_empty() && heap.closures.is_empty());
            assert!(matches!(locals[0], LocalSlot::Value(V::Number(7.0))));
            assert_eq!(work.steps_used(), bound + 1);
            let charged = heap.stats();
            if bound >= 2 {
                assert_eq!(charged.values, 4);
                assert!(charged.bytes > 0);
            }
            // A second failure retains the same exhausted shared work counter.
            assert!(
                heap.create_closure(
                    SourceClosurePrototypeId(1),
                    None,
                    &mut locals,
                    &origins(),
                    &mut work
                )
                .is_err()
            );
            assert_eq!(heap.stats().values, charged.values);
            assert_eq!(heap.stats().bytes, charged.bytes);
            let mut available = MatchBudget::new(MatchLimits::default());
            let value = heap
                .create_closure(
                    SourceClosurePrototypeId(1),
                    None,
                    &mut locals,
                    &origins(),
                    &mut available,
                )
                .unwrap();
            assert!(matches!(value, V::Closure(ClosureRef(1))));
            assert_eq!(heap.cells.len(), 1);
            assert_eq!(
                heap.closures[0].captures[0].0,
                heap.closures[0].captures[1].0
            );
        }
    }
    #[test]
    fn failed_allocation_admission_never_promotes_a_local_or_publishes_a_closure() {
        let owner = owner();
        for limits in [
            ProgramLimits {
                max_values: 3,
                ..ProgramLimits::default()
            },
            ProgramLimits {
                max_bytes: 0,
                ..ProgramLimits::default()
            },
        ] {
            let mut heap = Heap::owned(&owner, limits);
            let mut locals = locals();
            let mut work = MatchBudget::new(MatchLimits::default());
            let err = heap
                .create_closure(
                    SourceClosurePrototypeId(1),
                    None,
                    &mut locals,
                    &origins(),
                    &mut work,
                )
                .unwrap_err();
            assert_eq!(err.kind, ProgramRuntimeErrorKind::ResourceBound);
            assert!(heap.cells.is_empty() && heap.closures.is_empty());
            assert!(matches!(locals[0], LocalSlot::Value(V::Number(7.0))));
            assert_eq!(work.steps_used(), 2);
            if limits.max_bytes == 0 {
                assert_eq!(heap.stats().values, 4);
            }
        }
    }
    #[test]
    fn duplicate_and_sibling_captures_reuse_cells_while_new_binding_gets_a_new_cell() {
        let owner = owner();
        let mut heap = Heap::owned(&owner, ProgramLimits::default());
        let mut locals = locals();
        let mut work = MatchBudget::new(MatchLimits::default());
        let V::Closure(first) = heap
            .create_closure(
                SourceClosurePrototypeId(1),
                None,
                &mut locals,
                &origins(),
                &mut work,
            )
            .unwrap()
        else {
            panic!()
        };
        let V::Closure(second) = heap
            .create_closure(
                SourceClosurePrototypeId(1),
                None,
                &mut locals,
                &origins(),
                &mut work,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_ne!(first, second);
        assert_eq!(heap.cells.len(), 1);
        heap.write_local(&mut locals[0], V::Number(8.0)).unwrap();
        assert!(matches!(
            heap.closure_capture(first, 1).unwrap(),
            V::Number(8.0)
        ));
        heap.set_closure_capture(second, 0, V::Number(9.0)).unwrap();
        assert!(matches!(
            heap.read_local(&locals[0]).unwrap(),
            V::Number(9.0)
        ));
        locals[0] = LocalSlot::Value(V::Number(20.0));
        heap.create_closure(
            SourceClosurePrototypeId(1),
            None,
            &mut locals,
            &origins(),
            &mut work,
        )
        .unwrap();
        assert_eq!(heap.cells.len(), 2);
        assert!(matches!(
            heap.closure_capture(first, 0).unwrap(),
            V::Number(9.0)
        ));
    }
}
