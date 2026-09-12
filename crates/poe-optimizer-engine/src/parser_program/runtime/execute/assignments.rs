//! LuaJIT lvalue preparation, conflict preservation and reverse stores.
use super::{Error, Frame, Result, Run, V};
use crate::parser_program::CompiledAssignmentTarget;
use poe_optimizer_data::modifier_parser::{
    ParserProgramAssignmentOperand as Operand, ParserProgramAssignmentTargetKind as TargetKind,
    ParserProgramExpr, ParserProgramLocation, ParserProgramValueList,
};

// Keep live lexical slots distinct from values evaluated into temporary source
// registers. Future lexical-cell promotion must preserve this distinction.
enum PreparedOperand {
    LocalRegister(u16),
    Value(V),
}
impl PreparedOperand {
    fn resolve(&self, frame: &Frame) -> V {
        match self {
            Self::LocalRegister(local) => frame.locals[usize::from(*local)].clone(),
            Self::Value(value) => value.clone(),
        }
    }
    fn preserve(&mut self, local: u16, frame: &Frame) {
        if matches!(self, Self::LocalRegister(slot) if *slot == local) {
            *self = Self::Value(frame.locals[usize::from(local)].clone());
        }
    }
}
enum PreparedTarget {
    Local(u16),
    Capture(u16),
    Indexed {
        table: PreparedOperand,
        key: PreparedOperand,
    },
}
impl PreparedTarget {
    fn preserve(&mut self, local: u16, frame: &Frame) {
        if let Self::Indexed { table, key } = self {
            table.preserve(local, frame);
            key.preserve(local, frame);
        }
    }
}
impl Run<'_, '_, '_> {
    pub(super) fn indexed_read(
        &mut self,
        frame: &mut Frame,
        table: &Operand,
        key: &ParserProgramExpr,
        depth: usize,
    ) -> Result<V> {
        let table = self.assignment_operand(frame, table, depth + 1)?;
        let key = self.expr(frame, key, depth + 1)?;
        // A direct local base stays in its source register during key effects.
        // Computed bases and captured upvalues were already evaluated above.
        self.heap.get(&table.resolve(frame), &key)
    }
    pub(super) fn mixed_assign(
        &mut self,
        frame: &mut Frame,
        targets: &[CompiledAssignmentTarget],
        values: &ParserProgramValueList,
        depth: usize,
    ) -> Result<()> {
        let callback = self.library.0.programs[frame.program].callback;
        // Reserve bounded temporary target/operand storage before evaluating any
        // source operand. Both failed and successful invocations keep the charge.
        self.heap.charge_values(targets.len().saturating_mul(3))?;
        self.heap.charge_bytes(
            targets
                .len()
                .saturating_mul(std::mem::size_of::<PreparedTarget>()),
        )?;
        let mut prepared = Vec::new();
        prepared
            .try_reserve_exact(targets.len())
            .map_err(|_| Error::resource("assignment target allocation"))?;
        for target in targets {
            let result = (|| {
                self.tick(depth)?;
                match &target.source.operation {
                    TargetKind::Local { local } => {
                        // lj_parse.c assign_hazard copies conflicting previous
                        // address registers when THIS local target is encountered,
                        // after earlier address effects and before later ones.
                        for previous in &target.preserves {
                            self.tick(depth)?;
                            PreparedTarget::preserve(&mut prepared[*previous], *local, frame);
                        }
                        Ok(PreparedTarget::Local(*local))
                    }
                    TargetKind::Capture { upvalue } => Ok(PreparedTarget::Capture(*upvalue)),
                    TargetKind::Indexed { table, key } => {
                        let table = self.assignment_operand(frame, table, depth + 1)?;
                        let key = self.assignment_operand(frame, key, depth + 1)?;
                        // Neither table type nor key validity is checked until the
                        // store. RHS effects precede a TSET failure in the source.
                        Ok(PreparedTarget::Indexed { table, key })
                    }
                }
            })();
            prepared
                .push(result.map_err(|error: Error| error.at(callback, target.source.location))?);
        }
        // Evaluate every RHS expression, including excess values. Adjustment to
        // target count happens only at stores; an empty result pack fills Nil.
        let values = self.values(frame, values, depth + 1)?;
        for (index, target) in prepared.iter().enumerate().rev() {
            let value = values.get(index).cloned().unwrap_or(V::Nil);
            self.assignment_store(frame, target, value, targets[index].source.location, depth)?;
        }
        Ok(())
    }
    fn assignment_operand(
        &mut self,
        frame: &mut Frame,
        operand: &Operand,
        depth: usize,
    ) -> Result<PreparedOperand> {
        self.tick(depth)?;
        match operand {
            Operand::LocalRegister { local } => Ok(PreparedOperand::LocalRegister(*local)),
            Operand::Evaluated { value } => Ok(PreparedOperand::Value(self.expr(
                frame,
                value,
                depth + 1,
            )?)),
        }
    }
    fn assignment_store(
        &mut self,
        frame: &mut Frame,
        target: &PreparedTarget,
        value: V,
        location: ParserProgramLocation,
        depth: usize,
    ) -> Result<()> {
        let callback = self.library.0.programs[frame.program].callback;
        (|| {
            self.tick(depth)?;
            match target {
                PreparedTarget::Local(local) => frame.locals[usize::from(*local)] = value,
                PreparedTarget::Capture(upvalue) => {
                    let closure = frame.closure.ok_or_else(|| {
                        Error::unsupported("capture assignment requires a session closure")
                    })?;
                    self.heap.set_closure_capture(closure, *upvalue, value)?;
                }
                PreparedTarget::Indexed { table, key } => {
                    let table = table.resolve(frame);
                    let key = key.resolve(frame);
                    self.heap.set(&table, key, value, self.patterns)?;
                }
            }
            Ok(())
        })()
        .map_err(|error: Error| error.at(callback, location))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immutable_plan_records_only_first_conflict_for_each_live_operand() {
        use poe_optimizer_data::modifier_parser::ParserProgramAssignmentTarget;
        let indexed = || ParserProgramAssignmentTarget {
            location: ParserProgramLocation { start: 0, end: 1 },
            operation: TargetKind::Indexed {
                table: Operand::LocalRegister { local: 0 },
                key: Operand::LocalRegister { local: 1 },
            },
        };
        let local = |local| ParserProgramAssignmentTarget {
            location: ParserProgramLocation { start: 0, end: 1 },
            operation: TargetKind::Local { local },
        };
        let plan = CompiledAssignmentTarget::compile(&[
            indexed(),
            indexed(),
            local(0),
            indexed(),
            local(0),
            local(1),
            local(1),
        ]);
        assert_eq!(&*plan[2].preserves, &[0, 1]);
        assert_eq!(&*plan[4].preserves, &[3]);
        assert_eq!(&*plan[5].preserves, &[0, 1, 3]);
        assert!(plan[6].preserves.is_empty());
        assert!(
            plan.iter()
                .enumerate()
                .all(|(index, target)| target.preserves.iter().all(|previous| *previous < index))
        );
    }

    #[test]
    fn live_registers_and_hazard_copies_remain_distinct() {
        // This is a structural slot test. Source-created closures that mutate an
        // active frame's locals are not yet admitted by the native factory seam.
        let mut frame = Frame {
            program: 0,
            closure: None,
            locals: vec![V::Number(1.0), V::Number(2.0)],
            extra: Vec::new(),
            loops: Vec::new(),
        };
        let mut target = PreparedTarget::Indexed {
            table: PreparedOperand::LocalRegister(0),
            key: PreparedOperand::LocalRegister(1),
        };
        frame.locals[0] = V::Number(3.0);
        frame.locals[1] = V::Number(4.0);
        target.preserve(0, &frame);
        frame.locals[0] = V::Number(5.0);
        frame.locals[1] = V::Number(6.0);
        target.preserve(0, &frame); // A later duplicate target cannot recopy it.
        let PreparedTarget::Indexed { table, key } = target else {
            unreachable!()
        };
        assert!(table.resolve(&frame).lua_equal(&V::Number(3.0)));
        assert!(key.resolve(&frame).lua_equal(&V::Number(6.0)));
    }
}
