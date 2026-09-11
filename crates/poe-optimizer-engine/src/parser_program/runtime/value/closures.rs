//! Session-owned closure identities and shared capture cells on the existing heap.
use super::{Error, Heap, Result, V, index, input_value, validate_input};
use poe_optimizer_data::modifier_parser::ParserCallbackId;
use poe_optimizer_data::source_program::SourceSessionInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::parser_program::runtime) struct ClosureRef(pub u32);
#[derive(Debug, Clone, Copy)]
struct CellRef(u32);
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
        let roots = self.import_graph(&input.state, &input.coverage, true, space)?;
        self.cells.extend(cells);
        self.closures.extend(closures);
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
