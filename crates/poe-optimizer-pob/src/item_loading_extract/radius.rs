//! Pinned complete radius source shapes. Policy values are read from those
//! original bodies and their constructed source data; the setter is not run.
//! A changed source body needs review, including a review of changed literals.
use super::*;

type Witness = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);
const WITNESSES: &[Witness] = &[
    (
        "jewel_radius_setter",
        "src/Modules/Data.lua",
        "data.setJewelRadiiGlobally = function(treeVersion)",
        "data.jewelRadii =",
        "d49aa4310c922701e71b3f5bd3d49257a8a9a537d7a12106c5611eadfb99a9e1",
    ),
    (
        "jewel_radius_header",
        "src/Classes/Item.lua",
        "\t\t\t\telseif specName == \"Radius\" and self.type == \"Jewel\" then",
        "\t\t\t\telseif specName == \"Limited to\"",
        "7b6dc9dfb6a1a53a308be6b65fa70499283a6b22720ace2b56a7d9d549dabac8",
    ),
    (
        "jewel_radius_finalization",
        "src/Classes/Item.lua",
        "\tself:BuildModList()\n\tif deferJewelRadiusIndexAssignment then",
        "\nfunction ItemClass:NormaliseQuality()",
        "3d5c4f770d2d79c9c0369fb06b102478e8ad46e44d9d885fb2b9726cdd10d652",
    ),
    (
        "jewel_radius_assembly_reset",
        "src/Classes/Item.lua",
        "function ItemClass:BuildModList()",
        "\tself.baseModList = baseList",
        "dcb02f502fc58bc50cbcfc2b4c2ff29ef8fba96a9857b6b88acf8693926591a6",
    ),
    (
        "jewel_radius_data_startup",
        "src/Modules/Data.lua",
        "data.jewelRadius = data.setJewelRadiiGlobally(latestTreeVersion)",
        "\n-- Stat descriptions",
        "e432a5c65ca1f47c45a56318ced60cebdc81b4895d26c317d0901f0618ba0f47",
    ),
    (
        "jewel_radius_build_startup",
        "src/Modules/Build.lua",
        "\t-- Initialise build components\n\tself.latestTree = main.tree[latestTreeVersion]",
        "\tself.importTab =",
        "4ba76f8f2b05d0eafa0ac25c4b20ed023622a426cc83605568ebfd697e0fc89a",
    ),
    (
        "jewel_radius_versions",
        "src/GameVersions.lua",
        "treeVersionList =",
        "---Tree version where",
        "8d820dfa0b9bd16eb96b7d0d28729eaa25d15209c4481bb6ced7adf58bda6bf9",
    ),
    (
        "jewel_radius_tree_switch",
        "src/Modules/Main.lua",
        "function main:LoadTree(treeVersion)",
        "\nfunction main:CanExit()",
        "ff4785658f6621059f0eefab04030c5de09f6110f448441fdebfdb9c36a3eff6",
    ),
    (
        "jewel_radius_misc_import",
        "src/Modules/Data.lua",
        "local miscData = LoadModule(\"Data/Misc\")",
        "---@class PowerStat",
        "f5eb68c05b35fa0cd95270f614ff923cdabdb4220e237e76fcf5b20c14d26ec1",
    ),
];
fn after<'a>(text: &'a str, prefix: &str) -> Result<&'a str> {
    text.split_once(prefix)
        .map(|(_, rest)| rest)
        .ok_or_else(|| error(format!("jewel radius operand {prefix}")))
}
fn field(text: &str, prefix: &str) -> Result<String> {
    let rest = after(text, prefix)?.trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    if end == 0 || end > 128 {
        return Err(error("jewel radius field bound"));
    }
    Ok(rest[..end].into())
}
fn number(text: &str) -> Result<f64> {
    let text = text.trim_start();
    let end = text
        .find(|c: char| !c.is_ascii_digit() && !matches!(c, '+' | '-' | '.' | 'e' | 'E'))
        .unwrap_or(text.len());
    let value = text[..end].parse::<f64>().map_err(error)?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(error("nonfinite source radius operand"))
    }
}
fn scalar(value: Value) -> Result<f64> {
    match value {
        Value::Integer(v) => Ok(v as f64),
        Value::Number(v) if v.is_finite() => Ok(v),
        _ => Err(error("source radius constant is not a finite number")),
    }
}
pub(super) fn extract(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
) -> Result<(JewelRadiusPolicy, BTreeMap<String, ItemSourceSpan>)> {
    let mut bodies = BTreeMap::new();
    let mut spans = BTreeMap::new();
    for &(name, path, begin, end, expected) in WITNESSES {
        let (body, source_span) = chunk(sources, path, begin, end)?;
        if body.len() > 128 * 1024 || hash(body.as_bytes()) != expected {
            return Err(error(format!(
                "changed complete jewel radius source {name}"
            )));
        }
        bodies.insert(name, body);
        spans.insert(name.into(), source_span);
    }
    let setter = bodies["jewel_radius_setter"];
    let header = bodies["jewel_radius_header"];
    let finalization = bodies["jewel_radius_finalization"];
    let reset = bodies["jewel_radius_assembly_reset"];
    let version_pattern = quoted_after(setter, "treeVersion:match(")?;
    if quoted_after(setter, "version:match(")? != version_pattern {
        return Err(error("radius version patterns disagree"));
    }
    let canonical_separator = quoted_after(setter, "sMajor..")?;
    let initial_maximum = number(after(setter, "local maxJewelRadius = ")?)?;
    let outputs = setter
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("radiusInfo.") && line.contains(" = radiusInfo."))
        .collect::<Vec<_>>();
    if outputs.len() != 2 {
        return Err(error("radius squared field inventory"));
    }
    let outer_squared_field = field(outputs[0], "radiusInfo.")?;
    let inner_squared_field = field(outputs[1], "radiusInfo.")?;
    let outer_field = field(outputs[0], " = radiusInfo.")?;
    let inner_field = field(outputs[1], " = radiusInfo.")?;
    let constant_name = quoted_after(setter, "data.gameConstants[")?;
    let data: Table = lua
        .globals()
        .raw_get("data")
        .map_err(|cause| error(format!("jewel radius source data table: {cause}")))?;
    if data.metatable().is_some() {
        return Err(error("radius source data metatable"));
    }
    let constants: Table = data.raw_get("gameConstants").map_err(|cause| {
        error(format!(
            "jewel radius source data.gameConstants table: {cause}"
        ))
    })?;
    if constants.metatable().is_some() {
        return Err(error("radius constants metatable"));
    }
    let distance_multiplier =
        scalar(constants.raw_get(constant_name.as_str())?).map_err(|cause| {
            error(format!(
                "jewel radius source data.gameConstants[{constant_name}]: {cause}"
            ))
        })?;
    // The original Misc import block in the selected Data chunk constructs this
    // live table. Join its scalar to the unique original source declaration.

    let misc_path = "src/Data/Misc.lua";
    let misc = source(sources, misc_path)?;
    let key = format!("[\"{constant_name}\"] = ");
    let mut values = misc
        .lines()
        .enumerate()
        .filter_map(|(i, line)| line.trim().strip_prefix(&key).map(|rhs| (i, rhs)));
    let (constant_line, rhs) = values
        .next()
        .ok_or_else(|| error("missing radius constant declaration"))?;
    if values.next().is_some() || number(rhs)?.to_bits() != distance_multiplier.to_bits() {
        return Err(error("radius constant declaration/live binding mismatch"));
    }
    spans.insert(
        "jewel_radius_distance_constant".into(),
        span(
            sources,
            misc_path,
            constant_line as u32 + 1,
            constant_line as u32 + 1,
        )?,
    );
    let latest_tree_version: String = lua
        .globals()
        .raw_get("latestTreeVersion")
        .map_err(|cause| error(format!("jewel radius source latestTreeVersion: {cause}")))?;
    let versions: Table = lua.globals().raw_get("treeVersionList").map_err(|cause| {
        error(format!(
            "jewel radius source treeVersionList table: {cause}"
        ))
    })?;
    if versions.metatable().is_some() {
        return Err(error("radius tree version metatable"));
    }
    let count = versions.raw_len();
    if count == 0 || count > 256 {
        return Err(error("radius tree version bound"));
    }
    let declaration = bodies["jewel_radius_versions"]
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("treeVersionList = "))
        .ok_or_else(|| error("radius version declaration"))?;
    // Only the already authenticated literal list is evaluated, in a return
    // expression with no assignments to the constructed host's globals.
    let declared: Table = lua.load(format!("return {declaration}")).eval()?;
    if declared.raw_len() != count {
        return Err(error("radius version declaration count"));
    }
    for index in 1..=count {
        if declared.raw_get::<String>(index)? != versions.raw_get::<String>(index)? {
            return Err(error("radius version declaration/live binding mismatch"));
        }
    }
    let final_version: String = versions.raw_get(count)?;
    if latest_tree_version != final_version {
        return Err(error(
            "latest radius version does not match original final version",
        ));
    }
    let header_name = quoted_after(header, "elseif specName == ")?;
    let jewel_type = quoted_after(header, " and self.type == ")?;
    if quoted_after(reset, "elseif self.type == ")? != jewel_type {
        return Err(error("radius header/assembly reset types disagree"));
    }
    let item_label_field = field(after(header, " then\n")?, "self.")?;
    let item_index_assignment = header
        .lines()
        .find(|line| line.trim().ends_with(" = index"))
        .ok_or_else(|| error("radius header index assignment"))?;
    let item_index_field = field(item_index_assignment, "self.")?;
    let label_pattern = quoted_after(header, "specVal:match(")?;
    let variable_pattern = quoted_after(header, "if specVal:match(")?;
    let variable_label = quoted_after(header, ") == ")?;
    let label_field = field(header, " == data.")?;
    let item_data_field = field(finalization, " = self.")?;
    let deferred_index_field = field(finalization, &format!(" = self.{item_data_field}."))?;
    let override_field = field(
        finalization,
        &format!("if self.{item_data_field} and self.{item_data_field}."),
    )?;
    let reset_tail = after(reset, "elseif self.type == ")?;
    if field(reset_tail, "self.")? != item_data_field {
        return Err(error("radius finalization data field differs from reset"));
    }
    let policy = JewelRadiusPolicy {
        version_pattern,
        canonical_separator,
        latest_tree_version,
        distance_multiplier,
        initial_maximum,
        outer_field,
        inner_field,
        outer_squared_field,
        inner_squared_field,
        label_field,
        header: header_name,
        jewel_type,
        label_pattern,
        variable_pattern,
        variable_label,
        item_label_field,
        item_index_field,
        item_data_field,
        deferred_index_field,
        override_field,
    };
    Ok((policy, spans))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sources() -> BTreeMap<String, String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2");
        let mut sources = BTreeMap::new();
        for path in WITNESSES.iter().map(|w| w.1).chain(["src/Data/Misc.lua"]) {
            sources
                .entry(path.into())
                .or_insert_with(|| crate::source::read_verified_text(&root, path).unwrap());
        }
        sources
    }
    fn host(sources: &BTreeMap<String, String>) -> Lua {
        let lua = Lua::new();
        lua.load(&sources["src/GameVersions.lua"]).exec().unwrap();
        let data = lua.create_table().unwrap();
        lua.globals().raw_set("data", data.clone()).unwrap();
        let misc = sources["src/Data/Misc.lua"].clone();
        lua.globals()
            .raw_set(
                "LoadModule",
                lua.create_function(move |lua, name: String| {
                    if name != "Data/Misc" {
                        return Err(mlua::Error::RuntimeError(
                            "unexpected radius test dependency".into(),
                        ));
                    }
                    lua.load(&misc)
                        .set_name("@src/Data/Misc.lua")
                        .eval::<Value>()
                })
                .unwrap(),
            )
            .unwrap();
        named_eval::<()>(
            &lua,
            sources,
            DATA,
            "local miscData = LoadModule(\"Data/Misc\")",
            "---@class PowerStat",
            "",
        )
        .unwrap();
        assert_eq!(
            lua.globals().raw_get::<Table>("data").unwrap().to_pointer(),
            data.to_pointer()
        );
        lua
    }
    #[test]
    fn original_radius_operands_and_all_source_boundaries_are_retained() {
        let sources = sources();
        let lua = host(&sources);
        let (p, spans) = extract(&lua, &sources).unwrap();
        assert_eq!(spans.len(), 10);
        assert_eq!(p.version_pattern, "(%d+)_(%d+)");
        assert_eq!(p.canonical_separator, "_");
        assert_eq!(p.latest_tree_version, "0_5");
        assert_eq!(p.distance_multiplier, 1.2);
        assert_eq!(p.initial_maximum, 0.0);
        assert_eq!(p.outer_field, "outer");
        assert_eq!(p.inner_field, "inner");
        assert_eq!(p.outer_squared_field, "outerSquared");
        assert_eq!(p.inner_squared_field, "innerSquared");
        assert_eq!(p.label_field, "label");
        assert_eq!(p.header, "Radius");
        assert_eq!(p.jewel_type, "Jewel");
        assert_eq!(p.label_pattern, "^[%a ]+");
        assert_eq!(p.variable_pattern, "^%a+");
        assert_eq!(p.variable_label, "Variable");
        assert_eq!(p.item_label_field, "jewelRadiusLabel");
        assert_eq!(p.item_index_field, "jewelRadiusIndex");
        assert_eq!(p.item_data_field, "jewelData");
        assert_eq!(p.deferred_index_field, "radiusIndex");
        assert_eq!(p.override_field, "timeLostJewelRadiusOverride");
        assert_eq!(spans["jewel_radius_setter"].line, 638);
        assert_eq!(spans["jewel_radius_header"].line, 816);
        assert_eq!(spans["jewel_radius_finalization"].line, 1796);
        assert_eq!(
            spans["jewel_radius_build_startup"].path,
            "src/Modules/Build.lua"
        );
    }
    #[test]
    fn missing_radius_source_tables_have_dependency_context() {
        let sources = sources();
        let lua = host(&sources);
        let data: Table = lua.globals().raw_get("data").unwrap();
        data.raw_set("gameConstants", Value::Nil).unwrap();
        assert!(
            extract(&lua, &sources)
                .unwrap_err()
                .to_string()
                .contains("jewel radius source data.gameConstants table")
        );
        lua.globals().raw_set("data", Value::Nil).unwrap();
        assert!(
            extract(&lua, &sources)
                .unwrap_err()
                .to_string()
                .contains("jewel radius source data table")
        );
        let lua = host(&sources);
        lua.globals()
            .raw_set("treeVersionList", Value::Nil)
            .unwrap();
        assert!(
            extract(&lua, &sources)
                .unwrap_err()
                .to_string()
                .contains("jewel radius source treeVersionList table")
        );
    }
    #[test]
    fn changed_radius_algorithm_is_rejected_before_extracting_policy() {
        let mut sources = sources();
        let lua = host(&sources);
        let data = sources.get_mut(DATA).unwrap();
        assert!(data.contains("jMinor <= tMinor"));
        *data = data.replace("jMinor <= tMinor", "jMinor < tMinor");
        assert!(
            extract(&lua, &sources)
                .unwrap_err()
                .to_string()
                .contains("changed complete jewel radius source")
        );
    }
    #[test]
    fn radius_live_constants_and_latest_version_must_match_source_declarations() {
        let sources = sources();
        let lua = host(&sources);
        let constants: Table = lua
            .globals()
            .raw_get::<Table>("data")
            .unwrap()
            .raw_get("gameConstants")
            .unwrap();
        let original: f64 = constants
            .raw_get("PassiveTreeJewelDistanceMultiplier")
            .unwrap();
        constants
            .raw_set("PassiveTreeJewelDistanceMultiplier", original + 1.0)
            .unwrap();
        assert!(
            extract(&lua, &sources)
                .unwrap_err()
                .to_string()
                .contains("constant declaration/live")
        );
        constants
            .raw_set("PassiveTreeJewelDistanceMultiplier", original)
            .unwrap();
        lua.globals().raw_set("latestTreeVersion", "0_99").unwrap();
        assert!(
            extract(&lua, &sources)
                .unwrap_err()
                .to_string()
                .contains("original final version")
        );
        lua.globals().raw_set("latestTreeVersion", "0_5").unwrap();
        let versions: Table = lua.globals().raw_get("treeVersionList").unwrap();
        versions.raw_set(1, "different").unwrap();
        assert!(
            extract(&lua, &sources)
                .unwrap_err()
                .to_string()
                .contains("version declaration/live")
        );
    }
}
