//! Source-authenticated inventory definitions, without loading the ItemsTab UI.
//! Transform bodies are described, never invoked. Complete passive socket IDs
//! come from the authenticated full tree, not the partial bundled class tree.
use super::*;
use poe_optimizer_data::{
    tree_data::{SourceValue, TreeNodeKind},
    tree_projection::AuthenticatedTreeSnapshot,
};

pub(super) fn spans() -> BTreeMap<String, ItemSourceSpan> {
    [
        (
            "inventory_declarations",
            "src/Classes/ItemsTab.lua",
            32,
            40,
            "da7de389b4f06731c4375cf668b3d932e0c7f8f5099a177b5201d23c7e8c6c36",
        ),
        (
            "inventory_slot_layout",
            "src/Classes/ItemsTab.lua",
            190,
            310,
            "b703235e2e1e94f1c8715dc5a962b5d8946fefaddd6f3a81b98bd72a1536af7a",
        ),
        (
            "inventory_passive_layout",
            "src/Classes/ItemsTab.lua",
            331,
            345,
            "532afdf7456d76c13b2e5eba3698f08456195f40e02c0181731451cdf9a3d0e4",
        ),
        (
            "inventory_initial_fields",
            "src/Classes/ItemsTab.lua",
            149,
            152,
            "2acd6c89e422fb6946d331ec628a5c90a025d78338db9d753e25a2d001ca125c",
        ),
        (
            "inventory_initial_set",
            "src/Classes/ItemsTab.lua",
            1182,
            1191,
            "c81b0f23bc441fea1ff49ecb607831478852144e97f0c2d4080f4330766985b1",
        ),
        (
            "inventory_slot_control",
            "src/Classes/ItemSlotControl.lua",
            22,
            148,
            "867f6af094cc6b55733baa1c4515602484bd3d49ee044cc7e3e4f452114bec3d",
        ),
        (
            "inventory_load",
            "src/Classes/ItemsTab.lua",
            1193,
            1320,
            "fe07d238f17c557bc010d543156f60f17a95c14d5ce2d788f3b854599f87eb84",
        ),
        (
            "inventory_create_set",
            "src/Classes/ItemsTab.lua",
            1571,
            1589,
            "385bd5c958c8f88ad199c007e7ba9d0c44b0312f28178488b39d701887a983cd",
        ),
        (
            "inventory_power_rows",
            "src/Modules/Data.lua",
            131,
            184,
            "ddbce02489807d66c4d7cbd6671cdd56262105e859832687908b8e4b7f4c5430",
        ),
        (
            "inventory_power_minions",
            "src/Modules/Data.lua",
            218,
            238,
            "fdd55d2d59793862642b876868ed6090014d8f3241c429cbb25ff8ed2907ed03",
        ),
        (
            "inventory_tree_types",
            "src/Classes/PassiveTree.lua",
            212,
            280,
            "0befb87a34768909fddce687417f154294cb068512d3bb791f2e2fe562a488f6",
        ),
    ]
    .into_iter()
    .map(|(role, path, line, end_line, sha256)| {
        (
            role.into(),
            ItemSourceSpan {
                path: path.into(),
                line,
                end_line,
                sha256: sha256.into(),
            },
        )
    })
    .collect()
}

