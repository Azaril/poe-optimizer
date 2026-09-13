//! Unchanged original method slices with explicitly supplied, plain component state.
//! No original import, SyncLoadouts, callback traversal, or production context is claimed.
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::loadouts::{BUILD_LOADOUT_POLICY_SCHEMA_VERSION, BuildLoadoutPolicy};
use poe_optimizer_import::{
    loadouts::{
        LiveLoadoutContext, LoadoutError, LoadoutErrorKind, LoadoutLink, LoadoutSpec, Result,
    },
    selected_view::{NumericValue, SelectionDomain},
};
use poe_optimizer_pob::source;
use serde::Serialize;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::BTreeMap, path::PathBuf, sync::OnceLock};

pub const BUILD: &str = "src/Modules/Build.lua";
pub const TREE: &str = "src/Classes/TreeTab.lua";
const VERSIONS: &str = "src/GameVersions.lua";
const MAX_ROWS: usize = 4096;
const MAX_BYTES: usize = 256 * 1024;

fn original(path: &str) -> &'static str {
    static TEXT: OnceLock<BTreeMap<&str, String>> = OnceLock::new();
    &TEXT.get_or_init(|| {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
        [BUILD, TREE, VERSIONS]
            .into_iter()
            .map(|path| (path, source::read_verified_text(&root, path).unwrap()))
            .collect()
    })[path]
}
fn body(path: &str, first: usize, last: usize, declaration: &str, next: &str) -> String {
    let lines: Vec<_> = original(path).lines().collect();
    assert_eq!(lines[first - 1], declaration);
    assert_eq!(lines[last - 1], "end");
    assert_eq!(lines[last], "");
    assert_eq!(lines[last + 1], next);
    assert_eq!(original(path).matches(declaration).count(), 1);
    format!("{}\n", lines[first - 1..last].join("\n"))
}
fn original_bodies() -> (String, String) {
    (
        body(
            BUILD,
            899,
            954,
            "function buildMode:GetLoadoutByName(loadoutName)",
            "function buildMode:SetActiveLoadout(loadout)",
        ),
        body(
            TREE,
            484,
            490,
            "function TreeTabClass:GetSpecList()",
            "function TreeTabClass:Load(xml, dbFileName)",
        ),
    )
}
fn quoted_after<'a>(text: &'a str, prefix: &str) -> &'a str {
    let tail = text.split_once(prefix).expect("original operand prefix").1;
    tail.split_once('"')
        .expect("original operand closing quote")
        .0
}
fn install(lua: &Lua, path: &str, first: usize, body: &str, prelude: &str) {
    assert!(!prelude.contains('\n'));
    let chunk = format!("{prelude}\n{}{body}", "\n".repeat(first - 2));
    lua.load(chunk).set_name(format!("@{path}")).exec().unwrap();
}
fn check_function(function: &Function, path: &str, first: usize, last: usize) {
    let info = function.info();
    assert_eq!(info.what, "Lua");
    assert_eq!(info.source.as_deref(), Some(format!("@{path}").as_str()));
    assert_eq!(info.line_defined, Some(first));
    assert_eq!(info.last_line_defined, Some(last));
}

