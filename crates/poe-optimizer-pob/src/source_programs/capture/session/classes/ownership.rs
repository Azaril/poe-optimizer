//! Positive definition ownership follows represented table edges, never function captures.
use super::*;
pub(in crate::source_programs::capture::session) fn discover(
    graph: &mut Graph<'_>,
    request: &SourceSessionCaptureRequest,
    class_request: &SourceClassCaptureRequest,
    classes: &PreparedClasses,
) -> Result<BTreeSet<usize>> {
    struct Discovery<'a, 'b> {
        graph: &'b mut Graph<'a>,
        projections: BTreeMap<usize, &'b SourceTableSelection>,
        classes: &'b PreparedClasses,
        tables: BTreeSet<usize>,
    }
    impl Discovery<'_, '_> {
        fn table(&mut self, table: Table, depth: usize) -> Result<()> {
            let pointer = table.to_pointer() as usize;
            if self.tables.contains(&pointer) {
                return Ok(());
            }
            if depth > 64 || self.tables.len() >= MAX_TABLES {
                return Err(error("immutable ownership table graph bound"));
            }
            self.graph.projection_row()?;
            self.tables.insert(pointer);
            let projection = self.projections.get(&pointer).copied();
            if projection.is_none() && !self.classes.ids.contains_key(&pointer) {
                plain(&table, "immutable ownership table")?;
            }
            let mut children = Vec::new();
            for (index, entry) in table.pairs::<Value, Value>().enumerate() {
                self.graph.projection_row()?;
                if index >= 50_000 {
                    return Err(error("immutable ownership table row bound"));
                }
                let (key, value) = entry?;
                let key = self.graph.projection_key(key)?;
                let selected = self
                    .classes
                    .represented_value(pointer, &key, &value)
                    .unwrap_or_else(|| {
                        projection.is_none_or(|projection| match &key {
                            SourceTableKey::Text(key) => projection.fields.contains(key),
                            SourceTableKey::Integer(key) => projection.indexed.contains(key),
                        })
                    });
                // A method's hidden state cannot declare itself immutable. Only
                // represented table-to-table edges extend explicit ownership.
                if selected && let Value::Table(table) = value {
                    children.push(table);
                }
            }
            for child in children {
                self.table(child, depth + 1)?;
            }
            Ok(())
        }
    }
    let mut discovery = Discovery {
        graph,
        projections: request
            .definitions
            .projections
            .iter()
            .map(|selection| (selection.table.to_pointer() as usize, selection))
            .collect(),
        classes,
        tables: BTreeSet::new(),
    };
    for selection in &request.definitions.projections {
        discovery.table(selection.table.clone(), 0)?;
    }
    for table in request.definition_roots.values() {
        discovery.table(table.clone(), 0)?;
    }
    if let Some(environment) = &request.definitions.environment {
        discovery.table(environment.table.clone(), 0)?;
    }
    for selection in &class_request.classes {
        discovery.table(selection.table.clone(), 0)?;
    }
    Ok(discovery.tables)
}
