//! Parameter acquisition after complete original methods and live helpers authenticate.
use super::*;

struct ListCall {
    name: String,
    start: usize,
    end: usize,
}

fn member(text: &str, prefix: &str) -> Result<String> {
    let name = ident(after(text, prefix)?);
    if name.is_empty() {
        return Err(error("missing jewel field"));
    }
    Ok(name.into())
}

fn branch<'a>(lua: &Lua, text: &'a str, item_type: &str) -> Result<&'a str> {
    let start = after(text, "elseif self.type == ")?.trim_start();
    if string(lua, start)? != item_type {
        return Err(error("jewel reset and local predicates disagree"));
    }
    start[quoted_end(start)?..]
        .trim_start()
        .strip_prefix("then")
        .ok_or_else(|| error("jewel branch condition"))
}

fn list_calls(lua: &Lua, text: &str) -> Result<Vec<ListCall>> {
    let mut calls = Vec::new();
    let mut offset = 0;
    while let Some(relative) = text[offset..].find("modList:List(") {
        if calls.len() == 7 {
            return Err(error("extra jewel List query"));
        }
        let start = offset + relative;
        let args_start = start + "modList:List(".len();
        let values = args(&text[args_start..])?;
        if values.len() != 2 || values[0] != "nil" || quoted_end(values[1])? != values[1].len() {
            return Err(error("jewel List query shape"));
        }
        let close = text[args_start..]
            .find(')')
            .ok_or_else(|| error("jewel query close"))?;
        offset = args_start + close + 1;
        calls.push(ListCall {
            name: string(lua, values[1])?,
            start,
            end: offset,
        });
    }
    if calls.len() != 7 {
        return Err(error("incomplete jewel List query inventory"));
    }
    Ok(calls)
}

fn override_policy(query: &ListCall, stage: &str) -> Result<ItemAssemblyOverridePolicy> {
    let key_field = member(stage, "[value.")?;
    let value_field = member(stage, " = value.")?;
    Ok(ItemAssemblyOverridePolicy {
        query_name: query.name.clone(),
        key_field,
        value_field,
    })
}

fn fields(text: &str, prefix: &str) -> Vec<String> {
    let mut rest = text;
    let mut result = Vec::new();
    while let Some((_, next)) = rest.split_once(prefix) {
        result.push(ident(next).into());
        rest = next;
    }
    result
}

fn field_assignment<'a>(text: &'a str, field: &str) -> Result<&'a str> {
    let prefix = format!("jewelData.{field} = ");
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&prefix))
        .ok_or_else(|| error("jewel assignment"))
}

fn require(text: &str, expected: &str) -> Result<()> {
    if text.contains(expected) {
        Ok(())
    } else {
        Err(error(format!("inconsistent jewel operand use: {expected}")))
    }
}

