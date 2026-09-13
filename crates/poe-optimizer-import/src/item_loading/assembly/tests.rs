use super::*;
use crate::item_loading::{
    AssemblyExecution, FormatRequest, ItemLoadMachine, ParseOutcome, ParseRequest,
    UnavailableItemLoadProvider,
};
use poe_optimizer_data::{
    game_data::{GameDataSnapshot, bundled_snapshot},
    item_loading::{ItemMetadataTable, ItemMetadataValue},
};
use std::{collections::BTreeMap, sync::OnceLock};
fn data() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn definitions() -> ItemAssemblyDefinitions<'static> {
    let d = data();
    d.item_assembly()
        .bind(
            d.item_loading(),
            d.item_scalability(),
            &d.package().actor,
            d.modifier_parser(),
        )
        .unwrap()
}
#[derive(Default)]
struct Capture {
    request: Option<AssemblyRequest>,
}
impl ItemLoadProvider for Capture {
    fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
        DependencyResult::Available(ParseOutcome {
            modifiers: Some(Vec::new()),
            extra: None,
        })
    }
    fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
        DependencyResult::Available(r.text.clone())
    }
    fn assemble_with_trace(&mut self, r: &AssemblyRequest) -> AssemblyExecution {
        self.request = Some(r.clone());
        AssemblyExecution {
            outcome: DependencyResult::Unavailable("captured input".into()),
            prefix: None,
        }
    }
}
fn request() -> AssemblyRequest {
    // Authentic machine ownership; the parser stub is only input setup for the
    // structural algorithm tests below, never a source parity observation.
    let mut machine = ItemLoadMachine::new(data().item_loading());
    let mut provider = Capture::default();
    machine
        .apply_text("Rarity: NORMAL\nGold Ring", &mut provider)
        .unwrap();
    provider.request.unwrap()
}
fn context<'r>(
    request: &'r AssemblyRequest,
    dependency: &'r mut UnavailableItemLoadProvider,
) -> Context<'static, 'r, UnavailableItemLoadProvider> {
    let limits = AssemblyLimits::default();
    let mut arena = Arena::new(limits);
    let root = arena.new_table().unwrap();
    Context {
        arena,
        root,
        definitions: definitions(),
        request,
        dependencies: dependency,
        stage: "test",
        patterns: text::Patterns::new(limits),
        sequence: 0,
    }
}
fn record(name: &str, kind: &str, value: ItemMetadataValue) -> ItemMetadataTable {
    ItemMetadataTable {
        fields: [
            ("name".into(), ItemMetadataValue::Text(name.into())),
            ("type".into(), ItemMetadataValue::Text(kind.into())),
            ("flags".into(), ItemMetadataValue::Number(0.0)),
            ("keywordFlags".into(), ItemMetadataValue::Number(0.0)),
            ("value".into(), value),
        ]
        .into(),
        indexed: BTreeMap::new(),
    }
}
#[test]
fn local_failure_keeps_consumed_alias_prefix_and_bad_row() {
    let request = request();
    let mut dependency = UnavailableItemLoadProvider;
    let mut c = context(&request, &mut dependency);
    let query = &c.definitions.policy().requirements.attributes[0].base;
    let list = c.mod_list().unwrap();
    let shared = c
        .arena
        .import_metadata(&record(
            &query.name,
            &query.mod_type,
            ItemMetadataValue::Number(7.0),
        ))
        .unwrap();
    let bad = c
        .arena
        .import_metadata(&record(
            &query.name,
            &query.mod_type,
            ItemMetadataValue::Boolean(false),
        ))
        .unwrap();
    c.arena.append(list, Value::Table(shared)).unwrap();
    c.arena.append(list, Value::Table(shared)).unwrap();
    c.arena.append(list, Value::Table(bad)).unwrap();
    assert_eq!(
        c.local(list, query).unwrap_err().kind,
        AssemblyErrorKind::Source
    );
    assert_eq!(c.arena.dense_len(list).unwrap(), 1);
    assert_eq!(c.arena.get_index(list, 1).unwrap(), Value::Table(bad));
    assert_eq!(
        c.arena.get_field(shared, "value").unwrap(),
        Value::Number(7.0)
    );
}
#[test]
fn scale_one_and_unscalable_keep_identity_while_scaling_copies() {
    let request = request();
    let mut dependency = UnavailableItemLoadProvider;
    let mut c = context(&request, &mut dependency);
    let list = c.mod_list().unwrap();
    let record = c
        .arena
        .import_metadata(&record(
            "FiniteTestStat",
            &c.definitions.policy().kinds.base,
            ItemMetadataValue::Number(8.0),
        ))
        .unwrap();
    c.scale_add(list, record, 1.0).unwrap();
    c.scale_add(list, record, 0.5).unwrap();
    assert_eq!(c.arena.get_index(list, 1).unwrap(), Value::Table(record));
    let copied = table(c.arena.get_index(list, 2).unwrap()).unwrap();
    assert_ne!(copied, record);
    assert_eq!(
        c.arena.get_field(copied, "value").unwrap(),
        Value::Number(4.0)
    );
    let tag = c.arena.new_table().unwrap();
    c.arena
        .set_field(tag, "unscalable", Value::Boolean(true))
        .unwrap();
    c.arena.set_index(record, 1, Value::Table(tag)).unwrap();
    c.scale_add(list, record, 0.25).unwrap();
    assert_eq!(c.arena.get_index(list, 3).unwrap(), Value::Table(record));
    assert_eq!(
        c.arena.get_field(record, "value").unwrap(),
        Value::Number(8.0)
    );
}
#[test]
fn selected_tagged_grant_is_a_frontier_but_unrelated_tag_survives() {
    let request = request();
    let mut dependency = UnavailableItemLoadProvider;
    let mut c = context(&request, &mut dependency);
    let policy = c.definitions.policy();
    let list = c.mod_list().unwrap();
    let other = c
        .arena
        .import_metadata(&record(
            "UnrelatedResidual",
            &policy.kinds.list,
            ItemMetadataValue::Boolean(true),
        ))
        .unwrap();
    let selected = c
        .arena
        .import_metadata(&record(
            &policy.grants.query_name,
            &policy.kinds.list,
            ItemMetadataValue::Boolean(true),
        ))
        .unwrap();
    let tag = c.arena.new_table().unwrap();
    c.arena
        .set_field(tag, "type", Value::Text("Condition".into()))
        .unwrap();
    c.arena.set_index(other, 1, Value::Table(tag)).unwrap();
    c.arena.set_index(selected, 1, Value::Table(tag)).unwrap();
    c.arena.append(list, Value::Table(other)).unwrap();
    let empty = table(c.nil_query(list, &policy.grants.query_name, false).unwrap()).unwrap();
    assert_eq!(c.arena.dense_len(empty).unwrap(), 0);
    c.arena.append(list, Value::Table(selected)).unwrap();
    assert_eq!(
        c.nil_query(list, &policy.grants.query_name, false)
            .unwrap_err()
            .kind,
        AssemblyErrorKind::Unsupported
    );
    assert_eq!(c.arena.dense_len(list).unwrap(), 2);
    assert_eq!(c.arena.get_index(other, 1).unwrap(), Value::Table(tag));
}
#[test]
fn input_adaptation_failure_has_no_forged_source_prefix() {
    let request = request();
    let mut dependency = UnavailableItemLoadProvider;
    let result = assemble_with_limits(
        definitions(),
        &request,
        &mut dependency,
        None,
        AssemblyLimits {
            max_tables: 1,
            ..AssemblyLimits::default()
        },
    );
    assert_eq!(result.stage, "input");
    assert_eq!(result.result.unwrap_err().kind, AssemblyErrorKind::Resource);
    assert!(result.partial.is_none());
}
#[test]
fn pattern_and_graph_work_share_one_limit_without_reset() {
    let limits = AssemblyLimits {
        max_steps: 20,
        ..AssemblyLimits::default()
    };
    let mut arena = Arena::new(limits);
    let mut patterns = text::Patterns::new(limits);
    assert!(patterns.find(&mut arena, "a", "a").unwrap());
    let used = arena.usage().steps;
    arena.work(20 - used).unwrap();
    assert_eq!(
        patterns.find(&mut arena, "a", "a").unwrap_err().kind,
        AssemblyErrorKind::Resource
    );
}

