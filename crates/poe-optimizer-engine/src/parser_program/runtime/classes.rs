//! Source-bound Common class allocation and lookup on the existing identity heap.
//! This is not a second evaluator: method/constructor bodies run in `Run`.
use super::value::{Heap, TableBehavior, TableRef, V};
use super::{ProgramRuntimeError as Error, RuntimeResult as Result};
use crate::lua_pattern::MatchBudget;
use poe_optimizer_data::source_program::SourceClassId;

const UNMODELED_METAMETHODS: &[&str] = &[
    "__eq",
    "__lt",
    "__le",
    "__len",
    "__add",
    "__sub",
    "__mul",
    "__div",
    "__mod",
    "__pow",
    "__unm",
    "__concat",
    "__newindex",
    "__pairs",
    "__ipairs",
    "__tostring",
    "__gc",
    "__mode",
    "__metatable",
];
pub(super) fn unsupported_metamethod(key: &[u8]) -> bool {
    // Parent proxies explicitly model their __newindex table forwarding.
    key != b"__newindex"
        && UNMODELED_METAMETHODS
            .iter()
            .any(|name| name.as_bytes() == key)
}

impl Heap<'_> {
    pub(super) fn class_super_parents(&mut self, id: SourceClassId) -> Result<Vec<SourceClassId>> {
        let owner = self.owner().clone();
        let class = owner
            .class(id)
            .ok_or_else(|| Error::input("unknown source class"))?;
        let parents = class.super_parents.as_deref().unwrap_or_default();
        // The observed pairs order is part of the injected class projection.
        // Reserve clone/iteration storage before publishing any allocated state.
        self.charge_values(
            parents
                .len()
                .checked_mul(3)
                .ok_or_else(|| Error::resource("class traversal allocation"))?,
        )?;
        Ok(parents.to_vec())
    }
    /// Admit an observed instance's lookup protocol independently of allocation
    /// caches and constructor execution. No instance fields are synthesized here.
    pub(super) fn validate_instance_protocol(&self, id: SourceClassId) -> Result<()> {
        let owner = self.owner().clone();
        let class = owner
            .class(id)
            .ok_or_else(|| Error::input("unknown source class"))?;
        let table = owner.table(class.table).expect("validated class table");
        if table.fields.get("__index")
            != Some(&poe_optimizer_data::source_program::SourceValue::Table(
                class.table,
            ))
        {
            return Err(Error::unsupported(
                "class instance prototype is not a captured self index",
            ));
        }
        for name in UNMODELED_METAMETHODS {
            if class.unsupported_fields.contains(*name)
                || table.fields.get(*name).is_some_and(|value| {
                    !matches!(value, poe_optimizer_data::source_program::SourceValue::Nil)
                })
            {
                return Err(Error::unsupported(format!(
                    "unrepresented class metamethod {name}"
                )));
            }
        }
        Ok(())
    }
    pub(super) fn class_call_fallback(
        &self,
        id: SourceClassId,
    ) -> Result<poe_optimizer_data::source_program::SourceTableCallFallback> {
        use poe_optimizer_data::source_program::{SourceTableCallFallback, SourceValue};
        let class = self
            .owner()
            .class(id)
            .ok_or_else(|| Error::input("unknown source class"))?;
        let table = self
            .owner()
            .table(class.table)
            .expect("validated class table");
        Ok(
            if class.unsupported_fields.contains("__call")
                || table
                    .fields
                    .get("__call")
                    .is_some_and(|value| !matches!(value, SourceValue::Nil))
            {
                SourceTableCallFallback::Unavailable
            } else {
                SourceTableCallFallback::NonCallable
            },
        )
    }
    pub(super) fn allocate_instance(
        &mut self,
        id: SourceClassId,
        work: &mut MatchBudget,
    ) -> Result<V> {
        let owner = self.owner().clone();
        let classes = owner
            .classes()
            .ok_or_else(|| Error::input("owner has no class definitions"))?;
        let class = owner
            .class(id)
            .ok_or_else(|| Error::input("unknown source class"))?;
        self.validate_instance_protocol(id)?;
        let table = owner.table(class.table).expect("validated class table");
        let policy = &classes.source;
        if class.constructor.is_some() {
            for name in [
                &policy.unconstructed_meta_field,
                &policy.constructor_initialized_field,
            ] {
                if class.unsupported_fields.contains(name) {
                    return Err(Error::unsupported("unrepresented constructor metadata"));
                }
            }
            if table.fields.get(&policy.unconstructed_meta_field)
                != Some(&poe_optimizer_data::source_program::SourceValue::Table(
                    class.table,
                ))
            {
                return Err(Error::unsupported(
                    "uncached or alternate unconstructed class metatable",
                ));
            }
            if matches!(
                table.fields.get(&policy.constructor_initialized_field),
                None | Some(
                    poe_optimizer_data::source_program::SourceValue::Nil
                        | poe_optimizer_data::source_program::SourceValue::Boolean(false)
                )
            ) {
                return Err(Error::unsupported(
                    "source constructor requires another wrapper initialization",
                ));
            }
        }

        if class
            .constructor
            .as_ref()
            .is_some_and(|constructor| constructor.wrapper.is_none())
        {
            return Err(Error::unsupported(
                "new class constructor wrapper has not been captured",
            ));
        }
        let inherited = self.class_super_parents(id)?;
        let object = self.new_table()?;
        self.set_behavior(
            &object,
            TableBehavior::Instance {
                class: id,
                call_fallback: self.class_call_fallback(id)?,
            },
        )?;
        self.raw_field_set(&object, &policy.object_alias, object.clone(), work)?;
        if table.fields.contains_key(&policy.parent_classes_field) {
            let initialized = self.new_table()?;
            self.raw_field_set(&object, &policy.parent_init, initialized, work)?;
            for parent_id in inherited {
                let parent = owner.class(parent_id).expect("validated parent");
                let proxy = self.new_table()?;
                self.set_behavior(&proxy, TableBehavior::ParentProxy)?;
                self.raw_field_set(
                    &proxy,
                    &policy.proxy_parent,
                    self.definition(parent.table)?,
                    work,
                )?;
                self.raw_field_set(&proxy, &policy.proxy_object, object.clone(), work)?;
                let name = self.bytes(class.name.as_bytes())?;
                self.raw_field_set(&proxy, &policy.proxy_class_name, name, work)?;
                self.raw_field_set(
                    &proxy,
                    "__index",
                    V::Callback(policy.parent_index_callback),
                    work,
                )?;
                self.raw_field_set(&proxy, "__newindex", object.clone(), work)?;
                self.raw_field_set(
                    &proxy,
                    "__call",
                    V::Callback(policy.parent_call_callback),
                    work,
                )?;
                self.raw_field_set(&object, &parent.name, proxy, work)?;
            }
        }
        Ok(object)
    }
    pub(super) fn raw_field_set(
        &mut self,
        table: &V,
        name: &str,
        value: V,
        work: &mut MatchBudget,
    ) -> Result<()> {
        let key = self.bytes(name.as_bytes())?;
        self.raw_set(table, key, value, work)
    }
    pub(super) fn raw_field(&mut self, table: &V, name: &str) -> Result<V> {
        let key = self.bytes(name.as_bytes())?;
        self.raw_get(table, &key)
    }
    pub(super) fn get(&mut self, table: &V, key: &V) -> Result<V> {
        self.get_depth(table, key, 0)
    }
    fn get_depth(&mut self, table: &V, key: &V, depth: usize) -> Result<V> {
        if depth >= 64 {
            return Err(Error::resource("class lookup nesting"));
        }
        let value = self.raw_get(table, key)?;
        if !matches!(value, V::Nil) {
            return Ok(value);
        }
        match self.behavior(table) {
            Some(TableBehavior::Instance { class: id, .. }) => {
                let id = self.owner().class(id).expect("retained class").table;
                let class = self.definition(id)?;
                self.get_depth(&class, key, depth + 1)
            }
            Some(TableBehavior::ParentProxy) => {
                let index = self.raw_field(table, "__index")?;
                let owner = self.owner().clone();
                let policy = &owner.classes().expect("proxy owner").source;
                match index {
                    V::Callback(callback) if callback == policy.parent_index_callback => {
                        self.parent_index(table, key, depth + 1)
                    }
                    V::Table(_) => self.get_depth(&index, key, depth + 1),
                    V::Nil => Ok(V::Nil),
                    _ => Err(Error::unsupported("modified proxy index metamethod")),
                }
            }
            None => {
                self.ensure_index_fallback(table)?;
                if let Some(id) = self.class_for_table(table) {
                    let class = self.owner().class(id).expect("known definition class");
                    if key
                        .as_bytes()
                        .and_then(|key| std::str::from_utf8(key).ok())
                        .is_some_and(|key| class.unsupported_fields.contains(key))
                    {
                        return Err(Error::unsupported("unrepresented source class field"));
                    }
                }
                Ok(V::Nil)
            }
        }
    }
    pub(super) fn parent_index(&mut self, proxy: &V, key: &V, depth: usize) -> Result<V> {
        let owner = self.owner().clone();
        let policy = &owner
            .classes()
            .ok_or_else(|| Error::input("missing proxy policy"))?
            .source;
        let object_key = self.bytes(policy.proxy_object.as_bytes())?;
        let object = self.get_depth(proxy, &object_key, depth + 1)?;
        let value = self.raw_get(&object, key)?;
        if !matches!(value, V::Nil) {
            return Ok(value);
        }
        let parent_key = self.bytes(policy.proxy_parent.as_bytes())?;
        let parent = self.get_depth(proxy, &parent_key, depth + 1)?;
        self.get_depth(&parent, key, depth + 1)
    }
    pub(super) fn set(
        &mut self,
        table: &V,
        key: V,
        value: V,
        work: &mut MatchBudget,
    ) -> Result<()> {
        self.set_depth(table, key, value, 0, work)
    }
    fn set_depth(
        &mut self,
        table: &V,
        key: V,
        value: V,
        depth: usize,
        work: &mut MatchBudget,
    ) -> Result<()> {
        if depth >= 64 {
            return Err(Error::resource("class assignment nesting"));
        }
        if matches!(self.behavior(table), Some(TableBehavior::ParentProxy))
            && matches!(self.raw_get(table, &key)?, V::Nil)
        {
            let target = self.raw_field(table, "__newindex")?;
            match target {
                V::Table(_) => return self.set_depth(&target, key, value, depth + 1, work),
                V::Nil => (),
                _ => return Err(Error::unsupported("modified proxy newindex metamethod")),
            }
        }
        self.raw_set(table, key, value, work)
    }
    pub(super) fn class_for_table(&self, value: &V) -> Option<SourceClassId> {
        let V::Table(TableRef::Definition(table)) = value else {
            return None;
        };
        self.owner()
            .classes()?
            .classes
            .iter()
            .position(|class| class.table == *table)
            .map(|index| SourceClassId(index as u32 + 1))
    }
}
