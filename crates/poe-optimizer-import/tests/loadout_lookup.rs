//! Authored component contracts over private current-state mocks.
//! These are not source parity or a completed cross-domain loading lifecycle.
use poe_optimizer_data::loadouts::{BUILD_LOADOUT_POLICY_SCHEMA_VERSION, BuildLoadoutPolicy};
use poe_optimizer_import::{
    loadouts::{
        LiveLoadoutContext, LoadoutError, LoadoutErrorKind, LoadoutIds, LoadoutLimits, LoadoutLink,
        LoadoutProgram, LoadoutSpec, Result,
    },
    selected_view::{NumericValue, SelectionDomain as Domain},
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum LinkKey {
    Bytes(Vec<u8>),
    Position(usize),
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Read {
    Singleton(Domain),
    SpecCount,
    Spec(usize),
    Order(Domain, usize),
    Title(Domain, u64),
    Link(Domain, LinkKey),
}
#[derive(Clone)]
struct Spec {
    title: Option<String>,
    version: Option<String>,
}
#[derive(Clone)]
struct Sets {
    // A separately supplied proof of the original length comparison, not a
    // calculation from this mock's sparse positional storage.
    singleton: Option<bool>,
    order: Vec<Option<NumericValue>>,
    winners: Vec<(NumericValue, Option<String>)>,
    links: BTreeMap<LinkKey, Option<NumericValue>>,
}
impl Default for Sets {
    fn default() -> Self {
        Self {
            singleton: Some(false),
            order: Vec::new(),
            winners: Vec::new(),
            links: BTreeMap::new(),
        }
    }
}
#[derive(Default)]
struct Context {
    specs: Vec<Spec>,
    sets: [Sets; 3],
    tree_links: BTreeMap<LinkKey, Option<NumericValue>>,
    reads: Mutex<Vec<Read>>,
}
fn index(domain: Domain) -> usize {
    match domain {
        Domain::Items => 0,
        Domain::Skills => 1,
        Domain::Configuration => 2,
        Domain::Passives => panic!("passives do not use set-key order"),
    }
}
impl Context {
    fn record(&self, read: Read) {
        self.reads.lock().unwrap().push(read);
    }
    fn reads(&self) -> Vec<Read> {
        self.reads.lock().unwrap().clone()
    }
    fn sets_mut(&mut self, domain: Domain) -> &mut Sets {
        &mut self.sets[index(domain)]
    }
    fn add_spec(&mut self, title: Option<&str>, version: Option<&str>) {
        self.specs.push(Spec {
            title: title.map(str::to_owned),
            version: version.map(str::to_owned),
        });
    }
    fn add_set(&mut self, domain: Domain, key: f64, title: Option<&str>) {
        let sets = self.sets_mut(domain);
        sets.order.push(Some(NumericValue::new(key)));
        sets.winners
            .push((NumericValue::new(key), title.map(str::to_owned)));
    }
}
impl LiveLoadoutContext for Context {
    fn is_singleton(&self, domain: Domain) -> Result<bool> {
        self.record(Read::Singleton(domain));
        self.sets[index(domain)].singleton.ok_or_else(|| {
            LoadoutError::new(LoadoutErrorKind::Unavailable, "unproved sparse Lua length")
        })
    }
    fn spec_prefix_len(&self) -> Result<usize> {
        self.record(Read::SpecCount);
        Ok(self.specs.len())
    }
    fn spec(&self, position: usize) -> Result<LoadoutSpec<'_>> {
        self.record(Read::Spec(position));
        let spec = self.specs.get(position - 1).ok_or_else(|| {
            LoadoutError::new(LoadoutErrorKind::Contract, "inconsistent spec prefix")
        })?;
        Ok(LoadoutSpec {
            title: spec.title.as_deref(),
            tree_version: spec.version.as_deref(),
        })
    }
    fn ordered_set(&self, domain: Domain, position: usize) -> Result<Option<NumericValue>> {
        self.record(Read::Order(domain, position));
        Ok(self.sets[index(domain)]
            .order
            .get(position - 1)
            .copied()
            .flatten())
    }
    fn set_title(&self, domain: Domain, key: NumericValue) -> Result<Option<&str>> {
        self.record(Read::Title(domain, key.value().to_bits()));
        // Lua numeric equality determines the winner; the order occurrence still
        // retains the exact numeric value that the source returns.
        self.sets[index(domain)]
            .winners
            .iter()
            .rev()
            .find(|(candidate, _)| candidate.value() == key.value())
            .map(|(_, title)| title.as_deref())
            .ok_or_else(|| LoadoutError::new(LoadoutErrorKind::Source, "missing set winner"))
    }
    fn linked_set(&self, domain: Domain, link: LoadoutLink<'_>) -> Result<Option<NumericValue>> {
        let key = match link {
            LoadoutLink::Bytes(value) => LinkKey::Bytes(value.to_vec()),
            LoadoutLink::Position(value) => LinkKey::Position(value),
        };
        self.record(Read::Link(domain, key.clone()));
        let links = match domain {
            Domain::Passives => &self.tree_links,
            _ => &self.sets[index(domain)].links,
        };
        links
            .get(&key)
            .copied()
            .ok_or_else(|| LoadoutError::new(LoadoutErrorKind::Source, "missing special-link row"))
    }
}
fn policy() -> BuildLoadoutPolicy {
    BuildLoadoutPolicy {
        schema_version: BUILD_LOADOUT_POLICY_SCHEMA_VERSION,
        default_title: "Default".into(),
        latest_tree_version: "current".into(),
        tree_version_display: BTreeMap::from([("old".into(), "Older".into())]),
        version_prefix: "[".into(),
        version_suffix: "] ".into(),
        single_link_pattern: "%{(%w+)%}".into(),
    }
}
fn program() -> LoadoutProgram {
    LoadoutProgram::new(Arc::new(policy()), LoadoutLimits::default()).unwrap()
}
fn failure<T>(result: Result<T>) -> LoadoutError {
    match result {
        Err(error) => error,
        Ok(_) => panic!("expected a component failure"),
    }
}
fn bits(ids: LoadoutIds) -> [Option<u64>; 4] {
    [ids.spec, ids.items, ids.skills, ids.configuration]
        .map(|value| value.map(|value| value.value().to_bits()))
}
fn key(value: f64) -> Option<u64> {
    Some(value.to_bits())
}

#[test]
fn singleton_probes_precede_complete_spec_display_even_after_an_early_name_match() {
    let program = program();
    let mut context = Context::default();
    context.add_spec(Some("Wanted"), Some("current"));
    context.add_spec(Some("Not reached by a name scan"), Some("missing-version"));
    context.sets_mut(Domain::Items).singleton = Some(true);
    let error = failure(program.lookup(&context, "Wanted"));
    assert_eq!(error.kind, LoadoutErrorKind::Source);
    assert_eq!(
        context.reads(),
        vec![
            Read::Singleton(Domain::Skills),
            Read::Singleton(Domain::Items),
            Read::Singleton(Domain::Configuration),
            Read::SpecCount,
            Read::Spec(1),
            Read::Spec(2),
        ]
    );

    let mut context = Context::default();
    context.sets_mut(Domain::Skills).singleton = None;
    assert_eq!(
        failure(program.lookup(&context, "anything")).kind,
        LoadoutErrorKind::Unavailable
    );
    assert_eq!(context.reads(), vec![Read::Singleton(Domain::Skills)]);
}

#[test]
fn first_nonexact_row_can_link_before_a_later_exact_title() {
    let program = program();
    let mut context = Context::default();
    context.add_spec(Some("Other"), Some("current"));
    context.add_spec(Some("Wanted {L}"), Some("current"));
    context
        .tree_links
        .insert(LinkKey::Bytes(b"L".to_vec()), Some(NumericValue::new(99.0)));
    context.add_set(Domain::Items, 10.0, Some("Other"));
    context.add_set(Domain::Items, 20.0, Some("Wanted {L}"));
    context
        .sets_mut(Domain::Items)
        .links
        .insert(LinkKey::Bytes(b"L".to_vec()), Some(NumericValue::new(42.0)));
    context.add_set(Domain::Skills, 3.0, Some("Wanted {L}"));
    let selected = program.lookup(&context, "Wanted {L}").unwrap().unwrap();
    assert_eq!(bits(selected.ids()), [key(99.0), key(42.0), key(3.0), None]);
    let reads = context.reads();
    let reached: Vec<_> = reads
        .iter()
        .filter_map(|read| {
            if let Read::Link(domain, _) = read {
                Some(*domain)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(reached, vec![Domain::Passives, Domain::Items]);
    assert!(!reads.contains(&Read::Order(Domain::Items, 2)));
    assert!(!reads.contains(&Read::Title(Domain::Items, 20.0f64.to_bits())));
}

#[test]
fn first_exact_title_avoids_missing_links_and_domains_resolve_in_source_order() {
    let program = program();
    let mut context = Context::default();
    context.add_spec(Some("Wanted {missing}"), Some("current"));
    for (domain, value) in [
        (Domain::Items, 4.0),
        (Domain::Skills, 2.0),
        (Domain::Configuration, 8.0),
    ] {
        context.add_set(domain, value, Some("Wanted {missing}"));
    }
    let selected = program
        .lookup(&context, "Wanted {missing}")
        .unwrap()
        .unwrap();
    assert_eq!(
        bits(selected.ids()),
        [key(1.0), key(4.0), key(2.0), key(8.0)]
    );
    let reads = context.reads();
    assert!(!reads.iter().any(|read| matches!(read, Read::Link(..))));
    let titles: Vec<_> = reads
        .iter()
        .filter_map(|read| {
            if let Read::Title(domain, _) = read {
                Some(*domain)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        titles,
        vec![Domain::Items, Domain::Skills, Domain::Configuration]
    );
}

#[test]
fn partial_results_and_one_nil_are_distinct_and_singletons_need_no_winner() {
    let program = program();
    let empty = Context::default();
    assert!(program.lookup(&empty, "absent").unwrap().is_none());
    let mut context = Context::default();
    context.add_spec(Some("Different"), Some("current"));
    let sets = context.sets_mut(Domain::Configuration);
    sets.singleton = Some(true);
    sets.order.push(Some(NumericValue::new(7.0)));
    let selected = program.lookup(&context, "absent").unwrap().unwrap();
    assert_eq!(bits(selected.ids()), [None, None, None, key(7.0)]);
    assert!(
        !context
            .reads()
            .iter()
            .any(|read| matches!(read, Read::Title(..)))
    );
}

#[test]
fn numeric_keys_keep_bits_and_signed_zero_still_uses_the_current_winner() {
    let program = program();
    for value in [-0.0f64, 0.0, 2.5, f64::INFINITY, f64::NEG_INFINITY] {
        let mut context = Context::default();
        context.add_set(Domain::Items, value, Some("Key"));
        let selected = program.lookup(&context, "Key").unwrap().unwrap();
        assert_eq!(bits(selected.ids()), [None, key(value), None, None]);
    }
    let mut context = Context::default();
    context.add_set(Domain::Items, -0.0, Some("Earlier"));
    context.add_set(Domain::Items, 0.0, Some("Current winner"));
    let selected = program.lookup(&context, "Current winner").unwrap().unwrap();
    assert_eq!(bits(selected.ids()), [None, key(-0.0), None, None]);
    assert!(!context.reads().contains(&Read::Order(Domain::Items, 2)));
}

#[test]
fn missing_rows_error_but_a_present_link_without_id_returns_nil_immediately() {
    let program = program();
    let mut context = Context::default();
    context
        .sets_mut(Domain::Items)
        .order
        .push(Some(NumericValue::new(1.0)));
    let error = failure(program.lookup(&context, "Wanted {L}"));
    assert_eq!(error.kind, LoadoutErrorKind::Source);
    assert_eq!(error.message, "missing set winner");
    assert!(
        !context
            .reads()
            .iter()
            .any(|read| matches!(read, Read::Link(..)))
    );

    context
        .sets_mut(Domain::Items)
        .winners
        .push((NumericValue::new(1.0), Some("Other".into())));
    let error = failure(program.lookup(&context, "Wanted {L}"));
    assert_eq!(error.kind, LoadoutErrorKind::Source);
    assert_eq!(error.message, "missing special-link row");
    context.add_set(Domain::Items, 2.0, Some("Wanted {L}"));
    context
        .sets_mut(Domain::Items)
        .links
        .insert(LinkKey::Bytes(b"L".to_vec()), None);
    context.reads.lock().unwrap().clear();
    assert!(program.lookup(&context, "Wanted {L}").unwrap().is_none());
    assert!(!context.reads().contains(&Read::Order(Domain::Items, 2)));
}

#[test]
fn ipairs_holes_and_proved_singleton_are_independent_and_unknown_length_stops_early() {
    let program = program();
    let mut context = Context::default();
    context.add_set(Domain::Items, 7.0, Some("Other"));
    context.sets_mut(Domain::Items).order.push(None);
    context.add_set(Domain::Items, 9.0, Some("Wanted"));
    assert!(program.lookup(&context, "Wanted").unwrap().is_none());
    assert!(context.reads().contains(&Read::Order(Domain::Items, 2)));
    assert!(!context.reads().contains(&Read::Order(Domain::Items, 3)));

    context.sets_mut(Domain::Items).singleton = Some(true);
    context.reads.lock().unwrap().clear();
    let selected = program.lookup(&context, "Wanted").unwrap().unwrap();
    assert_eq!(bits(selected.ids()), [None, key(7.0), None, None]);
    assert!(
        !context
            .reads()
            .iter()
            .any(|read| matches!(read, Read::Title(..)))
    );

    context.sets_mut(Domain::Items).singleton = None;
    context.reads.lock().unwrap().clear();
    assert_eq!(
        failure(program.lookup(&context, "Wanted")).kind,
        LoadoutErrorKind::Unavailable
    );
    assert_eq!(
        context.reads(),
        vec![
            Read::Singleton(Domain::Skills),
            Read::Singleton(Domain::Items)
        ]
    );
}

#[test]
fn custom_policy_controls_display_defaults_versions_and_link_captures() {
    let mut injected = policy();
    injected.default_title = "Vacant".into();
    injected.version_prefix = "<".into();
    injected.version_suffix = "> ".into();
    injected
        .tree_version_display
        .insert("old".into(), "Previous".into());
    injected.single_link_pattern = "%((%w+)%)".into();
    let program = LoadoutProgram::new(Arc::new(injected), LoadoutLimits::default()).unwrap();
    let mut context = Context::default();
    context.add_spec(None, Some("old"));
    context.add_spec(Some(""), Some("current"));
    assert_eq!(
        program.spec_list(&context).unwrap(),
        vec!["<Previous> Vacant", ""]
    );
    context
        .tree_links
        .insert(LinkKey::Bytes(b"L".to_vec()), Some(NumericValue::new(8.5)));
    let selected = program.lookup(&context, "Wanted (L)").unwrap().unwrap();
    assert_eq!(bits(selected.ids()), [key(8.5), None, None, None]);
    let selected = program.lookup(&context, "").unwrap().unwrap();
    assert_eq!(bits(selected.ids()), [key(2.0), None, None, None]);

    let mut injected = policy();
    injected.single_link_pattern = "()L".into();
    let positions = LoadoutProgram::new(Arc::new(injected), LoadoutLimits::default()).unwrap();
    let mut context = Context::default();
    context.add_spec(Some("Other"), Some("current"));
    context
        .tree_links
        .insert(LinkKey::Position(2), Some(NumericValue::new(6.0)));
    let selected = positions.lookup(&context, "xL").unwrap().unwrap();
    assert_eq!(bits(selected.ids()), [key(6.0), None, None, None]);
}

#[test]
fn malformed_patterns_are_lazy_after_exact_matches_and_empty_iterations() {
    let mut injected = policy();
    injected.single_link_pattern = "[".into();
    let program = LoadoutProgram::new(Arc::new(injected), LoadoutLimits::default()).unwrap();
    let empty = Context::default();
    assert!(program.lookup(&empty, "Wanted").unwrap().is_none());
    let mut context = Context::default();
    context.add_spec(Some("Wanted"), Some("current"));
    context.add_set(Domain::Items, 1.0, Some("Wanted"));
    assert!(program.lookup(&context, "Wanted").unwrap().is_some());
    assert_eq!(
        failure(program.lookup(&context, "Other")).kind,
        LoadoutErrorKind::Source
    );
    context.add_set(Domain::Skills, 2.0, Some("Other"));
    assert_eq!(
        failure(program.lookup(&context, "Wanted")).kind,
        LoadoutErrorKind::Source
    );
}

#[test]
fn resource_limits_stop_before_unbounded_context_reads_or_output() {
    let context = Context::default();
    let program = LoadoutProgram::new(
        Arc::new(policy()),
        LoadoutLimits {
            max_steps: 0,
            ..LoadoutLimits::default()
        },
    )
    .unwrap();
    assert_eq!(
        failure(program.lookup(&context, "anything")).kind,
        LoadoutErrorKind::Resource
    );
    assert!(context.reads().is_empty());

    for limits in [
        LoadoutLimits {
            max_specs: 0,
            ..LoadoutLimits::default()
        },
        LoadoutLimits {
            max_output_bytes: 0,
            ..LoadoutLimits::default()
        },
    ] {
        let program = LoadoutProgram::new(Arc::new(policy()), limits).unwrap();
        let mut context = Context::default();
        context.add_spec(Some("Title"), Some("current"));
        assert_eq!(
            failure(program.spec_list(&context)).kind,
            LoadoutErrorKind::Resource
        );
        assert_eq!(context.reads(), vec![Read::SpecCount]);
    }

    let program = LoadoutProgram::new(
        Arc::new(policy()),
        LoadoutLimits {
            max_set_reads: 0,
            ..LoadoutLimits::default()
        },
    )
    .unwrap();
    let context = Context::default();
    assert_eq!(
        failure(program.lookup(&context, "anything")).kind,
        LoadoutErrorKind::Resource
    );
    assert!(
        !context
            .reads()
            .iter()
            .any(|read| matches!(read, Read::Order(..)))
    );

    let mut injected = policy();
    injected.single_link_pattern.clear();
    let program = LoadoutProgram::new(
        Arc::new(injected),
        LoadoutLimits {
            max_text_bytes: 4,
            ..LoadoutLimits::default()
        },
    )
    .unwrap();
    let mut context = Context::default();
    context.add_spec(Some("Long title"), Some("current"));
    assert_eq!(
        failure(program.spec_list(&context)).kind,
        LoadoutErrorKind::Resource
    );

    assert_eq!(
        failure(LoadoutProgram::new(
            Arc::new(policy()),
            LoadoutLimits {
                max_compiled_bytes: 0,
                ..LoadoutLimits::default()
            },
        ))
        .kind,
        LoadoutErrorKind::Resource
    );
}

#[test]
fn selections_belong_to_exact_context_and_program_even_when_policy_storage_is_shared() {
    let policy = Arc::new(policy());
    let first = LoadoutProgram::new(policy.clone(), LoadoutLimits::default()).unwrap();
    let second = LoadoutProgram::new(policy, LoadoutLimits::default()).unwrap();
    let mut context = Context::default();
    context.add_spec(Some("Title"), Some("current"));
    let mut equal_context = Context::default();
    equal_context.add_spec(Some("Title"), Some("current"));
    let selected = first.lookup(&context, "Title").unwrap().unwrap();
    assert!(selected.belongs_to(&context, &first));
    assert!(!selected.belongs_to(&context, &second));
    assert!(!selected.belongs_to(&equal_context, &first));
    assert!(Arc::ptr_eq(first.policy(), second.policy()));

    let first_list = first.spec_list(&context).unwrap();
    let mut next_list = first.spec_list(&context).unwrap();
    next_list[0].push_str(" changed by caller");
    assert_eq!(first_list, vec!["Title"]);
    assert_eq!(first.spec_list(&context).unwrap(), first_list);
}

#[test]
fn independent_calls_share_immutable_program_and_current_state_across_threads() {
    let program = program();
    let mut context = Context::default();
    context.add_spec(Some("Title"), Some("current"));
    context.add_set(Domain::Items, 2.5, Some("Title"));
    std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..8 {
            let program = &program;
            let context = &context;
            workers.push(scope.spawn(move || {
                let selected = program.lookup(context, "Title").unwrap().unwrap();
                assert!(selected.belongs_to(context, program));
                assert_eq!(bits(selected.ids()), [key(1.0), key(2.5), None, None]);
                assert_eq!(program.spec_list(context).unwrap(), vec!["Title"]);
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    });
}

#[test]
fn repeated_current_version_comparisons_respect_text_and_work_limits() {
    let version = "v".repeat(64);
    let mut injected = policy();
    injected.latest_tree_version = version.clone();
    let policy = Arc::new(injected);

    let text_limited = LoadoutProgram::new(
        policy.clone(),
        LoadoutLimits {
            max_text_bytes: 32,
            ..LoadoutLimits::default()
        },
    )
    .unwrap();
    let mut context = Context::default();
    context.add_spec(Some("T"), Some(&version));
    assert_eq!(
        failure(text_limited.spec_list(&context)).kind,
        LoadoutErrorKind::Resource
    );

    let work_limited = LoadoutProgram::new(
        policy,
        LoadoutLimits {
            max_steps: 320,
            ..LoadoutLimits::default()
        },
    )
    .unwrap();
    let mut repeated = Context::default();
    for _ in 0..4 {
        repeated.add_spec(Some("T"), Some(&version));
    }
    assert_eq!(
        failure(work_limited.spec_list(&repeated)).kind,
        LoadoutErrorKind::Resource
    );
    // This reaches multiple actual comparisons, then exhausts one shared call
    // budget before fetching every row. Equal version values are still read work.
    let spec_reads = repeated
        .reads()
        .iter()
        .filter(|read| matches!(read, Read::Spec(_)))
        .count();
    assert!((2..4).contains(&spec_reads));
}
