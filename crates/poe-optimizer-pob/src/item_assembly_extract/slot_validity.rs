//! Literal acquisition for the complete fixed ItemsTab slot-validity method.
//!
//! This role authenticates supplied source text without loading the ItemsTab UI.
//! The parity host separately checks the actual original Function. Even a
//! literal-only upstream change requires review of the complete-body pin here;
//! custom DATA policies have their own bounded validation and need no such pin.
use super::*;

pub(super) fn span() -> ItemSourceSpan {
    ItemSourceSpan {
        path: "src/Classes/ItemsTab.lua".into(),
        line: 2603,
        end_line: 2687,
        sha256: "b5235ee91e6d351d628fcb070862521f71c6ce1f6caa1e7f0efc2e53abd16c8c".into(),
    }
}
fn lines<'a>(body: &'a str, prefix: &str) -> Result<Vec<&'a str>> {
    let values = body
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with(prefix))
        .collect::<Vec<_>>();
    if values.is_empty() || values.len() > 64 {
        return Err(error(format!("slot-validity source rows {prefix}")));
    }
    Ok(values)
}
fn line<'a>(body: &'a str, prefix: &str) -> Result<&'a str> {
    let values = lines(body, prefix)?;
    if values.len() != 1 {
        return Err(error(format!(
            "ambiguous slot-validity source row {prefix}"
        )));
    }
    Ok(values[0])
}
fn strings(lua: &Lua, text: &str) -> Result<Vec<String>> {
    let mut rest = text;
    let mut values = Vec::new();
    while let Some(at) = rest.find(['"', '\'']) {
        if values.len() >= 64 {
            return Err(error("slot-validity literal bound"));
        }
        let quoted = &rest[at..];
        values.push(string(lua, quoted)?);
        rest = &quoted[quoted_end(quoted)?..];
    }
    Ok(values)
}
fn fixed_strings<const N: usize>(lua: &Lua, text: &str) -> Result<[String; N]> {
    strings(lua, text)?
        .try_into()
        .map_err(|_| error("slot-validity literal arity"))
}
fn fields(text: &str, prefix: &str) -> Result<Vec<String>> {
    let mut values = Vec::new();
    for rest in text.split(prefix).skip(1) {
        let name = ident(rest);
        if name.is_empty() || values.len() >= 64 {
            return Err(error("slot-validity field bound"));
        }
        values.push(name.to_owned());
    }
    if values.is_empty() {
        return Err(error("missing slot-validity field"));
    }
    Ok(values)
}
fn field(text: &str, prefix: &str) -> Result<String> {
    let name = ident(after(text, prefix)?);
    if name.is_empty() {
        return Err(error("empty slot-validity field"));
    }
    Ok(name.to_owned())
}
fn boolean(text: &str) -> Result<bool> {
    match text.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(error("slot-validity default is not a Boolean literal")),
    }
}