fn rows<'a>(body: &'a str, prefix: &str) -> Result<Vec<&'a str>> {
    let rows = body
        .lines()
        .map(str::trim)
        .filter(|row| row.starts_with(prefix))
        .collect::<Vec<_>>();
    if rows.is_empty() || rows.len() > 1024 {
        return Err(error(format!("inventory source row bound: {prefix}")));
    }
    Ok(rows)
}
fn row<'a>(body: &'a str, prefix: &str) -> Result<&'a str> {
    let found = rows(body, prefix)?;
    if found.len() != 1 {
        return Err(error(format!("ambiguous inventory source row: {prefix}")));
    }
    Ok(found[0])
}
fn strings(lua: &Lua, text: &str) -> Result<Vec<String>> {
    let mut rest = text;
    let mut result = Vec::new();
    while let Some(at) = rest.find(['"', '\'']) {
        if result.len() >= 128 {
            return Err(error("inventory source literal bound"));
        }
        let quoted = &rest[at..];
        result.push(string(lua, quoted)?);
        rest = &quoted[quoted_end(quoted)?..];
    }
    Ok(result)
}
fn fixed_strings<const N: usize>(lua: &Lua, text: &str) -> Result<[String; N]> {
    strings(lua, text)?
        .try_into()
        .map_err(|_| error("inventory source literal arity"))
}
fn literal_bool(text: &str) -> Result<bool> {
    match text.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(error("inventory source Boolean literal")),
    }
}
fn body<'a>(bodies: &'a BTreeMap<String, &str>, role: &str) -> Result<&'a str> {
    bodies
        .get(role)
        .copied()
        .ok_or_else(|| error(format!("missing inventory source role {role}")))
}
fn authenticated(bodies: &BTreeMap<String, &str>) -> Result<()> {
    for (role, span) in spans() {
        if hash(body(bodies, &role)?.as_bytes()) != span.sha256 {
            return Err(error(format!("changed complete inventory source {role}")));
        }
    }
    Ok(())
}

fn source_truth(value: Option<&SourceValue>) -> bool {
    !matches!(value, None | Some(SourceValue::Boolean(false)))
}
fn raw_kind(fields: &BTreeMap<String, SourceValue>) -> TreeNodeKind {
    let truth = |key| source_truth(fields.get(key));
    if truth("classesStart") {
        TreeNodeKind::ClassStart
    } else if truth("isAscendancyStart") {
        TreeNodeKind::AscendancyStart
    } else if truth("isOnlyImage") {
        TreeNodeKind::ImageOnly
    } else if truth("isJewelSocket") {
        TreeNodeKind::Socket
    } else if truth("ks") || truth("isKeystone") {
        TreeNodeKind::Keystone
    } else if truth("not") || truth("isNotable") {
        TreeNodeKind::Notable
    } else {
        TreeNodeKind::Normal
    }
}
fn kind_index(kind: TreeNodeKind) -> usize {
    match kind {
        TreeNodeKind::ClassStart => 0,
        TreeNodeKind::AscendancyStart => 1,
        TreeNodeKind::ImageOnly => 2,
        TreeNodeKind::Socket => 3,
        TreeNodeKind::Keystone => 4,
        TreeNodeKind::Notable => 5,
        TreeNodeKind::Normal => 6,
    }
}
fn source_id(value: Option<&SourceValue>) -> Result<u32> {
    match value {
        Some(SourceValue::Integer(value)) => u32::try_from(*value).map_err(error),
        Some(SourceValue::Number(value))
            if value.is_finite()
                && value.fract() == 0.0
                && (0.0..=f64::from(u32::MAX)).contains(value) =>
        {
            Ok(*value as u32)
        }
        _ => Err(error("inventory full-tree source node skill is not a u32")),
    }
}
fn passive_nodes(
    lua: &Lua,
    types: &str,
    tree: &AuthenticatedTreeSnapshot,
    node_type: &str,
    contained_field: &str,
) -> Result<ItemInventoryPassiveNodes> {
    let type_rows = rows(types, "node.type = ")?;
    if type_rows.len() != 7 {
        return Err(error("inventory source node classifier arity"));
    }
    let type_names = type_rows
        .into_iter()
        .map(|r| text_after(lua, r, " = "))
        .collect::<Result<Vec<_>>>()?;
    let snapshot = tree.snapshot();
    let mut ids = Vec::new();
    for (&id, node) in &snapshot.nodes {
        let kind = raw_kind(&node.source.named);
        if id != node.id || source_id(node.source.named.get("skill"))? != id || kind != node.kind {
            return Err(error(
                "inventory full-tree raw node/classifier identity differs",
            ));
        }
        if type_names[kind_index(kind)] == node_type
            || source_truth(node.source.named.get(contained_field))
        {
            if ids.len() >= 65_536 {
                return Err(error("inventory full-tree socket count bound"));
            }
            ids.push(id);
        }
    }
    // The complete snapshot map is keyed by verified actual node.id, and the
    // original constructor sorts these numeric IDs. No Lua pairs order is used.
    Ok(ItemInventoryPassiveNodes {
        tree_version: snapshot.identity.tree_version.clone(),
        full_snapshot_sha256: tree.content_sha256().into(),
        ids,
    })
}

