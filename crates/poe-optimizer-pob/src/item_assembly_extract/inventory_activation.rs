//! Complete source authentication for activation and rune-choice operands.
//! This does not execute the rune-row parser or admit a names-only effect.
use super::*;

pub(super) fn spans() -> BTreeMap<String, ItemSourceSpan> {
    [
        (
            "inventory_activate",
            "src/Classes/ItemsTab.lua",
            1628,
            1670,
            "1d580b92193c92cb3fe81a4011c70521d34000b764f2771143cb9428c9a5af0b",
        ),
        (
            "inventory_colors",
            "src/Data/Global.lua",
            7,
            88,
            "09793ea44db76613e20ea5d4c661c07ee73271443f8e347d698c0fd95902f7ed",
        ),
        (
            "inventory_dropdown_constructor",
            "src/Classes/DropDownControl.lua",
            20,
            64,
            "ec1091a6e35d07bbc5d30973d5b30b1b8deaf275b3d9ec9399d6e4613b6dfef0",
        ),
        (
            "inventory_dropdown_list",
            "src/Classes/DropDownControl.lua",
            524,
            530,
            "6ba2151c33e3516f652be4317824264ef5714913e8da2fa3fdd3563bb8dede49",
        ),
        (
            "inventory_dropdown_selection",
            "src/Classes/DropDownControl.lua",
            142,
            175,
            "0c4f8ca293af44669985a5ba4c37c3edd991aee72c4dc6017aa8d49117d11de4",
        ),
        (
            "inventory_new_set",
            "src/Classes/ItemsTab.lua",
            1591,
            1596,
            "f6af7380fca676dc2aff7b2d0e520bbde629d2cd777c174620bd5f906cf8c9bb",
        ),
        (
            "inventory_populate_slots",
            "src/Classes/ItemsTab.lua",
            1705,
            1709,
            "0f2788393342f875c6af6d7265421e9e46165af41b602f321f21ae95a3b1ff68",
        ),
        (
            "inventory_rune_choices",
            "src/Classes/ItemsTab.lua",
            2216,
            2248,
            "990bb0a3f2d653699fbb1c60a79b14255b13a3b20eff870d14286b2e6cd86781",
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
fn body<'a>(bodies: &'a BTreeMap<String, &str>, role: &str) -> Result<&'a str> {
    bodies
        .get(role)
        .copied()
        .ok_or_else(|| error(format!("missing activation source {role}")))
}
fn after<'a>(text: &'a str, marker: &str) -> Result<&'a str> {
    text.split_once(marker)
        .map(|(_, tail)| tail)
        .ok_or_else(|| error(format!("missing activation operand {marker}")))
}
fn number(text: &str) -> Result<f64> {
    let value = text
        .split(|c: char| !(c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E')))
        .next()
        .unwrap_or("")
        .parse::<f64>()
        .map_err(error)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(error("nonfinite activation source operand"))
    }
}
fn colors(lua: &Lua, text: &str) -> Result<BTreeMap<String, String>> {
    let mut colors = BTreeMap::<String, String>::new();
    for row in text.lines().map(str::trim) {
        if row.is_empty() || row == "colorCodes = {" || row == "}" {
            continue;
        }
        if colors.len() >= 256 {
            return Err(error("activation color count bound"));
        }
        let (key, value) = row
            .split_once('=')
            .ok_or_else(|| error("activation color assignment shape"))?;
        let key = key.trim().strip_prefix("colorCodes.").unwrap_or(key.trim());
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(error("activation color key shape"));
        }
        let value = value.trim().trim_end_matches(',');
        let color = if let Some(alias) = value.strip_prefix("colorCodes.") {
            colors
                .get(alias)
                .cloned()
                .ok_or_else(|| error("activation color alias precedes its definition"))?
        } else {
            string(lua, value)?
        };
        if colors.insert(key.into(), color).is_some() {
            return Err(error("duplicate activation color"));
        }
    }
    // Exact values and absence of a metatable bind the original initialized
    // lookup, rather than assuming an unrelated same-named global is equivalent.
    let live: Table = lua.globals().raw_get("colorCodes")?;
    if live.metatable().is_some() {
        return Err(error("activation color lookup has a metatable"));
    }
    let mut count = 0;
    for entry in live.pairs::<Value, Value>() {
        count += 1;
        if count > 256 {
            return Err(error("activation live color count bound"));
        }
        let (Value::String(key), Value::String(value)) = entry? else {
            return Err(error("activation color lookup is not a string map"));
        };
        if colors.get(key.to_str()?.as_ref()).map(String::as_str) != Some(value.to_str()?.as_ref())
        {
            return Err(error(
                "activation live color differs from authenticated source",
            ));
        }
    }
    if count != colors.len() {
        return Err(error("activation live color membership differs"));
    }
    Ok(colors)
}
pub(super) fn policy(
    lua: &Lua,
    bodies: &BTreeMap<String, &str>,
) -> Result<ItemInventoryActivationPolicy> {
    for (role, span) in spans() {
        if hash(body(bodies, &role)?.as_bytes()) != span.sha256 {
            return Err(error(format!("changed complete activation source {role}")));
        }
    }
    let rows = body(bodies, "inventory_rune_choices")?;
    let empty = rows
        .lines()
        .next()
        .ok_or_else(|| error("missing empty rune source"))?;
    let layout = body(bodies, "inventory_slot_layout")?;
    let result = ItemInventoryActivationPolicy {
        rarity_colors: colors(lua, body(bodies, "inventory_colors")?)?,
        rune_choices: ItemInventoryRuneChoicePolicy {
            empty: ItemInventoryEmptyRuneChoice {
                name: string(lua, after(empty, "name = ")?)?,
                label: string(lua, after(empty, "label = ")?)?,
                line: string(lua, after(empty, "lines = { ")?)?,
                slot_type: string(lua, after(empty, "slot = ")?)?,
                required_level: number(after(empty, "req = ")?)?,
                order: number(after(empty, "order = ")?)?,
                group: number(after(empty, "group = ")?)?,
                is_socket_bound: match after(empty, "isSocketBound = ")?.split_whitespace().next() {
                    Some("false") => false,
                    Some("true") => true,
                    _ => return Err(error("empty rune socket binding operand")),
                },
            },
            order_default: number(
                rows.lines()
                    .find(|line| line.trim_start().starts_with("local order = "))
                    .and_then(|line| line.rsplit_once(" or "))
                    .map(|(_, tail)| tail)
                    .ok_or_else(|| error("missing rune default order"))?,
            )?,
            broad_slot_type: string(lua, after(layout, "or rune.slot == ")?)?,
            bonded_display_prefix: string(
                lua,
                rows.lines()
                    .find(|line| {
                        line.trim_start().starts_with("t_insert(lines, ")
                            && line.contains(" .. line)")
                    })
                    .and_then(|line| line.split_once("t_insert(lines, "))
                    .map(|(_, tail)| tail)
                    .ok_or_else(|| error("missing bonded display prefix"))?,
            )?,
            modifier_source_prefix: string(lua, after(rows, "modLib.setSource(mod, ")?)?,
        },
    };
    result.validate().map_err(error)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sources() -> BTreeMap<String, String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        spans()
            .into_iter()
            .chain(inventory::spans())
            .map(|(role, span)| {
                let full = std::fs::read_to_string(root.join(span.path))
                    .unwrap()
                    .replace("\r\n", "\n");
                let value = full
                    .split_inclusive('\n')
                    .skip(span.line as usize - 1)
                    .take((span.end_line - span.line + 1) as usize)
                    .collect();
                (role, value)
            })
            .collect()
    }
    fn host(bodies: &BTreeMap<String, String>) -> Lua {
        let lua = Lua::new();
        lua.load(&bodies["inventory_colors"]).exec().unwrap();
        lua
    }
    #[test]
    fn pinned_activation_operands_and_complete_color_aliases_are_extracted() {
        let owned = sources();
        let lua = host(&owned);
        let p = policy(
            &lua,
            &owned.iter().map(|(k, v)| (k.clone(), v.as_str())).collect(),
        )
        .unwrap();
        assert_eq!(p.rune_choices.empty.name, "None");
        assert_eq!(p.rune_choices.empty.order, -1.0);
        assert_eq!(p.rune_choices.empty.group, -1.0);
        assert_eq!(p.rune_choices.order_default, 0.0);
        assert_eq!(p.rune_choices.broad_slot_type, "armour");
        assert_eq!(p.rune_choices.bonded_display_prefix, "Bonded: ");
        assert_eq!(p.rune_choices.modifier_source_prefix, "Rune:");
        assert_eq!(p.rarity_colors["PHYS"], p.rarity_colors["NORMAL"]);
    }
    #[test]
    fn changed_selection_body_or_live_color_lookup_is_rejected() {
        let mut owned = sources();
        let lua = host(&owned);
        owned
            .get_mut("inventory_dropdown_selection")
            .unwrap()
            .push_str("-- changed");
        assert!(
            policy(
                &lua,
                &owned.iter().map(|(k, v)| (k.clone(), v.as_str())).collect()
            )
            .is_err()
        );
        let owned = sources();
        let lua = host(&owned);
        lua.globals()
            .get::<Table>("colorCodes")
            .unwrap()
            .raw_set("NORMAL", "changed")
            .unwrap();
        assert!(
            policy(
                &lua,
                &owned.iter().map(|(k, v)| (k.clone(), v.as_str())).collect()
            )
            .is_err()
        );
    }
}
