//! Extract parameters only after the complete original method and helper bindings authenticate.
use super::*;

const ARMOUR_ROLES: [(&str, ItemAssemblyArmourRole); 18] = [
    ("armourBase", ItemAssemblyArmourRole::ArmourBase),
    (
        "armourEvasionBase",
        ItemAssemblyArmourRole::ArmourEvasionBase,
    ),
    ("evasionBase", ItemAssemblyArmourRole::EvasionBase),
    (
        "evasionEnergyShieldBase",
        ItemAssemblyArmourRole::EvasionEnergyShieldBase,
    ),
    ("energyShieldBase", ItemAssemblyArmourRole::EnergyShieldBase),
    (
        "armourEnergyShieldBase",
        ItemAssemblyArmourRole::ArmourEnergyShieldBase,
    ),
    ("wardBase", ItemAssemblyArmourRole::WardBase),
    ("evasionPerLevel", ItemAssemblyArmourRole::EvasionPerLevel),
    (
        "energyShieldPerLevel",
        ItemAssemblyArmourRole::EnergyShieldPerLevel,
    ),
    ("wardPerLevel", ItemAssemblyArmourRole::WardPerLevel),
    ("armourInc", ItemAssemblyArmourRole::ArmourIncreased),
    (
        "armourEvasionInc",
        ItemAssemblyArmourRole::ArmourEvasionIncreased,
    ),
    ("evasionInc", ItemAssemblyArmourRole::EvasionIncreased),
    (
        "evasionEnergyShieldInc",
        ItemAssemblyArmourRole::EvasionEnergyShieldIncreased,
    ),
    (
        "energyShieldInc",
        ItemAssemblyArmourRole::EnergyShieldIncreased,
    ),
    ("wardInc", ItemAssemblyArmourRole::WardIncreased),
    (
        "armourEnergyShieldInc",
        ItemAssemblyArmourRole::ArmourEnergyShieldIncreased,
    ),
    ("defencesInc", ItemAssemblyArmourRole::DefencesIncreased),
];
fn array<T, const N: usize>(rows: Vec<T>) -> Result<[T; N]> {
    rows.try_into()
        .map_err(|_| error("local assembly role count"))
}
fn branch<'a>(slot: &'a str, kind: &str, next: &str) -> Result<&'a str> {
    after(slot, &format!("elseif self.base.{kind} then"))?
        .split_once(next)
        .map(|(body, _)| body)
        .ok_or_else(|| error("local assembly branch extent"))
}
fn field(text: &str, prefix: &str) -> Result<String> {
    let value = ident(after(text, prefix)?);
    if value.is_empty() {
        Err(error("local assembly field"))
    } else {
        Ok(value.into())
    }
}
fn assignment<'a>(text: &'a str, lhs: &str) -> Result<&'a str> {
    let prefix = format!("{lhs} = ");
    let mut matches = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix(&prefix));
    let value = matches
        .next()
        .ok_or_else(|| error(format!("missing local assignment {lhs}")))?;
    if matches.next().is_some() {
        return Err(error("duplicate local assignment"));
    }
    Ok(value)
}
fn assignments<'a>(text: &'a str, object: &str) -> Vec<(&'a str, &'a str)> {
    let prefix = format!("{object}.");
    text.lines()
        .filter_map(|line| line.trim().strip_prefix(&prefix)?.split_once(" = "))
        .collect()
}
fn role_terms(text: &str, increased: bool) -> Vec<ItemAssemblyArmourRole> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter_map(|word| ARMOUR_ROLES.iter().find(|(name, _)| *name == word))
        .filter(|(name, _)| name.ends_with("Inc") == increased)
        .map(|(_, role)| *role)
        .collect()
}
fn base_field(rhs: &str, prefix: &str) -> Result<ItemAssemblyBaseField> {
    Ok(ItemAssemblyBaseField {
        field: field(rhs, prefix)?,
        default: num_after(rhs, " or ")?,
    })
}
fn query_output(lua: &Lua, text: &str, object: &str, key: &str) -> Result<ItemAssemblyQueryOutput> {
    let rows = assignments(text, object);
    let (output, rhs) = rows
        .iter()
        .find(|(name, _)| *name == key)
        .ok_or_else(|| error("local query output field"))?;
    Ok(ItemAssemblyQueryOutput {
        output: (*output).into(),
        query: query(lua, rhs)?,
    })
}
fn override_policy(lua: &Lua, text: &str) -> Result<ItemAssemblyOverridePolicy> {
    let tail = after(text, "for _, value in ipairs(modList:List(nil, ")?;
    let query_name = string(lua, tail)?;
    let store = after(tail, "[value.")?;
    Ok(ItemAssemblyOverridePolicy {
        query_name,
        key_field: ident(store).into(),
        value_field: field(store, " = value.")?,
    })
}
fn armour(lua: &Lua, text: &str) -> Result<ItemAssemblyArmourPolicy> {
    let mut queries = Vec::new();
    for line in text.lines() {
        let Some((name, rhs)) = line
            .trim()
            .strip_prefix("local ")
            .and_then(|s| s.split_once(" = "))
        else {
            continue;
        };
        if let Some((_, role)) = ARMOUR_ROLES.iter().find(|(known, _)| *known == name) {
            queries.push(ItemAssemblyArmourQuery {
                role: *role,
                query: query(lua, rhs)?,
                base: if rhs.contains("self.base.armour.") {
                    Some(base_field(rhs, "self.base.armour.")?)
                } else {
                    None
                },
            });
        }
    }
    let rows = assignments(text, "armourData");
    if rows.len() != 12 {
        return Err(error("armour output inventory"));
    }
    let mut defences = Vec::new();
    for pair in rows[..8].as_chunks::<2>().0 {
        let base_roles = role_terms(pair[1].1, false);
        let role = match base_roles.first() {
            Some(ItemAssemblyArmourRole::ArmourBase) => ItemAssemblyDefenceRole::Armour,
            Some(ItemAssemblyArmourRole::EvasionBase) => ItemAssemblyDefenceRole::Evasion,
            Some(ItemAssemblyArmourRole::EnergyShieldBase) => ItemAssemblyDefenceRole::EnergyShield,
            Some(ItemAssemblyArmourRole::WardBase) => ItemAssemblyDefenceRole::Ward,
            _ => return Err(error("armour defence source role")),
        };
        let round_args = args(after(pair[1].1, "round(")?)?;
        if round_args.len() != 1 {
            return Err(error("armour default precision shape"));
        }
        defences.push(ItemAssemblyDefenceRule {
            role,
            base: base_field(pair[0].1, "self.base.armour.")?,
            base_output: pair[0].0.into(),
            output: pair[1].0.into(),
            base_roles,
            increased_roles: role_terms(pair[1].1, true),
        });
    }
    let mut per_level = Vec::new();
    for (output, rhs) in &rows[8..11] {
        let terms = role_terms(rhs, false);
        if terms.len() != 1 {
            return Err(error("armour per-level source role"));
        }
        per_level.push(ItemAssemblyPerLevelRule {
            base_role: terms[0],
            output: (*output).into(),
            increased_roles: role_terms(rhs, true),
        });
    }
    let quality_line = text
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("if calcLocal("))
        .ok_or_else(|| error("armour quality condition"))?;
    let block_args = args(after(rows[11].1, "calcLocal(")?)?;
    let block_tail = after(rows[11].1, &format!("calcLocal({})", block_args.join(", ")))?;
    let movement_args = args(after(text, "modList:NewMod(")?)?;
    if movement_args.len() != 5 {
        return Err(error("armour movement modifier arity"));
    }
    if !movement_args[2].starts_with("-self.base.armour.") || movement_args[3] != "self.modSource" {
        return Err(error("armour movement source/sign"));
    }
    let negated_text = after(movement_args[4], "neg = ")?;
    let negated = if negated_text.starts_with("true") {
        true
    } else if negated_text.starts_with("false") {
        false
    } else {
        return Err(error("armour movement negation"));
    };
    Ok(ItemAssemblyArmourPolicy {
        base_field: field(text, "self.base.")?,
        output_field: field(assignment(text, "local armourData")?, "self.")?,
        queries: array(queries)?,
        alternate_quality: query(lua, quality_line)?,
        quality_threshold: num_after(quality_line, " > ")?,
        suppressed_quality: number(assignment(text, "qualityScalar")?)?,
        defences: array(defences)?,
        per_level: array(per_level)?,
        percent_divisor: num_after(rows[1].1, " / ")?,
        fraction_base: num_after(rows[1].1, " * (")?,
        // The authenticated no-decimal round branch is the integer-round operation.
        round_places: 0,
        block: ItemAssemblyBlockRule {
            base_field: field(rows[11].1, "self.base.armour.")?,
            output: rows[11].0.into(),
            base: query(lua, rows[11].1)?,
            increased: query(lua, block_tail)?,
        },
        movement: ItemAssemblyMovementRule {
            base_field: field(movement_args[2], "self.base.armour.")?,
            modifier_name: string(lua, movement_args[0])?,
            mod_type: string(lua, movement_args[1])?,
            // Represents the authenticated unary minus, not an upstream numeric literal.
            multiplier: -1.0,
            tag_type: text_after(lua, movement_args[4], "type = ")?,
            condition: text_after(lua, movement_args[4], "var = ")?,
            negated,
        },
        overrides: override_policy(lua, text)?,
    })
}
fn duration(lua: &Lua, text: &str, object: &str, base: &str) -> Result<ItemAssemblyDurationPolicy> {
    let rows = assignments(text, object);
    let (output, rhs) = rows
        .iter()
        .find(|(key, _)| *key == "duration")
        .ok_or_else(|| error("local duration output"))?;
    let a = args(after(rhs, "round(")?)?;
    if a.len() != 2 {
        return Err(error("local duration precision"));
    }
    Ok(ItemAssemblyDurationPolicy {
        base_field: field(rhs, &format!("self.base.{base}."))?,
        output: (*output).into(),
        increased: query(lua, assignment(text, "local durationInc")?)?,
        more: query(lua, assignment(text, "local durationMore")?)?,
        round_places: precision(a[1])?,
    })
}
fn charges(lua: &Lua, text: &str, object: &str, base: &str) -> Result<ItemAssemblyChargePolicy> {
    let rows = assignments(text, object);
    let output_key = |key: &str| -> Result<String> {
        rows.iter()
            .find(|(name, _)| *name == key)
            .map(|(name, _)| (*name).into())
            .ok_or_else(|| error("local charge output field"))
    };
    let maximum = assignment(text, &format!("{object}.chargesMax"))?;
    let first = args(after(maximum, "calcLocal(")?)?;
    let remaining = after(maximum, &format!("calcLocal({})", first.join(", ")))?;
    let used = assignment(text, &format!("{object}.chargesUsed"))?;
    let effect = assignment(text, &format!("{object}.effectInc"))?;
    let effect_first = args(after(effect, "calcLocal(")?)?;
    let effect_remaining = after(effect, &format!("calcLocal({})", effect_first.join(", ")))?;
    Ok(ItemAssemblyChargePolicy {
        maximum_base_field: field(maximum, &format!("self.base.{base}."))?,
        maximum_output: output_key("chargesMax")?,
        maximum_base: query(lua, maximum)?,
        maximum_increased: query(lua, remaining)?,
        used_base_field: field(used, &format!("self.base.{base}."))?,
        used_output: output_key("chargesUsed")?,
        used_increased: query(lua, used)?,
        gain_base: query_output(lua, text, object, "gainBase")?,
        gain_increased: query_output(lua, text, object, "gainInc")?,
        gain_multiplier: query_output(lua, text, object, "gainMod")?,
        effect_output: output_key("effectInc")?,
        effect_queries: [query(lua, effect)?, query(lua, effect_remaining)?],
    })
}
fn recovery(lua: &Lua, text: &str) -> Result<ItemAssemblyRecoveryPolicy> {
    let mut channels = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(name) = line
            .strip_prefix("if self.base.flask.")
            .and_then(|s| s.strip_suffix(" then"))
        else {
            continue;
        };
        let role = match name {
            "life" => ItemAssemblyRecoveryRole::Life,
            "mana" => ItemAssemblyRecoveryRole::Mana,
            _ => continue,
        };
        let block = after(text, line)?
            .split_once("\n\t\t\tend")
            .ok_or_else(|| error("recovery channel extent"))?
            .0;
        let rows = assignments(block, "flaskData");
        let count = if role == ItemAssemblyRecoveryRole::Life {
            6
        } else {
            5
        };
        if rows.len() != count {
            return Err(error("recovery channel output inventory"));
        }
        let last = rows.last().ok_or_else(|| error("empty recovery channel"))?;
        let a = args(after(last.1, "calcLocal(")?)?;
        let list = match a.first().copied() {
            Some("baseList") => ItemAssemblyQueryList::Base,
            Some("modList") => ItemAssemblyQueryList::Slot,
            _ => return Err(error("unknown recovery query list")),
        };
        channels.push(ItemAssemblyRecoveryChannel {
            role,
            base_field: field(rows[0].1, "self.base.flask.")?,
            base_output: rows[0].0.into(),
            instant_output: rows[1].0.into(),
            gradual_output: rows[2].0.into(),
            total_output: rows[3].0.into(),
            additional: if rows.len() == 6 {
                Some(ItemAssemblyQueryOutput {
                    output: rows[4].0.into(),
                    query: query(lua, rows[4].1)?,
                })
            } else {
                None
            },
            effect_not_removed: ItemAssemblyScopedQueryOutput {
                output: last.0.into(),
                query: query(lua, last.1)?,
                list,
            },
        });
    }
    Ok(ItemAssemblyRecoveryPolicy {
        instant: query_output(lua, text, "flaskData", "instantPerc")?,
        increased: query(lua, assignment(text, "local recoveryMod")?)?,
        rate: query(lua, assignment(text, "local rateMod")?)?,
        channels: array(channels)?,
    })
}
pub(super) fn extract(
    lua: &Lua,
    slot: &str,
) -> Result<(
    ItemAssemblyArmourPolicy,
    ItemAssemblyFlaskPolicy,
    ItemAssemblyCharmPolicy,
)> {
    let a = branch(slot, "armour", "elseif self.base.flask then")?;
    let f = branch(slot, "flask", "elseif self.base.charm then")?;
    let c = branch(slot, "charm", "elseif self.type ==")?;
    let flask_maximum = assignment(f, "flaskData.chargesMax")?;
    let charm_maximum = assignment(c, "charmData.chargesMax")?;
    Ok((
        armour(lua, a)?,
        ItemAssemblyFlaskPolicy {
            base_field: field(f, "self.base.")?,
            output_field: field(assignment(f, "local flaskData")?, "self.")?,
            duration: duration(lua, f, "flaskData", "flask")?,
            recovery: recovery(lua, f)?,
            charges: charges(lua, f, "flaskData", "flask")?,
            overrides: override_policy(lua, f)?,
            percent_divisor: num_after(flask_maximum, " / ")?,
            fraction_base: num_after(flask_maximum, " * (")?,
        },
        ItemAssemblyCharmPolicy {
            base_field: field(c, "self.base.")?,
            output_field: field(assignment(c, "local charmData")?, "self.")?,
            duration: duration(lua, c, "charmData", "charm")?,
            charges: charges(lua, c, "charmData", "charm")?,
            overrides: override_policy(lua, c)?,
            percent_divisor: num_after(charm_maximum, " / ")?,
            fraction_base: num_after(charm_maximum, " * (")?,
        },
    ))
}