fn power_stats(lua: &Lua, definitions: &str, expansion: &str) -> Result<ItemInventoryPowerStats> {
    // Only the authenticated literal table runs in an empty environment.
    // Closures are created but never called, and source globals are untouched.
    let table: Table = lua
        .load(format!(
            "local data = {{}}\n{definitions}\nreturn data.powerStatList"
        ))
        .set_environment(lua.create_table()?)
        .eval()?;
    let source_rows = rows(definitions, "{ stat=")?;
    if source_rows.len() != table.raw_len() || source_rows.len() > 1024 {
        return Err(error("inventory power-stat literal row correspondence"));
    }
    let mut result = ItemInventoryPowerStats {
        rows: Vec::new(),
        transforms: Vec::new(),
    };
    for (index, source_row) in source_rows.iter().enumerate() {
        let entry: Table = table.raw_get(index + 1)?;
        let transform = match entry.raw_get::<Value>("transform")? {
            Value::Nil => None,
            Value::Function(_) => {
                let shape = after(source_row, "transform=function(value) return ")?;
                let transform = if shape.starts_with("-value end") {
                    ItemInventoryPowerTransform::Negate
                } else if shape.starts_with("value:gsub(") {
                    let [pattern, replacement] = fixed_strings(lua, after(shape, "value:gsub(")?)?;
                    ItemInventoryPowerTransform::Gsub {
                        pattern,
                        replacement,
                    }
                } else {
                    return Err(error("inventory power transform outside typed description"));
                };
                if result.transforms.len() >= 128 {
                    return Err(error("inventory power transform count bound"));
                }
                let id = u16::try_from(result.transforms.len()).map_err(error)?;
                result.transforms.push(transform);
                Some(id)
            }
            _ => return Err(error("inventory power transform is not nil or a function")),
        };
        result.rows.push(ItemInventoryPowerStat {
            stat: entry.raw_get("stat")?,
            label: entry.raw_get("label")?,
            transform,
        });
    }
    let mut excluded = Vec::new();
    for line in expansion
        .lines()
        .map(str::trim)
        .take_while(|line| *line != "}")
    {
        if let Some(name) = line.strip_suffix(" = true,") {
            if ident(name) != name {
                return Err(error("inventory minion exclusion is not an identifier"));
            }
            excluded.push(name);
        }
    }
    let pattern = text_after(
        lua,
        row(expansion, "if (not statEntry.stat)")?,
        "statEntry.stat:match(",
    )?;
    // The authenticated source uses a literal-word Lua pattern. Refuse expansion
    // to a pattern language here instead of silently changing its meaning.
    if pattern.is_empty() || !pattern.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(error(
            "inventory minion exclusion pattern is not a literal word",
        ));
    }
    let prefix = text_after(lua, row(expansion, "minionStat.stat = ")?, " = ")?;
    let label_prefix = text_after(lua, row(expansion, "minionStat.label = ")?, " = ")?;
    let initial_len = result.rows.len();
    for index in 0..initial_len {
        let original = &result.rows[index];
        let Some(stat) = &original.stat else { continue };
        if stat.contains(&pattern) || excluded.contains(&stat.as_str()) {
            continue;
        }
        let label = original
            .label
            .as_ref()
            .ok_or_else(|| error("inventory minion row has nil source label"))?;
        let copied = ItemInventoryPowerStat {
            stat: Some(format!("{prefix}{stat}")),
            label: Some(format!("{label_prefix}{label}")),
            transform: original.transform,
        };
        if result.rows.len() >= 1024 {
            return Err(error("inventory expanded power-stat row bound"));
        }
        result.rows.push(copied);
    }
    Ok(result)
}

