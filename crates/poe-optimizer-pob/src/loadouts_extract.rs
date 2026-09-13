//! Bounded offline acquisition for the complete fixed loadout lookup methods.
//!
//! Full pinned files authenticate lexical scope; complete method hashes require
//! review when source control flow changes. Only GameVersions construction runs,
//! in a fresh empty environment. No build, lookup, or synchronization method runs.
use crate::{
    game_data::{GameDataExtractionError, error, hash},
    source,
};
use mlua::{Lua, LuaOptions, StdLib, Table, Value};
use poe_optimizer_data::loadouts::{BUILD_LOADOUT_POLICY_SCHEMA_VERSION, BuildLoadoutPolicy};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

type Result<T> = std::result::Result<T, GameDataExtractionError>;
const BUILD: &str = "src/Modules/Build.lua";
const TREE: &str = "src/Classes/TreeTab.lua";
const VERSIONS: &str = "src/GameVersions.lua";
const PATHS: [&str; 3] = [BUILD, TREE, VERSIONS];
const MAX_SOURCE_BYTES: usize = 1024 * 1024;
const MAX_VERSIONS: usize = 256;
const MAX_TEXT_BYTES: usize = 4096;
const VERSION_SOURCE_SHA256: &str =
    "7e672e7aa27d20c2c851d3fd3d509c7fb16148f6bc0ac836466669ca4fde905e";

struct Method {
    first: usize,
    last: usize,
    declaration: &'static str,
    next: &'static str,
    sha256: &'static str,
}
const LOOKUP: Method = Method {
    first: 899,
    last: 954,
    declaration: "function buildMode:GetLoadoutByName(loadoutName)",
    next: "function buildMode:SetActiveLoadout(loadout)",
    sha256: "8d182160fcb55b1bbacdc0deb208e98441442a8500b1e4f816ca8ca6a9241116",
};
const SPEC_LIST: Method = Method {
    first: 484,
    last: 490,
    declaration: "function TreeTabClass:GetSpecList()",
    next: "function TreeTabClass:Load(xml, dbFileName)",
    sha256: "4d401409450457a81884f375fc87cb4d23d52aabd7a3178629a6dba7e05d2da7",
};

/// Acquire only this component from three individually authenticated source files.
/// Full package extraction separately verifies the complete source inventory.
pub fn extract(root: &Path) -> Result<BuildLoadoutPolicy> {
    let sources = PATHS
        .into_iter()
        .map(|path| Ok((path.into(), source::read_verified_text(root, path)?)))
        .collect::<Result<BTreeMap<String, String>>>()?;
    from_sources(&sources)
}

