//! Source-owned complete node read projection, including derived charm-socket flags.
use super::*;
use poe_optimizer_data::item_loading::{ItemMetadataTable as M, ItemMetadataValue as V};
struct ReadBudget {
    entries: usize,
    bytes: usize,
}
impl ReadBudget {
    fn text(&mut self, text: &str) -> Result<()> {
        self.bytes = self
            .bytes
            .checked_add(text.len())
            .ok_or_else(|| error("node read bytes overflow"))?;
        if text.len() > 4096 || self.bytes > 256 * 1024 {
            return Err(error("node read text bound"));
        }
        Ok(())
    }
    fn value(&mut self, value: &SourceValue, depth: usize) -> Result<()> {
        if depth > 16 {
            return Err(error("node read depth bound"));
        }
        match value {
            SourceValue::Boolean(_) => Ok(()),
            SourceValue::Integer(v) if v.unsigned_abs() <= 9_007_199_254_740_991 => Ok(()),
            SourceValue::Number(v) if v.is_finite() => Ok(()),
            SourceValue::String(s) => self.text(s),
            SourceValue::Table(t) => {
                self.entries = self
                    .entries
                    .checked_add(t.named.len() + t.indexed.len())
                    .ok_or_else(|| error("node read entry overflow"))?;
                if self.entries > 262_144 {
                    return Err(error("node read entry bound"));
                }
                for (k, v) in &t.named {
                    self.text(k)?;
                    self.value(v, depth + 1)?;
                }
                for v in t.indexed.values() {
                    self.value(v, depth + 1)?;
                }
                Ok(())
            }
            _ => Err(error("node read number must be exactly representable")),
        }
    }
}
fn finite(value: &SourceValue, depth: usize) -> Result<V> {
    if depth > 16 {
        return Err(error("node validity acquisition depth"));
    }
    Ok(match value {
        SourceValue::Boolean(v) => V::Boolean(*v),
        SourceValue::Integer(v) if v.unsigned_abs() <= 9_007_199_254_740_991 => {
            V::Number(*v as f64)
        }
        SourceValue::Number(v) if v.is_finite() => V::Number(*v),
        SourceValue::String(v) => V::Text(v.clone()),
        SourceValue::Table(t) => V::Table(M {
            fields: t
                .named
                .iter()
                .map(|(k, v)| Ok((k.clone(), finite(v, depth + 1)?)))
                .collect::<Result<_>>()?,
            indexed: t
                .indexed
                .iter()
                .map(|(k, v)| Ok((*k, finite(v, depth + 1)?)))
                .collect::<Result<_>>()?,
        }),
        _ => {
            return Err(error(
                "node validity acquisition requires an exactly representable number",
            ));
        }
    })
}
pub(super) fn project(
    lua: &Lua,
    tree: &AuthenticatedTreeSnapshot,
    policy: &ItemSlotValidityPolicy,
    flags: &str,
) -> Result<M> {
    let charm_name = text_after(lua, row(flags, "if socket.name == ")?, " == ")?;
    let assignment = flags
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("socket.") && line.ends_with(" = true"))
        .ok_or_else(|| error("derived charm socket assignment"))?;
    let charm_field = ident(after(assignment, "socket.")?);
    let fields = [
        &policy.jewel.sinister_field,
        &policy.jewel.contained_socket_field,
        &policy.jewel.charm_socket_field,
        &policy.jewel.expansion_field,
    ];
    let is_charm = |node: &poe_optimizer_data::tree_data::TreeNode| {
        let socket = node.kind == TreeNodeKind::Socket
            || (node.kind == TreeNodeKind::Notable
                && source_truth(node.source.named.get("ascendancyName"))
                && source_truth(node.source.named.get(&policy.jewel.contained_socket_field)));
        socket
            && matches!(node.source.named.get("name"), Some(SourceValue::String(name)) if *name==charm_name)
    };
    let mut budget = ReadBudget {
        entries: 0,
        bytes: 0,
    };
    if tree.snapshot().nodes.len() > 65_536 {
        return Err(error("node read map count bound"));
    }
    for node in tree.snapshot().nodes.values() {
        for field in fields {
            let derived = field == charm_field && is_charm(node);
            if derived || node.source.named.contains_key(field) {
                budget.entries = budget
                    .entries
                    .checked_add(1)
                    .ok_or_else(|| error("node read entry overflow"))?;
                if budget.entries > 262_144 {
                    return Err(error("node read entry bound"));
                }
                budget.text(field)?;
                if !derived {
                    budget.value(&node.source.named[field], 0)?;
                }
            }
        }
    }
    let mut out = M::default();
    for (&id, node) in &tree.snapshot().nodes {
        let mut row = M::default();
        for field in fields {
            if field == charm_field && is_charm(node) {
                row.fields.insert(field.clone(), V::Boolean(true));
            } else if let Some(value) = node.source.named.get(field) {
                row.fields.insert(field.clone(), finite(value, 0)?);
            }
        }
        out.indexed.insert(i64::from(id), V::Table(row));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn node_acquisition_preflight_bounds_before_copying_and_preserves_scalar_types() {
        let mut budget = ReadBudget {
            entries: 0,
            bytes: 0,
        };
        budget.value(&SourceValue::Boolean(false), 0).unwrap();
        budget.value(&SourceValue::Integer(0), 0).unwrap();
        assert!(budget.value(&SourceValue::Integer(i64::MAX), 0).is_err());
        assert!(
            budget
                .value(&SourceValue::Number(f64::INFINITY), 0)
                .is_err()
        );
        assert!(
            budget
                .value(&SourceValue::String("x".repeat(4097)), 0)
                .is_err()
        );
        let mut budget = ReadBudget {
            entries: 262_144,
            bytes: 0,
        };
        let value = SourceValue::Table(poe_optimizer_data::tree_data::SourceTable {
            named: [("child".into(), SourceValue::Boolean(true))].into(),
            indexed: Default::default(),
        });
        assert!(budget.value(&value, 0).is_err());
        assert!(
            ReadBudget {
                entries: 0,
                bytes: 0
            }
            .value(&value, 17)
            .is_err()
        );
    }
}
