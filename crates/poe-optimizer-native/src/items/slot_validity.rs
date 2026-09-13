//! Read-only validity queries over this preparation's owned inventory.
use super::PreparedItems;
use poe_optimizer_core::build_identity::ItemRecordId;
use poe_optimizer_import::{
    item_loading::assembly::AssemblyError,
    item_slot_validity::{
        SlotValidityContext, SlotValidityLimits, SlotValidityProgram, SlotValidityRequest,
        SlotValidityResult, Value,
    },
};

type Result<T> = std::result::Result<T, AssemblyError>;

/// Explicit component inputs. Saved XML assignments, tree context and actor flags
/// are not inferred by this API and do not become effective equipment through it.
pub struct PreparedItemSlotRequest<'a> {
    pub item: ItemRecordId,
    pub slot_name: &'a str,
    pub item_set: Value<'a>,
    pub flag_state: Value<'a>,
}

/// Reusable compiled query program bound to one privately prepared inventory.
/// Cloning shares program storage and retains the same inventory owner.
#[derive(Clone)]
pub struct PreparedItemSlotEvaluator<'a> {
    inventory: &'a PreparedItems,
    program: SlotValidityProgram,
}
impl PreparedItems {
    /// Compile bounded patterns once for repeated read-only validity queries.
    /// This does not resolve a set, infer actor flags or activate equipment.
    pub fn slot_evaluator(
        &self,
        limits: SlotValidityLimits,
    ) -> Result<PreparedItemSlotEvaluator<'_>> {
        Ok(PreparedItemSlotEvaluator {
            inventory: self,
            program: SlotValidityProgram::new(
                &self.data.snapshot().item_assembly().policy().slot_validity,
                limits,
            )?,
        })
    }
}
impl PreparedItemSlotEvaluator<'_> {
    pub fn shares_program_with(&self, other: &Self) -> bool {
        self.program.shares_storage_with(&other.program)
    }
    /// Query one already registered item using this inventory's data and lookup
    /// winners. Other lazy dependencies come from the caller's explicit context.
    /// A result does not complete ItemsTab loading, activate a slot or authorize
    /// actor calculations. Consumers joining stages still call validate_binding.
    pub fn check<'a, C: SlotValidityContext<'a>>(
        &'a self,
        request: PreparedItemSlotRequest<'a>,
        context: &mut C,
    ) -> Result<SlotValidityResult<'a>> {
        let item = self.inventory.item(request.item).ok_or_else(|| {
            AssemblyError::unsupported(
                "slot query item is not registered in this prepared inventory",
            )
        })?;
        if !item.is_complete() {
            return Err(AssemblyError::unsupported(
                "slot query requires complete owned item assembly",
            ));
        }
        self.program.check(
            SlotValidityRequest {
                item: Value::item(item),
                slot_name: request.slot_name,
                item_set: request.item_set,
                flag_state: request.flag_state,
            },
            &mut InventoryContext {
                inventory: self.inventory,
                caller: context,
            },
        )
    }
}

struct InventoryContext<'a, 'c, C> {
    inventory: &'a PreparedItems,
    caller: &'c mut C,
}
impl<'a, C: SlotValidityContext<'a>> SlotValidityContext<'a> for InventoryContext<'a, '_, C> {
    fn active_item_set(&mut self) -> Result<Value<'a>> {
        self.caller.active_item_set()
    }
    fn tree_node(&mut self, key: Value<'_>) -> Result<Value<'a>> {
        self.caller.tree_node(key)
    }
    fn effective_node(&mut self, key: Value<'_>) -> Result<Value<'a>> {
        self.caller.effective_node(key)
    }
    fn inventory_item(&mut self, key: Value<'_>) -> Result<Value<'a>> {
        // Source registration inserts numeric IDs. String "1" is not numeric 1;
        // nil, NaN and all other key types miss without coercion or a read error.
        let Value::Number(id) = key else {
            return Ok(Value::Nil);
        };
        Ok(self
            .inventory
            .registered_id(id)
            .and_then(|id| self.inventory.item(id))
            .map(Value::item)
            .unwrap_or(Value::Nil))
    }
    fn has_calculation_environment(&mut self) -> Result<bool> {
        self.caller.has_calculation_environment()
    }
    fn flag(&mut self, query_name: &str) -> Result<Value<'a>> {
        self.caller.flag(query_name)
    }
}
