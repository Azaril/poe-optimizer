//! Bounded source construction fixtures; complete startup parity lives in pob/tests.
use super::*;
use std::path::PathBuf;
const COMMON: &str = "src/Modules/Common.lua";
const FIXTURE: &str = "tests/class_capture.lua";
const FIXTURE_SOURCE: &str = r#"local Base = newClass("Base")
function Base:Base() self.base = true return self end
function Base:Record(value) self.value = value return self end
function Base:Unselected() return 9 end
local Derived = newClass("Derived", "Base")
function Derived:Derived() self:Base() return self end
function Derived:Owner() return Derived end
Data = {maximum = 83}
local function evaluate(value) return math.min(Data.maximum, value) end
return {evaluate = evaluate}
"#;
struct Fixture {
    lua: Lua,
    observer: SourceClosureObserver,
    sources: BTreeMap<String, String>,
    metadata: ItemLoadingSource,
    request: SourceClassCaptureRequest,
}
fn fixture(edit: impl FnOnce(String) -> String, source_name: &str) -> Fixture {
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let common = edit(crate::source::read_verified_text(&root, COMMON).unwrap());
    let start = common.find("common.classes = { }").unwrap();
    let end = common.find("\nfunction codePointToUTF8").unwrap();
    let prefix = common[..start]
        .split_inclusive('\n')
        .map(|line| {
            if line.starts_with("local pairs =")
                || line.starts_with("local ipairs =")
                || line.starts_with("local s_format =")
                || line.starts_with("common =")
            {
                line.to_string()
            } else {
                "\n".into()
            }
        })
        .collect::<String>();
    lua.load(format!("{prefix}{}", &common[start..end]))
        .set_name(format!("@{source_name}"))
        .exec()
        .unwrap();
    let roots: Table = lua
        .load(FIXTURE_SOURCE)
        .set_name(format!("@{FIXTURE}"))
        .eval()
        .unwrap();
    // Warm after Derived copied Base's original constructor, exercising the
    // source's distinction between a copied body and later wrapped parent entry.
    lua.load("new('Derived'):Derived(); new('Base'):Base()")
        .exec()
        .unwrap();
    let registry: Table = lua
        .globals()
        .get::<Table>("common")
        .unwrap()
        .get("classes")
        .unwrap();
    let sources: BTreeMap<String, String> = [
        (COMMON.into(), common),
        (FIXTURE.into(), FIXTURE_SOURCE.into()),
    ]
    .into();
    let metadata = ItemLoadingSource {
        upstream_revision: "a".repeat(40),
        files: sources
            .iter()
            .map(|(path, text)| (path.clone(), hash(text.as_bytes())))
            .collect(),
        construction_spans: BTreeMap::new(),
        module_order: vec![COMMON.into(), FIXTURE.into()],
    };
    let request = SourceClassCaptureRequest {
        classes: vec![
            SourceClassSelection {
                table: registry.get("Derived").unwrap(),
                methods: ["Base".into(), "Record".into(), "Owner".into()].into(),
            },
            SourceClassSelection {
                table: registry.get("Base").unwrap(),
                methods: BTreeSet::new(),
            },
        ],
        callbacks: [("evaluate".into(), roots.get("evaluate").unwrap())].into(),
        definition_roots: [("Data".into(), lua.globals().get("Data").unwrap())].into(),
        allocation: lua.globals().get("new").unwrap(),
        source_names: if source_name == COMMON {
            BTreeMap::new()
        } else {
            [(source_name.into(), COMMON.into())].into()
        },
    };
    Fixture {
        lua,
        observer,
        sources,
        metadata,
        request,
    }
}
#[test]
fn captures_requested_classes_wrappers_originals_aliases_cache_and_named_roots() {
    let fixture = fixture(|text| text, COMMON);
    let before: Value = fixture
        .lua
        .load("return common.classes.Derived._constructorInitialised")
        .eval()
        .unwrap();
    let observed = fixture
        .observer
        .observe_classes(
            &fixture.lua,
            &fixture.sources,
            fixture.metadata,
            fixture.request,
        )
        .unwrap();
    let owner = observed.owner();
    let classes = owner.classes().unwrap();
    assert_eq!(classes.classes.len(), 2);
    let child = owner.class(observed.class_ids()["Derived"]).unwrap();
    let base_id = observed.class_ids()["Base"];
    assert_eq!(child.parents, vec![base_id]);
    assert_eq!(child.super_parents, Some(vec![base_id]));
    assert!(child.unsupported_fields.contains("Unselected"));
    assert!(child.unsupported_fields.contains("__call"));
    assert_eq!(child.methods["Record"].declared_by, base_id);
    assert_eq!(child.methods["Base"].declared_by, base_id);
    assert!(child.constructor.as_ref().unwrap().wrapper.is_some());
    let callback = &owner.callbacks()[observed.callbacks()["Derived.Owner"].0 as usize - 1];
    assert_eq!(callback.upvalues[0].value, SourceValue::Table(child.table));
    assert!(owner.root_id("Data").is_some());
    assert_eq!(
        format!("{before:?}"),
        format!(
            "{:?}",
            fixture
                .lua
                .load("return common.classes.Derived._constructorInitialised")
                .eval::<Value>()
                .unwrap()
        )
    );
    let lowered = crate::source_programs::lower_from_sources(&fixture.sources, owner).unwrap();
    assert!(
        lowered
            .catalog()
            .data()
            .callbacks
            .contains_key(&observed.callbacks()["evaluate"])
    );
    assert!(
        lowered
            .catalog()
            .data()
            .callbacks
            .contains_key(&observed.callbacks()["Derived.Record"])
    );
    // Captured protocol-only builtins are real observed values and remain opaque
    // to generic program dispatch until that language operation is implemented.
    assert!(
        lowered
            .unsupported()
            .values()
            .any(|reason| reason.contains("builtin"))
    );
}
#[test]
fn source_names_are_explicit_and_protocol_field_names_are_injected() {
    let mut fixture = fixture(
        |text| text.replace("_parentInit", "_initializedParents"),
        "runtime/actual-common.lua",
    );
    let observed = fixture
        .observer
        .observe_classes(
            &fixture.lua,
            &fixture.sources,
            fixture.metadata.clone(),
            fixture.request.clone(),
        )
        .unwrap();
    assert_eq!(
        observed.owner().classes().unwrap().source.parent_init,
        "_initializedParents"
    );
    fixture.request.source_names.clear();
    assert!(
        fixture
            .observer
            .observe_classes(
                &fixture.lua,
                &fixture.sources,
                fixture.metadata,
                fixture.request
            )
            .unwrap_err()
            .to_string()
            .contains("source name")
    );
}
#[test]
fn omitted_parent_requested_missing_method_changed_protocol_and_proxy_captures_fail() {
    let mut missing = fixture(|text| text, COMMON);
    missing.request.classes.pop();
    assert!(
        missing
            .observer
            .observe_classes(
                &missing.lua,
                &missing.sources,
                missing.metadata,
                missing.request
            )
            .unwrap_err()
            .to_string()
            .contains("explicitly requested")
    );
    let mut missing = fixture(|text| text, COMMON);
    missing.request.classes[0]
        .methods
        .insert("NotPresent".into());
    assert!(
        missing
            .observer
            .observe_classes(
                &missing.lua,
                &missing.sources,
                missing.metadata,
                missing.request
            )
            .unwrap_err()
            .to_string()
            .contains("missing or not a function")
    );
    let changed = fixture(
        |text| text.replace("return ret\n", "return ret or true\n"),
        COMMON,
    );
    assert!(
        changed
            .observer
            .observe_classes(
                &changed.lua,
                &changed.sources,
                changed.metadata,
                changed.request
            )
            .unwrap_err()
            .to_string()
            .contains("protocol shape changed")
    );
    let mut proxy = fixture(|text| text, COMMON);
    proxy.request.definition_roots.insert(
        "bad".into(),
        proxy.lua.load("return new('Derived').Base").eval().unwrap(),
    );
    assert!(
        proxy
            .observer
            .observe_classes(&proxy.lua, &proxy.sources, proxy.metadata, proxy.request)
            .unwrap_err()
            .to_string()
            .contains("metatable/proxy")
    );
}
#[test]
fn named_definition_tables_are_not_silently_projected() {
    let mut fixture = fixture(|text| text, COMMON);
    let table = fixture.lua.create_table().unwrap();
    table
        .set(
            "unsupported",
            fixture
                .lua
                .globals()
                .get::<Function>("collectgarbage")
                .unwrap(),
        )
        .unwrap();
    fixture
        .request
        .definition_roots
        .insert("Complete".into(), table);
    assert!(
        fixture
            .observer
            .observe_classes(
                &fixture.lua,
                &fixture.sources,
                fixture.metadata,
                fixture.request
            )
            .unwrap_err()
            .to_string()
            .contains("builtin has no observed")
    );
}