fn authenticated<'a>(sources: &'a BTreeMap<String, String>, path: &str) -> Result<&'a str> {
    let text = sources
        .get(path)
        .ok_or_else(|| error(format!("missing build loadout source {path}")))?;
    if text.len() > MAX_SOURCE_BYTES {
        return Err(error(format!("build loadout source byte limit: {path}")));
    }
    if hash(text.as_bytes()) != source::expected_file_sha256(path)? {
        return Err(error(format!(
            "build loadout full source authentication: {path}"
        )));
    }
    Ok(text)
}
fn method(text: &str, expected: &Method) -> Result<String> {
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    if expected.last + 1 >= lines.len()
        || text.matches(expected.declaration).count() != 1
        || lines[expected.first - 1].trim_end_matches('\n') != expected.declaration
        || lines[expected.last - 1] != "end\n"
        || lines[expected.last] != "\n"
        || lines[expected.last + 1].trim_end_matches('\n') != expected.next
    {
        return Err(error("build loadout complete method boundary"));
    }
    let body = lines[expected.first - 1..expected.last].concat();
    if body.len() > 64 * 1024 || hash(body.as_bytes()) != expected.sha256 {
        return Err(error("changed complete build loadout method"));
    }
    Ok(body)
}
fn binding(text: &str, line: usize, declaration: &str) -> Result<()> {
    if text.lines().nth(line - 1) != Some(declaration)
        || text.lines().filter(|value| *value == declaration).count() != 1
    {
        return Err(error("build loadout lexical primitive binding"));
    }
    Ok(())
}
fn literal(text: &str, prefix: &str, count: usize) -> Result<String> {
    let mut values = text.match_indices(prefix);
    let mut result: Option<String> = None;
    for _ in 0..count {
        let (at, _) = values
            .next()
            .ok_or_else(|| error("missing build loadout operand"))?;
        let tail = &text[at + prefix.len()..];
        let (value, _) = tail
            .split_once('"')
            .ok_or_else(|| error("build loadout literal boundary"))?;
        if value.len() > MAX_TEXT_BYTES || value.contains(['\\', '\n', '\r']) {
            return Err(error("build loadout literal shape/bound"));
        }
        if result.as_deref().is_some_and(|previous| previous != value) {
            return Err(error("build loadout source operands disagree"));
        }
        result = Some(value.to_owned());
    }
    if values.next().is_some() {
        return Err(error("ambiguous build loadout operand"));
    }
    result.ok_or_else(|| error("missing build loadout literal"))
}
fn plain(table: &Table, role: &str) -> Result<()> {
    if table.metatable().is_some() {
        return Err(error(format!("build loadout {role} metatable")));
    }
    Ok(())
}
fn text(value: Value, role: &str) -> Result<String> {
    let Value::String(value) = value else {
        return Err(error(format!("build loadout {role} is not text")));
    };
    if value.as_bytes().len() > MAX_TEXT_BYTES {
        return Err(error(format!("build loadout {role} byte limit")));
    }
    Ok(value.to_str()?.to_owned())
}
fn versions(environment: &Table) -> Result<(String, BTreeMap<String, String>)> {
    plain(environment, "version environment")?;
    let list: Table = environment.raw_get("treeVersionList")?;
    plain(&list, "treeVersionList")?;
    let count = list.raw_len();
    if count == 0 || count > MAX_VERSIONS {
        return Err(error("build loadout version count"));
    }
    let mut visited = 0;
    for entry in list.pairs::<Value, Value>() {
        visited += 1;
        if visited > count {
            return Err(error("build loadout version list is not dense"));
        }
        let (key, value) = entry?;
        let index = match key {
            Value::Integer(value) => value as f64,
            Value::Number(value) => value,
            _ => return Err(error("build loadout version list key")),
        };
        if !index.is_finite() || index.fract() != 0.0 || index < 1.0 || index > count as f64 {
            return Err(error("build loadout version list is not dense"));
        }
        text(value, "version list row")?;
    }
    if visited != count {
        return Err(error("build loadout version list is not dense"));
    }
    let mut names = BTreeSet::new();
    for index in 1..=count {
        if !names.insert(text(list.raw_get(index)?, "version list row")?) {
            return Err(error("build loadout duplicate version"));
        }
    }
    let latest = text(environment.raw_get("latestTreeVersion")?, "latest version")?;
    if latest != text(list.raw_get(count)?, "last version")?
        || text(
            environment.raw_get("defaultTreeVersion")?,
            "default version",
        )? != text(list.raw_get(1)?, "first version")?
    {
        return Err(error("build loadout version list endpoints"));
    }
    let rows: Table = environment.raw_get("treeVersions")?;
    plain(&rows, "treeVersions")?;
    let mut displays = BTreeMap::new();
    for entry in rows.pairs::<Value, Value>() {
        if displays.len() == MAX_VERSIONS {
            return Err(error("build loadout display row limit"));
        }
        let (key, value) = entry?;
        let key = text(key, "display version")?;
        let Value::Table(row) = value else {
            return Err(error("build loadout display row table"));
        };
        plain(&row, "display row")?;
        let display = text(row.raw_get("display")?, "version display")?;
        if !names.contains(&key) || displays.insert(key, display).is_some() {
            return Err(error("build loadout display/list membership"));
        }
    }
    if displays.len() != count {
        return Err(error("build loadout display/list completeness"));
    }
    Ok((latest, displays))
}