#[test]
fn nil_queries_retain_high_keyword_bits_and_match_all_semantics() {
    let request = request();
    let mut dependency = UnavailableItemLoadProvider;
    let mut c = context(&request, &mut dependency);
    let policy = c.definitions.policy();
    assert_eq!(policy.nil_queries.keyword_flags, 0.0);
    let match_all = c.definitions.keyword_flags().fields[&policy.nil_queries.match_all_field]
        .as_f64()
        .unwrap();
    for flag in [false, true] {
        let name = if flag {
            &policy.jewel_restrictions.query_name
        } else {
            &policy.grants.query_name
        };
        let kind = if flag {
            &policy.kinds.flag
        } else {
            &policy.kinds.list
        };
        let list = c.mod_list().unwrap();
        for keywords in [
            4_294_967_296.0,
            1_099_511_627_776.0,
            4_294_967_296.0 + match_all,
        ] {
            let record = c
                .arena
                .import_metadata(&record(name, kind, ItemMetadataValue::Boolean(true)))
                .unwrap();
            c.arena
                .set_field(record, "keywordFlags", Value::Number(keywords))
                .unwrap();
            c.arena.append(list, Value::Table(record)).unwrap();
        }
        let result = c.nil_query(list, name, flag).unwrap();
        if flag {
            assert_eq!(result, Value::Nil);
        } else {
            assert_eq!(c.arena.dense_len(table(result).unwrap()).unwrap(), 0);
        }
        let record = c
            .arena
            .import_metadata(&record(name, kind, ItemMetadataValue::Number(0.0)))
            .unwrap();
        c.arena
            .set_field(record, "keywordFlags", Value::Number(match_all))
            .unwrap();
        c.arena.append(list, Value::Table(record)).unwrap();
        let result = c.nil_query(list, name, flag).unwrap();
        if flag {
            assert_eq!(result, Value::Boolean(true));
        } else {
            let output = table(result).unwrap();
            assert_eq!(c.arena.dense_len(output).unwrap(), 1);
            assert_eq!(c.arena.get_index(output, 1).unwrap(), Value::Number(0.0));
        }
    }
}