#[test]
fn omitted_class_fields_and_requested_methods_are_bounded_before_graph_growth() {
    let oversized = fixture(|text| text, COMMON);
    oversized.lua.load("for i=1,4100 do common.classes.Derived['omitted'..i]=common.classes.Base.Unselected end").exec().unwrap();
    assert!(
        oversized
            .observer
            .observe_classes(
                &oversized.lua,
                &oversized.sources,
                oversized.metadata,
                oversized.request
            )
            .unwrap_err()
            .to_string()
            .contains("omitted field count bound")
    );
    let mut fixture = fixture(|text| text, COMMON);
    fixture.request.classes[0].methods = (0..4100).map(|index| format!("method{index}")).collect();
    assert!(
        fixture
            .observer
            .observe_classes(
                &fixture.lua,
                &fixture.sources,
                fixture.metadata,
                fixture.request
            )
            .unwrap_err()
            .to_string()
            .contains("selected class method count bound")
    );
}

#[test]
fn class_context_keeps_projected_aliases_and_the_closed_protocol_global_contract() {
    let f = fixture(|text| text, COMMON);
    let env = f.lua.globals();
    let context = SourceCaptureContext {
        capture_iteration: false,
        projections: vec![SourceTableSelection {
            table: env.clone(),
            fields: ["Data".into(), "_G".into()].into(),
            indexed: BTreeSet::new(),
            allow_index_fallback: false,
            allow_call_fallback: false,
        }],
        environment: Some(SourceEnvironmentSelection {
            table: env.clone(),
            root_name: "Globals".into(),
        }),
        ..SourceCaptureContext::default()
    };
    let observed = f
        .observer
        .observe_classes_with_context(
            &f.lua,
            &f.sources,
            f.metadata.clone(),
            f.request.clone(),
            context.clone(),
        )
        .unwrap();
    let owner = observed.owner();
    let environment = owner.bind_environment().unwrap().unwrap();
    let data = owner
        .bind_root(SourceProgramDefinitionRoot::Named(
            owner.root_id("Data").unwrap(),
        ))
        .unwrap();
    assert_eq!(
        environment.table().fields["Data"],
        SourceValue::Table(data.table_id())
    );
    assert_eq!(
        environment.table().fields["_G"],
        SourceValue::Table(environment.table_id())
    );
    assert!(owner.classes().is_some());
    let original: Function = env.raw_get("rawget").unwrap();
    env.raw_set("rawget", env.raw_get::<Function>("type").unwrap())
        .unwrap();
    assert!(
        f.observer
            .observe_classes_with_context(
                &f.lua,
                &f.sources,
                f.metadata.clone(),
                f.request.clone(),
                context.clone()
            )
            .is_err()
    );
    env.raw_set("rawget", original).unwrap();
    let mut overlap = context;
    overlap.projections.push(SourceTableSelection {
        table: f.request.classes[0].table.clone(),
        fields: BTreeSet::new(),
        indexed: BTreeSet::new(),
        allow_index_fallback: false,
        allow_call_fallback: false,
    });
    assert!(
        f.observer
            .observe_classes_with_context(&f.lua, &f.sources, f.metadata, f.request, overlap)
            .unwrap_err()
            .to_string()
            .contains("class table cannot also")
    );
}