#[derive(Clone, Debug, Serialize)]
pub struct Spec {
    pub title: Option<String>,
    pub version: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Domain {
    pub absent: bool,
    pub order: Vec<Option<f64>>,
    pub rows: Vec<(f64, Option<String>)>,
    pub links: Option<Vec<(String, Option<f64>)>>,
    /// Explicit source #order == 1 assertion, checked against the constructed Lua table.
    /// Sparse cases never infer this from their ipairs prefix.
    pub singleton: bool,
}
impl Domain {
    pub fn named(rows: &[(f64, Option<&str>)]) -> Self {
        Self {
            absent: false,
            order: rows.iter().map(|(id, _)| Some(*id)).collect(),
            rows: rows
                .iter()
                .map(|(id, name)| (*id, name.map(str::to_owned)))
                .collect(),
            links: Some(vec![]),
            singleton: rows.len() == 1,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Case {
    pub name: String,
    pub query: String,
    pub specs: Vec<Option<Spec>>,
    pub tree_links: Option<Vec<(String, Option<f64>)>>,
    pub items: Domain,
    pub skills: Domain,
    pub configuration: Domain,
}
impl Case {
    pub fn empty(name: &str, query: &str) -> Self {
        Self {
            name: name.into(),
            query: query.into(),
            specs: vec![],
            tree_links: Some(vec![]),
            items: Domain::default(),
            skills: Domain::default(),
            configuration: Domain::default(),
        }
    }
    pub fn spec(&mut self, title: Option<&str>, version: Option<&str>) {
        self.specs.push(Some(Spec {
            title: title.map(str::to_owned),
            version: version.map(str::to_owned),
        }));
    }
}
pub struct Context<'a> {
    pub case: &'a Case,
    pub reads: RefCell<Vec<String>>,
}
impl<'a> Context<'a> {
    pub fn new(case: &'a Case) -> Self {
        Self {
            case,
            reads: RefCell::new(vec![]),
        }
    }
    fn record(&self, text: String) {
        let mut reads = self.reads.borrow_mut();
        assert!(reads.len() < MAX_ROWS);
        reads.push(text);
    }
    fn domain(&self, domain: SelectionDomain) -> &Domain {
        match domain {
            SelectionDomain::Items => &self.case.items,
            SelectionDomain::Skills => &self.case.skills,
            SelectionDomain::Configuration => &self.case.configuration,
            SelectionDomain::Passives => panic!("set access for tree"),
        }
    }
}
fn source_failure(message: &'static str) -> LoadoutError {
    LoadoutError::new(LoadoutErrorKind::Source, message)
}
impl LiveLoadoutContext for Context<'_> {
    fn is_singleton(&self, domain: SelectionDomain) -> Result<bool> {
        self.record(format!("singleton:{domain:?}"));
        Ok(!self.domain(domain).absent && self.domain(domain).singleton)
    }
    fn spec_prefix_len(&self) -> Result<usize> {
        self.record("spec_prefix".into());
        Ok(self
            .case
            .specs
            .iter()
            .take_while(|row| row.is_some())
            .count())
    }
    fn spec(&self, position: usize) -> Result<LoadoutSpec<'_>> {
        self.record(format!("spec:{position}"));
        let row = self
            .case
            .specs
            .get(position - 1)
            .and_then(Option::as_ref)
            .ok_or_else(|| source_failure("missing spec"))?;
        Ok(LoadoutSpec {
            title: row.title.as_deref(),
            tree_version: row.version.as_deref(),
        })
    }
    fn ordered_set(
        &self,
        domain: SelectionDomain,
        position: usize,
    ) -> Result<Option<NumericValue>> {
        self.record(format!("order:{domain:?}:{position}"));
        let state = self.domain(domain);
        if state.absent {
            return Err(source_failure("missing domain"));
        }
        Ok(state
            .order
            .get(position - 1)
            .copied()
            .flatten()
            .map(NumericValue::new))
    }
    fn set_title(&self, domain: SelectionDomain, key: NumericValue) -> Result<Option<&str>> {
        self.record(format!("title:{domain:?}:{:016x}", key.value().to_bits()));
        self.domain(domain)
            .rows
            .iter()
            .rev()
            .find(|(id, _)| *id == key.value())
            .map(|(_, title)| title.as_deref())
            .ok_or_else(|| source_failure("missing set row"))
    }
    fn linked_set(
        &self,
        domain: SelectionDomain,
        link: LoadoutLink<'_>,
    ) -> Result<Option<NumericValue>> {
        self.record(format!("link:{domain:?}"));
        let LoadoutLink::Bytes(bytes) = link else {
            panic!("unchanged original pattern has a byte capture")
        };
        let links = if domain == SelectionDomain::Passives {
            &self.case.tree_links
        } else {
            &self.domain(domain).links
        };
        links
            .as_ref()
            .and_then(|rows| rows.iter().rev().find(|(key, _)| key.as_bytes() == bytes))
            .map(|(_, id)| id.map(NumericValue::new))
            .ok_or_else(|| source_failure("missing link row"))
    }
}

pub struct Original {
    pub lua: Lua,
    pub lookup: Function,
    pub spec_list: Function,
    pub policy: BuildLoadoutPolicy,
    build_class: Table,
    tree_class: Table,
    primitives: Vec<(&'static str, Option<&'static str>, Function)>,
}
impl Original {
    pub fn new() -> Self {
        let lua = Lua::new();
        lua.set_memory_limit(32 * 1024 * 1024).unwrap();
        let primitives = [
            ("ipairs", None),
            ("string", Some("match")),
            ("table", Some("insert")),
        ]
        .into_iter()
        .map(|(owner, key)| {
            let function = if let Some(key) = key {
                lua.globals()
                    .raw_get::<Table>(owner)
                    .unwrap()
                    .raw_get(key)
                    .unwrap()
            } else {
                lua.globals().raw_get(owner).unwrap()
            };
            (owner, key, function)
        })
        .collect::<Vec<_>>();
        lua.load(original(VERSIONS))
            .set_name(format!("@{VERSIONS}"))
            .exec()
            .unwrap();
        let latest: String = lua.globals().raw_get("latestTreeVersion").unwrap();
        let versions: Table = lua.globals().raw_get("treeVersions").unwrap();
        let mut displays = BTreeMap::new();
        for entry in versions.pairs::<String, Table>() {
            let (key, row) = entry.unwrap();
            assert!(displays.len() < 64);
            displays.insert(key, row.raw_get::<String>("display").unwrap());
        }
        let (lookup_body, spec_body) = original_bodies();
        let fallback = quoted_after(&spec_body, "(spec.title or \"").to_owned();
        assert_eq!(
            quoted_after(&lookup_body, "sets[setOrder].title or \""),
            fallback
        );
        let pattern = quoted_after(&lookup_body, "string.match(value, \"").to_owned();
        assert_eq!(
            lookup_body
                .matches(&format!("string.match(value, \"{pattern}\")"))
                .count(),
            2
        );
        let prefix = quoted_after(&spec_body, "and (\"").to_owned();
        let suffix = quoted_after(&spec_body, ".display..\"").to_owned();
        let policy = BuildLoadoutPolicy {
            schema_version: BUILD_LOADOUT_POLICY_SCHEMA_VERSION,
            default_title: fallback,
            latest_tree_version: latest,
            tree_version_display: displays,
            version_prefix: prefix,
            version_suffix: suffix,
            single_link_pattern: pattern,
        };
        // Lexical aliases use the exact original declarations and original C primitives.
        assert_eq!(
            original(BUILD).lines().nth(6),
            Some("local ipairs = ipairs")
        );
        assert_eq!(original(TREE).lines().nth(5), Some("local ipairs = ipairs"));
        assert_eq!(
            original(TREE).lines().nth(8),
            Some("local t_insert = table.insert")
        );
        let build_class = lua.create_table().unwrap();
        let tree_class = lua.create_table().unwrap();
        lua.globals()
            .raw_set("buildMode", build_class.clone())
            .unwrap();
        lua.globals()
            .raw_set("TreeTabClass", tree_class.clone())
            .unwrap();
        install(
            &lua,
            BUILD,
            899,
            &lookup_body,
            "local ipairs = ipairs; local buildMode = buildMode",
        );
        install(
            &lua,
            TREE,
            484,
            &spec_body,
            "local ipairs = ipairs; local t_insert = table.insert; local TreeTabClass = TreeTabClass",
        );
        let lookup = build_class.raw_get("GetLoadoutByName").unwrap();
        let spec_list = tree_class.raw_get("GetSpecList").unwrap();
        let out = Self {
            lua,
            lookup,
            spec_list,
            policy,
            build_class,
            tree_class,
            primitives,
        };
        out.verify();
        out
    }
    pub fn verify(&self) {
        check_function(&self.lookup, BUILD, 899, 954);
        check_function(&self.spec_list, TREE, 484, 490);
        assert_eq!(
            self.build_class
                .raw_get::<Function>("GetLoadoutByName")
                .unwrap(),
            self.lookup
        );
        assert_eq!(
            self.tree_class.raw_get::<Function>("GetSpecList").unwrap(),
            self.spec_list
        );
        assert_eq!(self.lookup.environment().unwrap(), self.lua.globals());
        assert_eq!(self.spec_list.environment().unwrap(), self.lua.globals());
        for (owner, key, expected) in &self.primitives {
            let actual: Function = if let Some(key) = key {
                self.lua
                    .globals()
                    .raw_get::<Table>(*owner)
                    .unwrap()
                    .raw_get(*key)
                    .unwrap()
            } else {
                self.lua.globals().raw_get(*owner).unwrap()
            };
            assert_eq!(&actual, expected);
            assert_eq!(actual.info().what, "C");
        }
    }
    /// Only data globals change. Both retained complete source bodies stay unchanged.
    pub fn set_versions(&mut self, latest: &str, displays: BTreeMap<String, String>) {
        self.policy.latest_tree_version = latest.into();
        self.policy.tree_version_display = displays;
        self.lua
            .globals()
            .raw_set("latestTreeVersion", latest)
            .unwrap();
        let rows = self.lua.create_table().unwrap();
        for (key, label) in &self.policy.tree_version_display {
            let row = self.lua.create_table().unwrap();
            row.raw_set("display", label.as_str()).unwrap();
            rows.raw_set(key.as_str(), row).unwrap();
        }
        self.lua.globals().raw_set("treeVersions", rows).unwrap();
        self.verify();
    }
    pub fn state(&self, case: &Case) -> (Table, Table) {
        assert!(case.specs.len() < 128 && case.query.len() < 4096);
        let build = self.lua.create_table().unwrap();
        let tree = self.lua.create_table().unwrap();
        tree.raw_set("GetSpecList", self.spec_list.clone()).unwrap();
        let specs = self.lua.create_table().unwrap();
        for (index, row) in case.specs.iter().enumerate() {
            if let Some(row) = row {
                let spec = self.lua.create_table().unwrap();
                spec.raw_set("title", row.title.as_deref()).unwrap();
                spec.raw_set("treeVersion", row.version.as_deref()).unwrap();
                specs.raw_set(index + 1, spec).unwrap();
            }
        }
        tree.raw_set("specList", specs.clone()).unwrap();
        build.raw_set("treeTab", tree).unwrap();
        let read_set = self.lua.create_table().unwrap();
        read_set.raw_set("specList", specs).unwrap();
        read_set
            .raw_set(
                "latestTreeVersion",
                self.lua
                    .globals()
                    .raw_get::<Value>("latestTreeVersion")
                    .unwrap(),
            )
            .unwrap();
        read_set
            .raw_set(
                "treeVersions",
                self.lua.globals().raw_get::<Table>("treeVersions").unwrap(),
            )
            .unwrap();
        let links = self.links(&case.tree_links);
        build
            .raw_set("treeListSpecialLinks", links.clone())
            .unwrap();
        read_set.raw_set("treeListSpecialLinks", links).unwrap();
        for (domain, tab, order_key, sets_key, link_key) in [
            (
                &case.items,
                "itemsTab",
                "itemSetOrderList",
                "itemSets",
                "itemListSpecialLinks",
            ),
            (
                &case.skills,
                "skillsTab",
                "skillSetOrderList",
                "skillSets",
                "skillListSpecialLinks",
            ),
            (
                &case.configuration,
                "configTab",
                "configSetOrderList",
                "configSets",
                "configListSpecialLinks",
            ),
        ] {
            let links = self.links(&domain.links);
            build.raw_set(link_key, links.clone()).unwrap();
            read_set.raw_set(link_key, links).unwrap();
            if domain.absent {
                continue;
            }
            assert!(domain.rows.len() < 128 && domain.order.len() < 128);
            let owner = self.lua.create_table().unwrap();
            let order = self.lua.create_table().unwrap();
            let sets = self.lua.create_table().unwrap();
            for (index, id) in domain.order.iter().enumerate() {
                order.raw_set(index + 1, *id).unwrap();
            }
            assert_eq!(
                order.raw_len() == 1,
                domain.singleton,
                "explicit singleton proof: {} {tab}",
                case.name
            );
            for (id, title) in &domain.rows {
                let row = self.lua.create_table().unwrap();
                row.raw_set("title", title.as_deref()).unwrap();
                sets.raw_set(*id, row).unwrap();
            }
            owner.raw_set(order_key, order).unwrap();
            owner.raw_set(sets_key, sets).unwrap();
            build.raw_set(tab, owner.clone()).unwrap();
            read_set.raw_set(tab, owner).unwrap();
        }
        (build, read_set)
    }
    fn links(&self, input: &Option<Vec<(String, Option<f64>)>>) -> Value {
        let Some(input) = input else {
            return Value::Nil;
        };
        assert!(input.len() < 128);
        let table = self.lua.create_table().unwrap();
        for (key, id) in input {
            let row = self.lua.create_table().unwrap();
            row.raw_set("setId", *id).unwrap();
            table.raw_set(key.as_str(), row).unwrap();
        }
        Value::Table(table)
    }
    pub fn evidence(&self) -> Json {
        json!({"pin":source::UPSTREAM_REVISION,"manifest_sha256":source::manifest_sha256(),
            "files":([BUILD,TREE,VERSIONS].map(|path|json!({"path":path,"sha256":source::expected_file_sha256(path).unwrap()}))),
            "lookup_lines":[899,954],"spec_list_lines":[484,490],"policy":self.policy,
            "scope":"unchanged complete original method slices; supplied plain component state and original standard primitives; no imported closure, SyncLoadouts, activation, or production context claim"})
    }
}

/// The pure source component has no Rust callbacks. Host/resource/conversion
/// failures cannot be matched as Source merely because both calls returned Err.
pub fn is_source_error(error: &mlua::Error) -> bool {
    let mlua::Error::RuntimeError(text) = error else {
        return false;
    };
    (text.contains("src/Modules/Build.lua:") || text.contains("src/Classes/TreeTab.lua:"))
        && ![
            "stack overflow",
            "not enough memory",
            "memory allocation",
            "deadline",
            "instruction limit",
            "harness:",
        ]
        .iter()
        .any(|part| text.contains(part))
}

#[derive(Debug, PartialEq)]
pub struct Snapshot {
    graph: Json,
    identities: Vec<usize>,
}
impl Snapshot {
    pub fn digest(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&self.graph).unwrap())
        )
    }
}
/// Bounded canonical graph of all supplied fields read by these two bodies.
/// Function identity is verified separately; functions are rejected here.
pub fn snapshot(root: Value) -> Snapshot {
    fn value(
        input: Value,
        tables: &mut Vec<(usize, Json)>,
        rows: &mut usize,
        bytes: &mut usize,
        depth: usize,
    ) -> Json {
        *rows += 1;
        assert!(
            *rows <= MAX_ROWS && depth <= 16,
            "component graph row/depth bound"
        );
        match input {
            Value::Nil => json!(["nil"]),
            Value::Boolean(v) => json!(["bool", v]),
            Value::Integer(v) => json!(["number", format!("{:016x}", (v as f64).to_bits())]),
            Value::Number(v) => json!(["number", format!("{:016x}", v.to_bits())]),
            Value::String(v) => {
                *bytes += v.as_bytes().len();
                assert!(*bytes <= MAX_BYTES);
                json!(["bytes", v.as_bytes().as_ref()])
            }
            Value::Table(t) => {
                assert!(t.metatable().is_none(), "unrepresented component metatable");
                let pointer = t.to_pointer() as usize;
                if let Some(id) = tables.iter().position(|(p, _)| *p == pointer) {
                    return json!(["table", id]);
                }
                assert!(tables.len() < 512);
                let id = tables.len();
                tables.push((pointer, Json::Null));
                let mut entries = Vec::new();
                for entry in t.pairs::<Value, Value>() {
                    assert!(entries.len() < MAX_ROWS);
                    let (key, v) = entry.unwrap();
                    assert!(
                        matches!(key, Value::String(_) | Value::Integer(_) | Value::Number(_)),
                        "unrepresented key"
                    );
                    let key = value(key, tables, rows, bytes, depth + 1);
                    entries.push((key, v));
                }
                entries.sort_by_key(|(key, _)| serde_json::to_string(key).unwrap());
                let entries: Vec<_> = entries
                    .into_iter()
                    .map(|(key, v)| json!([key, value(v, tables, rows, bytes, depth + 1)]))
                    .collect();
                tables[id].1 = json!(entries);
                json!(["table", id])
            }
            other => panic!("unrepresented component value {}", other.type_name()),
        }
    }
    let mut tables = vec![];
    let root = value(root, &mut tables, &mut 0, &mut 0, 0);
    Snapshot {
        graph: json!({"root":root,"tables":tables.iter().map(|(_,v)|v).collect::<Vec<_>>()}),
        identities: tables.iter().map(|(p, _)| *p).collect(),
    }
}
pub fn ids(pack: &MultiValue) -> Option<BTreeMap<String, u64>> {
    assert_eq!(pack.len(), 1, "complete original return arity");
    match pack.front().unwrap() {
        Value::Nil => None,
        Value::Table(table) => {
            assert!(table.metatable().is_none());
            let mut out = BTreeMap::new();
            for entry in table.pairs::<String, Value>() {
                let (key, v) = entry.unwrap();
                assert!(
                    ["specId", "itemSetId", "skillSetId", "configSetId"].contains(&key.as_str())
                );
                let number = match v {
                    Value::Integer(v) => v as f64,
                    Value::Number(v) => v,
                    other => panic!("unexpected result {}", other.type_name()),
                };
                assert!(out.insert(key, number.to_bits()).is_none());
                assert!(out.len() <= 4);
            }
            Some(out)
        }
        other => panic!("unexpected source result {}", other.type_name()),
    }
}
