//! Bounded finite node fields for native equipment validity, separate from allocation.
use super::{Budget, ItemInventoryPassiveNodes, Result, error};
use crate::item_loading::{ItemMetadataTable, ItemMetadataValue};
pub(super) fn validate(b: &mut Budget, nodes: &ItemInventoryPassiveNodes) -> Result<()> {
    if !nodes.validity_nodes.fields.is_empty() {
        return Err(error("node validity map requires numeric keys"));
    }
    b.count(nodes.validity_nodes.indexed.len(), 65_536)?;
    let mut entries = 0usize;
    fn value(
        b: &mut Budget,
        v: &ItemMetadataValue,
        depth: usize,
        entries: &mut usize,
    ) -> Result<()> {
        if depth > 16 {
            return Err(error("node validity field depth"));
        }
        match v {
            ItemMetadataValue::Boolean(_) => Ok(()),
            ItemMetadataValue::Number(n) => b.number(*n),
            ItemMetadataValue::Text(s) => b.text(s),
            ItemMetadataValue::Table(t) => table(b, t, depth + 1, entries),
            _ => Err(error(
                "node validity field must retain finite raw table/scalar form",
            )),
        }
    }
    fn table(
        b: &mut Budget,
        t: &ItemMetadataTable,
        depth: usize,
        entries: &mut usize,
    ) -> Result<()> {
        *entries = entries
            .checked_add(t.fields.len() + t.indexed.len())
            .ok_or_else(|| error("node validity entry overflow"))?;
        b.count(*entries, 262_144)?;
        for (k, v) in &t.fields {
            b.text(k)?;
            value(b, v, depth, entries)?;
        }
        for v in t.indexed.values() {
            value(b, v, depth, entries)?;
        }
        Ok(())
    }
    for (id, v) in &nodes.validity_nodes.indexed {
        u32::try_from(*id).map_err(|_| error("node validity ID is outside u32"))?;
        let ItemMetadataValue::Table(t) = v else {
            return Err(error("node validity entry must be a table"));
        };
        table(b, t, 0, &mut entries)?;
    }
    if nodes
        .ids
        .iter()
        .any(|id| !nodes.validity_nodes.indexed.contains_key(&i64::from(*id)))
    {
        return Err(error(
            "constructor socket is absent from complete node validity map",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn nodes() -> ItemInventoryPassiveNodes {
        ItemInventoryPassiveNodes {
            tree_version: "caller".into(),
            full_snapshot_sha256: "a".repeat(64),
            ids: vec![7],
            validity_nodes: ItemMetadataTable {
                indexed: [(7, ItemMetadataValue::Table(Default::default()))].into(),
                ..Default::default()
            },
        }
    }
    #[test]
    fn full_node_read_map_preserves_false_zero_and_non_socket_nodes() {
        let mut n = nodes();
        n.validity_nodes.indexed.insert(
            8,
            ItemMetadataValue::Table(ItemMetadataTable {
                fields: [
                    ("caller_false".into(), ItemMetadataValue::Boolean(false)),
                    ("caller_zero".into(), ItemMetadataValue::Number(0.0)),
                ]
                .into(),
                ..Default::default()
            }),
        );
        validate(&mut Budget { bytes: 0 }, &n).unwrap();
        let encoded = serde_json::to_vec(&n).unwrap();
        assert_eq!(
            serde_json::from_slice::<ItemInventoryPassiveNodes>(&encoded).unwrap(),
            n
        );
    }
    #[test]
    fn node_read_map_rejects_missing_constructor_nodes_and_nonfinite_values() {
        let mut n = nodes();
        n.validity_nodes.indexed.clear();
        assert!(validate(&mut Budget { bytes: 0 }, &n).is_err());
        let mut n = nodes();
        n.validity_nodes
            .indexed
            .insert(7, ItemMetadataValue::Number(1.0));
        assert!(validate(&mut Budget { bytes: 0 }, &n).is_err());
        let mut n = nodes();
        n.validity_nodes.indexed.insert(
            9,
            ItemMetadataValue::Table(ItemMetadataTable {
                fields: [("bad".into(), ItemMetadataValue::Number(f64::INFINITY))].into(),
                ..Default::default()
            }),
        );
        assert!(validate(&mut Budget { bytes: 0 }, &n).is_err());
    }
}