pub(super) fn extract(
    lua: &Lua,
    bodies: &BTreeMap<String, &str>,
    tree: &AuthenticatedTreeSnapshot,
) -> Result<ItemInventoryPolicy> {
    authenticated(bodies)?;
    let declarations = body(bodies, "inventory_declarations")?;
    let layout = body(bodies, "inventory_slot_layout")?;
    let passive = body(bodies, "inventory_passive_layout")?;
    let control = body(bodies, "inventory_slot_control")?;
    let load = body(bodies, "inventory_load")?;
    let create = body(bodies, "inventory_create_set")?;
    let initial = body(bodies, "inventory_initial_set")?;
    let base_slots = strings(lua, row(declarations, "local baseSlots = ")?)?;
    let rune_slots = rows(declarations, "{ ")?
        .into_iter()
        .map(|r| {
            let [name, slot_type, label] = fixed_strings(lua, r)?;
            Ok(ItemInventoryRuneSlot {
                name,
                slot_type,
                label,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let weapon_patterns = rows(layout, "if slotName:match(")?
        .into_iter()
        .map(|r| text_after(lua, r, ":match("))
        .collect::<Result<Vec<_>>>()?;
    if weapon_patterns.len() != 2 || weapon_patterns[0] != weapon_patterns[1] {
        return Err(error("inventory shared weapon pattern differs"));
    }
    let embedded = row(layout, "local jewel = ")?;
    let socket_test = row(passive, "if node.type == ")?;
    let node_type = text_after(lua, socket_test, " == ")?;
    let contained_socket_field = ident(after(socket_test, " or node.")?).to_owned();
    let [slot_prefix, label] = fixed_strings(
        lua,
        after(row(passive, "local socketControl = ")?, "self, ")?,
    )?;
    let defaults = ItemInventoryDefaults {
        first_set_id: num_after(create, "\t\titemSet.id = ")?,
        set_id_increment: num_after(row(create, "itemSet.id = itemSet.id + ")?, " + ")?,
        reset_active_set_id: num_after(row(load, "self.activeItemSetId = ")?, " = ")?,
        empty_item_id: num_after(
            row(create, "itemSet[slotName] = { selItemId = ")?,
            "selItemId = ",
        )?,
        default_set_title: text_after(
            lua,
            row(load, "local itemSet = self:CreateItemSet(")?,
            "node.attrib.title or ",
        )?,
        empty_rune_name: text_after(
            lua,
            row(create, "itemSet[slotName] = { runeName = ")?,
            "runeName = ",
        )?,
        empty_slot_name: text_after(
            lua,
            row(load, "local slot = self.slots[")?,
            "node.attrib.name or ",
        )?,
        empty_url: text_after(
            lua,
            row(load, "itemSet[slotName].pbURL = ")?,
            "child.attrib.itemPbURL or ",
        )?,
        true_token: text_after(lua, row(load, "itemSet.useSecondWeaponSet = ")?, " == ")?,
        empty_item_label: text_after(lua, row(control, "self.list[1] = ")?, " = ")?,
        show_stat_differences: literal_bool(after(
            row(
                body(bodies, "inventory_initial_fields")?,
                "self.showStatDifferences = ",
            )?,
            " = ",
        )?)?,
    };
    // Shared defaults are one operand only after their source occurrences agree.
    if num_after(control, "\tself.selItemId = ")? != defaults.empty_item_id
        || num_after(row(initial, "self:CreateItemSet(")?, "self:CreateItemSet(")?
            != defaults.first_set_id
        || text_after(lua, row(initial, "self:CreateItemSet(")?, ", ")?
            != defaults.default_set_title
        || text_after(lua, row(load, "local runeName = ")?, " or ")? != defaults.empty_rune_name
        || rows(load, "local slotName = ")?
            .iter()
            .any(|r| text_after(lua, r, " or ").ok().as_ref() != Some(&defaults.empty_slot_name))
        || rows(load, "itemSet[id] = ")?
            .iter()
            .any(|r| text_after(lua, r, " or ").ok().as_ref() != Some(&defaults.empty_url))
    {
        return Err(error("inventory shared source defaults differ"));
    }
    let result = ItemInventoryPolicy {
        layout: ItemInventoryLayout {
            base_slots,
            swap: ItemInventorySwap {
                slot_pattern: weapon_patterns[0].clone(),
                suffix: text_after(lua, row(layout, "swapSlot = ")?, "slotName .. ")?,
                primary_weapon_set: count(after(row(layout, "slot.weaponSet = ")?, " = ")?)?,
                alternate_weapon_set: count(after(row(layout, "swapSlot.weaponSet = ")?, " = ")?)?,
            },
            embedded: ItemInventoryEmbedded {
                parent_slots: strings(lua, row(layout, "if slotName == ")?)?,
                count: count(after(row(layout, "for i = 1, ")?, "for i = 1, ")?)?,
                name_infix: text_after(lua, embedded, "parentSlot.slotName .. ")?,
                label_prefix: text_after(lua, embedded, ".. i, ")?,
            },
            passive: ItemInventoryPassive {
                nodes: passive_nodes(
                    lua,
                    body(bodies, "inventory_tree_types")?,
                    tree,
                    &node_type,
                    &contained_socket_field,
                )?,
                node_type,
                contained_socket_field,
                slot_prefix,
                label,
            },
            rune_slots,
            slot_number_patterns: fixed_strings(lua, row(control, "self.slotNum = ")?)?,
            activation_patterns: [
                text_after(lua, row(control, "if slotName:match(")?, ":match(")?,
                text_after(lua, row(control, "elseif slotName:match(")?, ":match(")?,
            ],
        },
        defaults,
        power_stats: power_stats(
            lua,
            body(bodies, "inventory_power_rows")?,
            body(bodies, "inventory_power_minions")?,
        )?,
    };
    result.validate().map_err(error)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_bodies() -> BTreeMap<String, String> {
        spans()
            .into_iter()
            .map(|(role, span)| {
                let source = std::fs::read_to_string(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../vendor/path-of-building-poe2")
                        .join(span.path),
                )
                .unwrap()
                .replace("\r\n", "\n");
                let body = source
                    .split_inclusive('\n')
                    .skip(span.line as usize - 1)
                    .take((span.end_line - span.line + 1) as usize)
                    .collect::<String>();
                (role, body)
            })
            .collect()
    }
    #[test]
    fn complete_inventory_source_pins_reject_a_literal_only_change() {
        let mut source = source_bodies();
        let borrowed = source
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str()))
            .collect();
        authenticated(&borrowed).unwrap();
        let body = source.get_mut("inventory_create_set").unwrap();
        *body = body.replace("selItemId = 0", "selItemId = 3");
        let borrowed = source
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str()))
            .collect();
        assert!(authenticated(&borrowed).is_err());
    }
    #[test]
    fn power_stat_projection_keeps_nil_first_match_and_shared_transform_identities() {
        let lua = Lua::new();
        let source = source_bodies();
        let p = power_stats(
            &lua,
            &source["inventory_power_rows"],
            &source["inventory_power_minions"],
        )
        .unwrap();
        assert_eq!(p.rows[0].stat, None);
        assert_eq!(p.rows[1].stat, None);
        assert_ne!(p.rows[0].label, p.rows[1].label);
        let original = p
            .rows
            .iter()
            .find(|r| r.stat.as_deref() == Some("PhysicalTakenHit"))
            .unwrap();
        let copied = p
            .rows
            .iter()
            .find(|r| r.stat.as_deref() == Some("MinionPhysicalTakenHit"))
            .unwrap();
        let distinct = p
            .rows
            .iter()
            .find(|r| r.stat.as_deref() == Some("LightningTakenHit"))
            .unwrap();
        assert_eq!(original.transform, copied.transform);
        assert_ne!(original.transform, distinct.transform);
        assert_eq!(
            p.transforms[usize::from(original.transform.unwrap())],
            ItemInventoryPowerTransform::Negate
        );
        assert!(
            matches!(&p.transforms[usize::from(p.rows[1].transform.unwrap())],
            ItemInventoryPowerTransform::Gsub { pattern, replacement } if pattern == "^The " && replacement.is_empty())
        );
    }
    #[test]
    fn raw_tree_classifier_preserves_truthiness_and_source_precedence() {
        let mut fields = BTreeMap::from([
            ("classesStart".into(), SourceValue::Boolean(false)),
            ("isJewelSocket".into(), SourceValue::Boolean(true)),
            ("containJewelSocket".into(), SourceValue::Integer(0)),
        ]);
        assert_eq!(raw_kind(&fields), TreeNodeKind::Socket);
        assert!(source_truth(fields.get("containJewelSocket")));
        fields.insert(
            "classesStart".into(),
            SourceValue::Table(Default::default()),
        );
        assert_eq!(raw_kind(&fields), TreeNodeKind::ClassStart);
    }
}