pub(super) fn extract(lua: &Lua, body: &str) -> Result<ItemSlotValidityPolicy> {
    if hash(body.as_bytes()) != span().sha256 {
        return Err(error("changed complete slot-validity source"));
    }
    let node = lines(body, "elseif node.")?;
    let [sinister, contained, charm]: [&str; 3] = node
        .try_into()
        .map_err(|_| error("slot-validity node branch count"))?;
    let unique_rarities = fixed_strings(lua, sinister)?;
    if fixed_strings::<2>(lua, line(body, "if item.rarity == ")?)? != unique_rarities {
        return Err(error("slot-validity shared rarity association"));
    }
    let charm_subtype = text_after(lua, charm, "item.base.subType == ")?;
    let charm_check = line(body, "if node.")?;
    if field(charm_check, "node.")? != field(charm, "node.")?
        || text_after(lua, charm_check, "item.base.subType == ")? != charm_subtype
    {
        return Err(error("slot-validity shared charm association"));
    }
    let cluster_guards = lines(body, "elseif item.")?
        .into_iter()
        .filter(|row| row.contains(" and not node."))
        .collect::<Vec<_>>();
    let [cluster_guard]: [&str; 1] = cluster_guards
        .try_into()
        .map_err(|_| error("slot-validity cluster guard count"))?;
    let outer = line(body, "elseif not node.")?;
    let fit = line(body, "return not item.")?;
    let expansion_field = field(cluster_guard, "node.")?;
    let outer_fields = fields(outer, "node.")?;
    if outer_fields != [expansion_field.clone(), expansion_field.clone()]
        || field(fit, "node.")? != expansion_field
    {
        return Err(error("slot-validity shared expansion association"));
    }
    let cluster_field = field(cluster_guard, "item.")?;
    if fields(fit, "item.")? != [cluster_field.clone(), cluster_field.clone()] {
        return Err(error("slot-validity shared cluster association"));
    }
    let jewel = ItemJewelSlotPolicy {
        slot_type: text_after(lua, line(body, "if slotType == ")?, " == ")?,
        item_type: text_after(lua, line(body, "if not node or ")?, "item.type ~= ")?,
        unique_rarities,
        sinister_field: field(sinister, "node.")?,
        contained_socket_field: field(contained, "node.")?,
        charm_socket_field: field(charm, "node.")?,
        expansion_size_field: field(outer, &format!("node.{expansion_field}."))?,
        expansion_field,
        cluster_size_field: field(fit, &format!("item.{cluster_field}."))?,
        cluster_field,
        charm_subtype,
        outer_size: num_after(outer, " == ")?,
    };
    let typed = lines(body, "elseif item.type == \"")?;
    let [flask, embedded]: [&str; 2] = typed
        .try_into()
        .map_err(|_| error("slot-validity item branch count"))?;
    let [flask_item_type, flask_slot_type] = fixed_strings(lua, flask)?;
    let flask_routes = [
        line(body, "if item.baseName:match(")?,
        line(body, "elseif item.baseName:match(")?,
    ]
    .map(|row| -> Result<ItemFlaskSlotRoute> {
        let [base_name_pattern, slot_name_pattern] = fixed_strings(lua, row)?;
        Ok(ItemFlaskSlotRoute {
            base_name_pattern,
            slot_name_pattern,
        })
    });
    let [route_one, route_two] = flask_routes;
    let subtype_rows = lines(body, "elseif item.base.subType == ")?;
    let [subtype_one, subtype_two]: [&str; 2] = subtype_rows
        .try_into()
        .map_err(|_| error("slot-validity subtype branch count"))?;
    let subtype = |row| -> Result<ItemSubtypeSlotRule> {
        let [base_subtype, slot_type] = fixed_strings(lua, row)?;
        Ok(ItemSubtypeSlotRule {
            base_subtype,
            slot_type,
        })
    };
    let [embedded_item, slot_pattern, excluded_rarity] = fixed_strings(lua, embedded)?;
    let embedded = ItemEmbeddedJewelSlotPolicy {
        item_type: embedded_item,
        slot_pattern,
        excluded_rarity,
        parent_rewrite: rewrite(lua, line(body, "local parentSlotName = ")?)?,
        restriction_field: field(line(body, "if slotItem and ")?, "not slotItem.")?,
    };
    let weapon_rows = lines(body, "elseif slotName == ")?;
    let [primary, offhand]: [&str; 2] = weapon_rows
        .try_into()
        .map_err(|_| error("slot-validity weapon branch count"))?;
    let [offhand_one, offhand_two] = fixed_strings(lua, offhand)?;
    let selection = line(body, "local weapon1Sel = ")?;
    let [selector, primary_one, primary_two] = fixed_strings(lua, selection)?;
    if selector != offhand_one {
        return Err(error("slot-validity offhand association"));
    }
    let defaults = after(line(body, "local giantsBlood, ")?, " = ")?;
    let defaults = defaults
        .split(',')
        .map(boolean)
        .collect::<Result<Vec<_>>>()?;
    let [default_giants, default_instruments, default_lord]: [bool; 3] = defaults
        .try_into()
        .map_err(|_| error("slot-validity flag default count"))?;
    let flag = |local: &str, default| -> Result<ItemSlotValidityFlag> {
        let state = line(body, &format!("{local} = flagState."))?;
        let query = line(
            body,
            &format!("{local} = self.build.calcsTab.mainEnv.modDB:Flag("),
        )?;
        Ok(ItemSlotValidityFlag {
            state_field: field(state, "flagState.")?,
            query_name: text_after(lua, query, ":Flag(nil, ")?,
            default,
        })
    };
    let weapon_types = lines(body, "elseif weapon1Base.type == ")?;
    let [talisman, staff]: [&str; 2] = weapon_types
        .try_into()
        .map_err(|_| error("slot-validity weapon type branch count"))?;
    let returns = lines(body, "return item.type == ")?;
    let [quiver, sceptre, focus, ordinary]: [&str; 4] = returns
        .try_into()
        .map_err(|_| error("slot-validity return branch count"))?;
    let [sceptre_type, excluded_one, excluded_two] = fixed_strings(lua, sceptre)?;
    let ordinary_guard = line(body, "elseif weapon1Base == ")?;
    let guarded_tags = fields(ordinary_guard, "weapon1Base.tags.")?;
    let (onehand_tag, giant_tags) = guarded_tags
        .split_first()
        .ok_or_else(|| error("missing slot-validity weapon tags"))?;
    if fields(line(body, "or (giantsBlood and ")?, "item.base.tags.")? != giant_tags {
        return Err(error("slot-validity shared giant tag association"));
    }
    let dual = line(body, "or (item.base.tags.")?;
    let primary_tags: [String; 2] =
        fields(line(body, "return item.base.tags.")?, "item.base.tags.")?
            .try_into()
            .map_err(|_| error("slot-validity primary tag arity"))?;
    let selected_sentinel = text_after(lua, line(body, "local weapon1Base = ")?, "base or ")?;
    if text_after(lua, ordinary_guard, "weapon1Base == ")? != selected_sentinel {
        return Err(error("slot-validity shared unarmed association"));
    }
    Ok(ItemSlotValidityPolicy {
        slot_pattern: text_after(lua, line(body, "local slotType, slotId = ")?, ":match(")?,
        jewel,
        flask: ItemFlaskSlotPolicy {
            item_type: flask_item_type,
            slot_type: flask_slot_type,
            routes: [route_one?, route_two?],
        },
        subtypes: [subtype(subtype_one)?, subtype(subtype_two)?],
        embedded,
        weapon: ItemWeaponSlotValidityPolicy {
            primary_slots: strings(lua, primary)?,
            offhand_slots: [
                ItemOffhandSlotLink {
                    offhand: offhand_one,
                    primary: primary_one,
                },
                ItemOffhandSlotLink {
                    offhand: offhand_two,
                    primary: primary_two,
                },
            ],
            empty_selection: num_after(selection, ".selItemId or ")?,
            unarmed_sentinel: selected_sentinel,
            primary_tags,
            onehand_tag: onehand_tag.clone(),
            dual_wield_tag: field(dual, "item.base.tags.")?,
            giants_blood: flag("giantsBlood", default_giants)?,
            instruments_of_power: flag("instrumentsOfPower", default_instruments)?,
            lord_of_the_wilds: flag("lordOfTheWilds", default_lord)?,
            bow_type: text_after(lua, line(body, "if weapon1Base.type == ")?, " == ")?,
            quiver_type: text_after(lua, quiver, " == ")?,
            talisman_type: text_after(lua, talisman, " == ")?,
            sceptre_type,
            staff_type: text_after(lua, staff, " == ")?,
            focus_type: text_after(lua, focus, " == ")?,
            talisman_excluded_rarities: [excluded_one, excluded_two],
            ordinary_offhand_types: strings(lua, ordinary)?,
            excluded_primary_types: dual
                .split("weapon1Base.type ~= ")
                .skip(1)
                .map(|tail| string(lua, tail))
                .collect::<Result<_>>()?,
            excluded_offhand_type: text_after(lua, dual, "item.type ~= ")?,
            giant_tags: giant_tags.to_vec(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_source_extracts_all_branch_operands_and_relations() {
        let body = source_body(super::super::tests::sources(), &span()).unwrap();
        let p = extract(&Lua::new(), body).unwrap();
        assert_eq!(p.slot_pattern, "^([%a ]+) (%d+)$");
        assert_eq!(p.jewel.unique_rarities, ["UNIQUE", "RELIC"]);
        assert_eq!(p.jewel.contained_socket_field, "containJewelSocket");
        assert_eq!(p.jewel.cluster_size_field, "sizeIndex");
        assert_eq!(p.jewel.outer_size, 2.0);
        assert_eq!(p.flask.routes[1].base_name_pattern, "Mana Flask");
        assert_eq!(p.flask.routes[0].slot_name_pattern, "Flask 1");
        assert_eq!(p.subtypes[1].base_subtype, "Transcendent Leg");
        assert_eq!(p.embedded.restriction_field, "canSocketJewelBase");
        assert_eq!(p.embedded.parent_rewrite.pattern, " Jewel Socket %d");
        assert_eq!(
            p.weapon.primary_slots,
            ["Weapon 1", "Weapon 1 Swap", "Weapon"]
        );
        assert_eq!(p.weapon.offhand_slots[1].primary, "Weapon 1 Swap");
        assert_eq!(p.weapon.empty_selection, 0.0);
        assert_eq!(p.weapon.primary_tags, ["onehand", "twohand"]);
        assert_eq!(p.weapon.giants_blood.state_field, "giantsBlood");
        assert_eq!(
            p.weapon.instruments_of_power.query_name,
            "InstrumentsOfPower"
        );
        assert!(p.weapon.lord_of_the_wilds.default);
        assert_eq!(
            p.weapon.ordinary_offhand_types,
            ["Shield", "Focus", "Sceptre"]
        );
        assert_eq!(p.weapon.excluded_primary_types, ["Wand", "Sceptre"]);
        assert_eq!(p.weapon.excluded_offhand_type, "Spear");
        assert_eq!(p.weapon.giant_tags, ["axe", "mace", "sword"]);
    }
    #[test]
    fn literal_control_flow_and_final_end_changes_require_source_review() {
        let original = super::super::tests::sources();
        for (old, new) in [
            ("size == 2", "size == 3"),
            ("item.type ~= \"Jewel\"", "item.type == \"Jewel\""),
            (
                "\nend\n\n-- Ensure weapon 2",
                "\nreturn false\nend\n\n-- Ensure weapon 2",
            ),
        ] {
            let mut changed = original.clone();
            let text = changed.get_mut(&span().path).unwrap();
            assert!(text.contains(old));
            *text = text.replace(old, new);
            assert!(source_body(&changed, &span()).is_err());
        }
    }
}