pub(super) fn extract(
    lua: &Lua,
    build: &str,
    slot: &str,
    item_type: &str,
) -> Result<ItemAssemblyJewelPolicy> {
    let body = branch(lua, slot, item_type)?;
    let reset = branch(lua, build, item_type)?;
    let output_field = member(reset, "self.")?;
    let reset_line = reset
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| error("jewel reset"))?;
    if reset_line != format!("self.{output_field} = {{ }}")
        || member(body, "local jewelData = self.")? != output_field
    {
        return Err(error("jewel reset/output mismatch"));
    }
    let name_item_field = member(body, "if self.")?;
    let name_pattern = text_after(lua, body, ":find(")?;
    let spectrum = args(after(body, "modLib.createMod(")?)?;
    let minion = args(after(body, "modList:NewMod(")?)?;
    if spectrum.len() != 4
        || minion.len() != 4
        || spectrum[3] != format!("self.{name_item_field}")
        || minion[3] != spectrum[3]
    {
        return Err(error("jewel spectrum arguments"));
    }
    let nested = minion[2]
        .trim()
        .strip_prefix('{')
        .and_then(|v| v.strip_suffix('}'))
        .ok_or_else(|| error("jewel spectrum nested record"))?
        .trim();
    let (nested_mod_field, reference) = nested
        .split_once(" = ")
        .ok_or_else(|| error("jewel spectrum nested field"))?;
    if ident(nested_mod_field) != nested_mod_field || reference != "spectrumMod" {
        return Err(error("jewel spectrum alias expression"));
    }
    require(body, "modList:AddMod(spectrumMod)")?;
    let calls = list_calls(lua, body)?;
    let stage = |index: usize| -> &str {
        &body[calls[index].end..calls.get(index + 1).map_or(body.len(), |next| next.start)]
    };
    let functions = ItemAssemblyJewelListField {
        query_name: calls[0].name.clone(),
        output_field: member(stage(0), "jewelData.")?,
    };
    require(
        stage(0),
        &format!(
            "jewelData.{0} = jewelData.{0} or {{ }}",
            functions.output_field
        ),
    )?;
    require(
        stage(0),
        &format!("t_insert(jewelData.{}, func)", functions.output_field),
    )?;
    let alternate_class_start = ItemAssemblyJewelListField {
        query_name: calls[2].name.clone(),
        output_field: member(stage(2), "jewelData.")?,
    };
    require(
        stage(2),
        &format!(
            "jewelData.{} = className",
            alternate_class_start.output_field
        ),
    )?;
    let from_nothing_output = member(stage(3), "jewelData.")?;
    require(
        stage(3),
        &format!("jewelData.{from_nothing_output} = {{ }}"),
    )?;
    if calls[3].name != calls[4].name {
        return Err(error("jewel repeated FromNothing query changed"));
    }
    require(stage(4), &format!("jewelData.{from_nothing_output}[value."))?;
    let item_field = member(stage(4), "if self.")?;
    let notables = ItemAssemblyJewelListField {
        query_name: calls[5].name.clone(),
        output_field: member(stage(5), "jewelData.")?,
    };
    let added_mods = ItemAssemblyJewelListField {
        query_name: calls[6].name.clone(),
        output_field: member(stage(6), "jewelData.")?,
    };
    require(
        stage(4),
        &format!("jewelData.{} = {{ }}", notables.output_field),
    )?;
    require(
        stage(5),
        &format!("t_insert(jewelData.{}, name)", notables.output_field),
    )?;
    require(
        stage(5),
        &format!("jewelData.{} = {{ }}", added_mods.output_field),
    )?;
    require(
        stage(6),
        &format!("t_insert(jewelData.{}, line)", added_mods.output_field),
    )?;
    let tail = stage(6);
    let correction = tail
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("if jewelData.") && line.contains(" < "))
        .ok_or_else(|| error("jewel skill correction"))?;
    let skill_field = member(correction, "if jewelData.")?;
    let matching_skill = text_after(lua, correction, " == ")?;
    let node_count_field = member(correction, " and jewelData.")?;
    let node_count_below = num_after(correction, " < ")?;
    let correction_fields = fields(correction, "jewelData.");
    if correction_fields
        != [
            skill_field.clone(),
            node_count_field.clone(),
            node_count_field.clone(),
        ]
    {
        return Err(error("jewel correction field references"));
    }
    let replacement_skill = string(
        lua,
        field_assignment(after(tail, correction)?, &skill_field)?,
    )?;
    let clamp = tail
        .lines()
        .map(str::trim)
        .find(|line| line.contains(" = m_min(m_max("))
        .ok_or_else(|| error("jewel node clamp"))?;
    if member(clamp, "jewelData.")? != node_count_field {
        return Err(error("jewel clamp output"));
    }
    let outer = args(after(clamp, "m_min(")?)?;
    if outer.len() != 2 {
        return Err(error("jewel minimum arity"));
    }
    let inner = args(after(outer[0], "m_max(")?)?;
    if inner.len() != 2 || inner[0] != format!("jewelData.{node_count_field}") {
        return Err(error("jewel maximum operands"));
    }
    let min_nodes_field = member(inner[1], &format!("self.{item_field}."))?;
    let max_nodes_field = member(outer[1], &format!("self.{item_field}."))?;
    let validation = tail
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(&format!("if jewelData.{skill_field} and not self.")))
        .ok_or_else(|| error("jewel skill lookup"))?;
    let skills_field = member(validation, &format!("self.{item_field}."))?;
    require(
        validation,
        &format!("self.{item_field}.{skills_field}[jewelData.{skill_field}]"),
    )?;
    require(
        after(tail, validation)?,
        &format!("jewelData.{skill_field} = nil"),
    )?;

    let lines = tail.lines().map(str::trim).collect::<Vec<_>>();
    let (index, validity_line) = lines
        .iter()
        .enumerate()
        .find(|(_, line)| {
            line.split_once(" = jewelData.")
                .is_some_and(|(_, rhs)| ident(rhs) == rhs)
        })
        .ok_or_else(|| error("jewel validity root"))?;
    let validity_output = member(validity_line, "jewelData.")?;
    let keystone_field = member(validity_line, " = jewelData.")?;
    let second = *lines
        .get(index + 1)
        .ok_or_else(|| error("jewel validity second clause"))?;
    let third = *lines
        .get(index + 2)
        .ok_or_else(|| error("jewel validity third clause"))?;
    let second_fields = fields(second, "jewelData.");
    let third_fields = fields(third, "jewelData.");
    if second_fields.len() != 3
        || third_fields.len() != 2
        || second_fields[0] != skill_field
        || second_fields[2] != node_count_field
        || second
            != format!(
                "or ((jewelData.{} or jewelData.{}) and jewelData.{})",
                skill_field, second_fields[1], node_count_field
            )
        || third
            != format!(
                "or (jewelData.{} and jewelData.{})",
                third_fields[0], third_fields[1]
            )
    {
        return Err(error("jewel validity operand structure"));
    }
    Ok(ItemAssemblyJewelPolicy {
        output_field,
        grand_spectrum: ItemAssemblyJewelSpectrum {
            name_item_field,
            name_pattern,
            modifier_name: string(lua, spectrum[0])?,
            modifier_type: string(lua, spectrum[1])?,
            modifier_value: number(spectrum[2])?,
            minion_name: string(lua, minion[0])?,
            minion_type: string(lua, minion[1])?,
            nested_mod_field: nested_mod_field.into(),
        },
        functions,
        overrides: override_policy(&calls[1], stage(1))?,
        alternate_class_start,
        from_nothing: ItemAssemblyJewelFromNothing {
            guard_query_name: calls[3].name.clone(),
            output_field: from_nothing_output,
            entries: override_policy(&calls[4], stage(4))?,
        },
        cluster: ItemAssemblyJewelCluster {
            item_field,
            notables,
            added_mods,
            skill_field,
            node_count_field,
            skills_field,
            min_nodes_field,
            max_nodes_field,
            correction: ItemAssemblyJewelSkillCorrection {
                matching_skill,
                replacement_skill,
                node_count_below,
            },
            validity: ItemAssemblyJewelValidity {
                output_field: validity_output,
                keystone_field,
                smalls_are_nothingness_field: second_fields[1].clone(),
                socket_count_override_field: third_fields[0].clone(),
                nothingness_count_field: third_fields[1].clone(),
            },
        },
    })
}