pub(crate) fn from_sources(sources: &BTreeMap<String, String>) -> Result<BuildLoadoutPolicy> {
    let build = authenticated(sources, BUILD)?;
    let tree = authenticated(sources, TREE)?;
    let version_source = authenticated(sources, VERSIONS)?;
    let lookup = method(build, &LOOKUP)?;
    let specs = method(tree, &SPEC_LIST)?;
    binding(build, 7, "local ipairs = ipairs")?;
    binding(tree, 6, "local ipairs = ipairs")?;
    binding(tree, 9, "local t_insert = table.insert")?;
    if hash(version_source.as_bytes()) != VERSION_SOURCE_SHA256 {
        return Err(error(
            "changed complete build loadout GameVersions construction",
        ));
    }
    let default_title = literal(&specs, "(spec.title or \"", 1)?;
    if literal(&lookup, "sets[setOrder].title or \"", 1)? != default_title {
        return Err(error("build loadout fallback operands disagree"));
    }
    let single_link_pattern = literal(&lookup, "string.match(value, \"", 2)?;
    let version_prefix = literal(&specs, "and (\"", 1)?;
    let version_suffix = literal(&specs, ".display..\"", 1)?;
    // Authenticated construction is only scalar/table literals and list indexing;
    // the empty environment exposes no callbacks or file/module access. The full
    // fixed-body hash excludes loops; the memory bound covers VM allocations.
    let lua = Lua::new_with(StdLib::NONE, LuaOptions::default())?;
    lua.set_memory_limit(4 * 1024 * 1024)?;
    let environment = lua.create_table()?;
    lua.load(version_source)
        .set_name(format!("@{VERSIONS}"))
        .set_environment(environment.clone())
        .exec()?;
    let (latest_tree_version, tree_version_display) = versions(&environment)?;
    let policy = BuildLoadoutPolicy {
        schema_version: BUILD_LOADOUT_POLICY_SCHEMA_VERSION,
        default_title,
        latest_tree_version,
        tree_version_display,
        version_prefix,
        version_suffix,
        single_link_pattern,
    };
    policy.validate().map_err(error)?;
    Ok(policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sources() -> BTreeMap<String, String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
        PATHS
            .into_iter()
            .map(|path| {
                (
                    path.into(),
                    source::read_verified_text(&root, path).unwrap(),
                )
            })
            .collect()
    }
    #[test]
    fn complete_original_lookup_policy_and_all_version_rows_are_acquired() {
        let sources = sources();
        let policy = from_sources(&sources).unwrap();
        assert_eq!(policy.default_title, "Default");
        assert_eq!(policy.latest_tree_version, "0_5");
        assert_eq!(policy.version_prefix, "[");
        assert_eq!(policy.version_suffix, "] ");
        assert_eq!(policy.single_link_pattern, "%{(%w+)%}");
        assert_eq!(
            policy.tree_version_display,
            (1..=5)
                .map(|n| (format!("0_{n}"), format!("0_{n}")))
                .collect()
        );
        assert_eq!(
            method(&sources[BUILD], &LOOKUP).unwrap().lines().count(),
            56
        );
        assert_eq!(
            method(&sources[TREE], &SPEC_LIST).unwrap().lines().count(),
            7
        );
    }
    #[test]
    fn missing_and_mutated_authenticated_sources_are_rejected() {
        let original = sources();
        for path in PATHS {
            let mut changed = original.clone();
            changed.remove(path);
            assert!(
                from_sources(&changed)
                    .unwrap_err()
                    .to_string()
                    .contains("missing build loadout source")
            );
        }
        for (path, before, after) in [
            (BUILD, "local ipairs = ipairs", "local ipairs = pairs"),
            (
                TREE,
                "local t_insert = table.insert",
                "local t_insert = table.remove",
            ),
            (
                BUILD,
                "local oneItem = self.itemsTab and #self.itemsTab.itemSetOrderList == 1",
                "local oneItem = true",
            ),
            (TREE, "return newSpecList", "return self.specList"),
            (
                VERSIONS,
                "latestTreeVersion = treeVersionList[#treeVersionList]",
                "latestTreeVersion = treeVersionList[1]",
            ),
            (VERSIONS, "display = \"0_4\"", "display = \"changed\""),
        ] {
            let mut changed = original.clone();
            let text = changed.get_mut(path).unwrap();
            assert!(text.contains(before));
            *text = text.replacen(before, after, 1);
            assert!(
                from_sources(&changed)
                    .unwrap_err()
                    .to_string()
                    .contains("full source authentication")
            );
        }
    }
    #[test]
    fn complete_method_and_lexical_shape_guards_reject_drift() {
        let sources = sources();
        for (path, expected, before, after) in [
            (
                BUILD,
                &LOOKUP,
                "local oneConfig = self.configTab and #self.configTab.configSetOrderList == 1",
                "local oneConfig = false",
            ),
            (
                TREE,
                &SPEC_LIST,
                "return newSpecList",
                "return self.specList",
            ),
        ] {
            let original_body = method(&sources[path], expected).unwrap();
            assert_eq!(original_body.matches(before).count(), 1);
            let changed_body = original_body.replacen(before, after, 1);
            assert_ne!(changed_body, original_body);
            assert_eq!(sources[path].matches(original_body.as_str()).count(), 1);
            let changed = sources[path].replacen(&original_body, &changed_body, 1);
            assert!(
                method(&changed, expected)
                    .unwrap_err()
                    .to_string()
                    .contains("changed complete")
            );
        }
        assert!(
            binding(
                &sources[BUILD].replacen("local ipairs = ipairs", "local ipairs = pairs", 1),
                7,
                "local ipairs = ipairs"
            )
            .is_err()
        );
        assert!(
            binding(
                &sources[TREE].replacen(
                    "local t_insert = table.insert",
                    "local t_insert = table.remove",
                    1
                ),
                9,
                "local t_insert = table.insert"
            )
            .is_err()
        );
    }
    #[test]
    fn constructed_version_projection_rejects_holes_extras_and_inheritance() {
        let sources = sources();
        let lua = Lua::new();
        for mutation in [
            "treeVersionList[2]=nil",
            "treeVersionList.extra='0_1'",
            "treeVersionList[2]='0_1'",
            "treeVersions['0_4']=nil",
            "treeVersions.extra={display='extra'}",
            "latestTreeVersion='0_1'",
            "setmetatable(treeVersions['0_1'], {})",
            "treeVersions['0_1'].display=17",
        ] {
            let environment = lua.create_table().unwrap();
            environment
                .raw_set(
                    "setmetatable",
                    lua.globals().raw_get::<Value>("setmetatable").unwrap(),
                )
                .unwrap();
            lua.load(&sources[VERSIONS])
                .set_environment(environment.clone())
                .exec()
                .unwrap();
            lua.load(mutation)
                .set_environment(environment.clone())
                .exec()
                .unwrap();
            assert!(versions(&environment).is_err(), "{mutation}");
        }
    }
}
