//! Final constituent composition uses the existing release and Engine checks.
#[path = "release/arithmetic.rs"]
mod arithmetic;
#[path = "release/attribute_support_domains.rs"]
mod attribute_support_domains;
#[path = "release/attribute_support_source.rs"]
mod attribute_support_source;
#[path = "release/checkpoint.rs"]
mod checkpoint;
#[path = "release/class_support_domains.rs"]
mod class_support_domains;
#[path = "release/class_support_source.rs"]
mod class_support_source;
#[path = "release/content_bindings.rs"]
mod content_bindings;
#[path = "release/declarations.rs"]
mod declarations;
#[path = "release/defence_components.rs"]
mod defence_components;
#[allow(dead_code, unused_imports)]
#[path = "../../poe-optimizer-engine/tests/support/owned_fixtures.rs"]
mod engine_fixtures;
#[path = "release/evaluation_content.rs"]
mod evaluation_content;
#[path = "../../../tests/support/owned_evaluation_release_fixture.rs"]
mod fixture;
#[path = "release/gem_support_domains.rs"]
mod gem_support_domains;
#[path = "release/gigantic_support_domains.rs"]
mod gigantic_support_domains;
#[path = "release/gigantic_support_source.rs"]
mod gigantic_support_source;
#[path = "release/initial_skill_types.rs"]
mod initial_skill_types;
#[path = "release/item_metadata.rs"]
mod item_metadata;
#[path = "release/local_scaling.rs"]
mod local_scaling;
#[path = "release/minion_support_domains.rs"]
mod minion_support_domains;
#[path = "release/minion_support_source.rs"]
mod minion_support_source;
#[path = "release/normalization_content.rs"]
mod normalization_content;
#[path = "release/originals.rs"]
mod originals;
#[path = "release/player_support_domains.rs"]
mod player_support_domains;
#[path = "release/player_support_source.rs"]
mod player_support_source;
#[path = "release/root_support_domains.rs"]
mod root_support_domains;
#[path = "release/scalar_attributes.rs"]
mod scalar_attributes;
#[path = "release/sniper_support_domains.rs"]
mod sniper_support_domains;
#[path = "release/socket_support_domains.rs"]
mod socket_support_domains;
#[path = "release/socket_support_source.rs"]
mod socket_support_source;
#[path = "release/source_dependencies.rs"]
mod source_dependencies;
#[path = "release/structural_support_domains.rs"]
mod structural_support_domains;
#[path = "release/structural_support_source.rs"]
mod structural_support_source;
#[path = "release/usage_support_domains.rs"]
mod usage_support_domains;
#[path = "release/view_support_domains.rs"]
mod view_support_domains;
#[path = "release/view_support_source.rs"]
mod view_support_source;

use flate2::{Compression, GzBuilder, read::GzDecoder};
use poe_optimizer_authoring::{
    BuiltRelease, OwnedReleaseEvaluationContent, ReleaseAllocationBase, ReleaseBuildError,
    ReleaseBuildLimits, ReleaseSource, ReleaseSourceParts, SourceEncoding, SymbolRegistry,
    build_release, load_names, lower_program, parse_program, parse_release_manifest, raise_program,
};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{DefId, OwnedDefinitionKey},
    owned_rules::{RuleExpression, RuleNode, RulePackageInput, RuleProgram},
    owned_schema::{DefinitionSchemaIndex, SchemaSubject},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::{
    owned_plan::{OwnedSupportEffectPlan, OwnedSupportMetricPlan, SupportEffectPlanInputs},
    owned_rules::CompiledRulePackage,
};
use poe_optimizer_import::{
    owned_recipe::CheckedOwnedRecipe,
    owned_release::{
        OWNED_RELEASE_VERSION, OwnedReleaseError, OwnedReleaseInput, OwnedReleaseReceipt,
        StagedOwnedRelease, assemble_owned_release, load_owned_release_artifacts,
    },
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::Path,
    process::{Command, Output},
    sync::{Arc, OnceLock},
};

fn fixture() -> &'static fixture::Fixture {
    use poe_optimizer_import::owned_item_source::{
        ItemSourceDialect, OWNED_ITEM_SOURCE_CATEGORY_POLICY_VERSION,
        OWNED_ITEM_SOURCE_POLICY_VERSION,
    };

    static FIXTURE: OnceLock<fixture::Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let mut fixture = fixture::fixture();
        assert!(fixture.input.items.rules.is_empty());
        let source = &mut fixture.input.item_source;
        assert_eq!(source.schema_version, OWNED_ITEM_SOURCE_POLICY_VERSION);
        assert!(matches!(
            &source.dialect,
            ItemSourceDialect::PobExportedSingleTextV1
        ));
        assert!(source.property_bindings.is_empty());
        assert!(source.template_defaults.is_empty());
        assert!(source.rule_layouts.is_empty());
        assert!(source.template_layouts.is_empty());
        // This local fixture authors an empty current dialect. The shared
        // fixture's numerical request and expected formulas are unchanged.
        source.schema_version = OWNED_ITEM_SOURCE_CATEGORY_POLICY_VERSION;
        source.dialect = ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
            flag_bindings: vec![],
            metadata_rules: vec![],
            single_modifier_conditions: vec![],
            preamble_observations: vec![],
            category_bindings: vec![],
        };
        fixture
    })
}

fn tracked_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/owned/poe2/3887ae68/migration-source")
}

fn tracked_sources() -> &'static Sources {
    static SOURCES: OnceLock<Sources> = OnceLock::new();
    SOURCES.get_or_init(|| {
        let root = tracked_root();
        let text = fs::read_to_string(root.join("release.kdl")).unwrap();
        let limits = ReleaseBuildLimits::default();
        let manifest = parse_release_manifest(&text, limits).unwrap();
        let mut decoded_bytes = 0;
        let files = manifest
            .sources()
            .iter()
            .map(|spec| {
                let bytes = fs::read(root.join(spec.path())).unwrap();
                decoded_bytes += match spec.encoding() {
                    SourceEncoding::Plain => bytes.len(),
                    SourceEncoding::Gzip => {
                        let mut decoded = Vec::new();
                        GzDecoder::new(bytes.as_slice())
                            .take(limits.max_decoded_bytes as u64 + 1)
                            .read_to_end(&mut decoded)
                            .unwrap();
                        assert!(decoded.len() <= limits.max_decoded_bytes);
                        decoded.len()
                    }
                };
                (spec.path().to_owned(), bytes)
            })
            .collect();
        Sources {
            manifest: text,
            files,
            decoded_bytes,
        }
    })
}

fn tracked_release() -> &'static BuiltRelease {
    static RELEASE: OnceLock<BuiltRelease> = OnceLock::new();
    RELEASE.get_or_init(|| tracked_sources().build(Default::default()).unwrap())
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::fast());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

#[derive(Clone)]
struct Sources {
    manifest: String,
    files: Vec<(String, Vec<u8>)>,
    decoded_bytes: usize,
}
impl Sources {
    fn new(input: &OwnedReleaseInput, compressed: bool) -> Self {
        let evaluation = authored_evaluation(input);
        Self::with_evaluation_content(input, evaluation.as_ref(), compressed)
    }

    fn with_evaluation_content(
        input: &OwnedReleaseInput,
        evaluation: Option<&OwnedReleaseEvaluationContent>,
        compressed: bool,
    ) -> Self {
        let mut components: Vec<(&str, Vec<u8>)> = Vec::new();
        macro_rules! component {
            ($name:literal, $value:expr) => {
                components.push(($name, serde_json::to_vec($value).unwrap()));
            };
        }
        component!("registry", &input.recipe.registry);
        component!("schema", &input.recipe.schema);
        component!("rules", &input.recipe.rules);
        component!("routing", &input.recipe.routing);
        component!("mapping", &input.mapping);
        component!("roles", &input.roles);
        component!("normalization", &input.normalization);
        component!("rewards", &input.rewards);
        component!("items", &input.items);
        component!("item-source", &input.item_source);
        if let Some(tree) = &input.tree {
            component!("tree-normalization", tree);
        }
        if let Some(evaluation) = evaluation {
            component!("metrics", &evaluation.metrics);
            if let Some(support) = &evaluation.support {
                component!("support-stages", &support.stages);
                component!("support-preparation", &support.preparation);
                component!("support-inputs", &support.inputs);
                component!("support-receiving", &support.receiving);
                if let Some(outputs) = &support.outputs {
                    component!("support-outputs", outputs);
                }
            }
        }
        component!("provenance", &input.provenance);
        let mut output = Self {
            manifest: "release {\n".into(),
            files: Vec::new(),
            decoded_bytes: 0,
        };
        for (component, bytes) in components {
            output.push(
                component,
                &format!("{component}.json"),
                bytes,
                compressed,
                None,
            );
        }
        for query in &input.query_sets {
            output.push(
                "queries",
                &format!("queries-{}.json", query.name),
                serde_json::to_vec(&query.queries).unwrap(),
                compressed,
                Some(query.name.as_str()),
            );
        }
        output.manifest.push_str("}\n");
        output
    }

    fn push(
        &mut self,
        component: &str,
        path: &str,
        bytes: Vec<u8>,
        compressed: bool,
        query: Option<&str>,
    ) {
        let path = if compressed {
            format!("{path}.gz")
        } else {
            path.into()
        };
        let encoding = if compressed { "gzip" } else { "plain" };
        let declaration = match query {
            Some(name) => {
                format!("    queries \"{name}\" source=\"{path}\" encoding=\"{encoding}\"\n")
            }
            None => format!("    {component} \"{path}\" encoding=\"{encoding}\"\n"),
        };
        self.manifest.push_str(&declaration);
        self.decoded_bytes += bytes.len();
        self.files
            .push((path, if compressed { gzip(&bytes) } else { bytes }));
    }

    fn sources(&self) -> Vec<ReleaseSource<'_>> {
        self.files
            .iter()
            .map(|(path, bytes)| ReleaseSource { path, bytes })
            .collect()
    }

    fn build(&self, limits: ReleaseBuildLimits) -> Result<BuiltRelease, ReleaseBuildError> {
        self.build_with_allocation(ReleaseAllocationBase::Bootstrap, limits)
    }

    fn build_with_allocation(
        &self,
        allocation: ReleaseAllocationBase<'_>,
        limits: ReleaseBuildLimits,
    ) -> Result<BuiltRelease, ReleaseBuildError> {
        let manifest = parse_release_manifest(&self.manifest, limits)?;
        build_release(&manifest, &self.sources(), allocation, limits)
    }

    fn encoded_bytes(&self) -> usize {
        self.files.iter().map(|(_, bytes)| bytes.len()).sum()
    }

    fn declare(&mut self, declaration: &str) {
        self.manifest
            .insert_str(self.manifest.rfind('}').unwrap(), declaration);
    }

    fn add_plain(&mut self, path: &str, text: String) {
        assert!(!self.files.iter().any(|(name, _)| name == path));
        self.decoded_bytes += text.len();
        self.files.push((path.into(), text.into_bytes()));
    }

    fn text(&self, path: &str) -> &str {
        std::str::from_utf8(&self.files.iter().find(|(name, _)| name == path).unwrap().1).unwrap()
    }

    fn replace_plain(&mut self, path: &str, text: String) {
        let bytes = &mut self
            .files
            .iter_mut()
            .find(|(name, _)| name == path)
            .unwrap()
            .1;
        self.decoded_bytes = self.decoded_bytes - bytes.len() + text.len();
        *bytes = text.into_bytes();
    }
}

fn files(release: &StagedOwnedRelease) -> BTreeMap<String, Vec<u8>> {
    release
        .artifacts()
        .map(|(name, bytes)| (name.into(), bytes.to_vec()))
        .collect()
}

fn invalid<T>(result: Result<T, ReleaseBuildError>, message: &str) {
    match result {
        Err(ReleaseBuildError::Invalid(actual)) => assert_eq!(actual, message),
        Err(other) => panic!("expected invalid {message:?}, got {other}"),
        Ok(_) => panic!("unexpected success: {message}"),
    }
}

fn limited<T>(result: Result<T, ReleaseBuildError>, message: &str) {
    match result {
        Err(ReleaseBuildError::Limit(actual)) => assert_eq!(actual, message),
        Err(other) => panic!("expected limit {message:?}, got {other}"),
        Ok(_) => panic!("unexpected success: {message}"),
    }
}

fn metric_report(
    release: &StagedOwnedRelease,
) -> poe_optimizer_engine::owned_plan::OwnedSupportMetricReport {
    let recipe = release.assembled();
    let definitions = Arc::new(recipe.schema().clone());
    let rules = Arc::new(
        CompiledRulePackage::compile_stored(
            recipe.rules(),
            definitions.as_ref(),
            Default::default(),
        )
        .unwrap(),
    );
    let evaluation = release.evaluation().unwrap();
    let support = evaluation.support().unwrap();
    let effects = OwnedSupportEffectPlan::compile_with_outputs(
        SupportEffectPlanInputs {
            request: Arc::new(fixture().request.clone()),
            definitions,
            rules,
            routing: Arc::new(recipe.routing().clone()),
            stages: Arc::new(support.stages().clone()),
            preparation: Arc::new(support.preparation().clone()),
            inputs: Arc::new(support.inputs().clone()),
            receiving: Arc::new(support.receiving().clone()),
        },
        Arc::new(support.outputs().unwrap().clone()),
        Default::default(),
        Default::default(),
    )
    .unwrap();
    let plan =
        OwnedSupportMetricPlan::compile(Arc::new(effects), Arc::new(evaluation.metrics().clone()))
            .unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}

fn assert_dsl_metric(release: &StagedOwnedRelease) {
    let actual = metric_report(release);
    let expected = &fixture().expected;
    // Renamed graph nodes alter content identity, not the independently held
    // observations, support disposition, gaps or requirement assessment.
    assert_ne!(actual.evaluation.identity, expected.evaluation.identity);
    assert_eq!(actual.evaluation.results, expected.evaluation.results);
    assert_eq!(actual.evaluation.gaps, expected.evaluation.gaps);
    assert_eq!(
        actual.evaluation.requirements,
        expected.evaluation.requirements
    );
    assert_eq!(actual.support, expected.support);
}

// Fixture aliases are deliberately synthetic. They exercise the public text APIs
// without establishing a production naming policy or interpreting source labels.
fn fixture_names(schema: &OwnedDefinitionSchemaPackage) -> (String, SymbolRegistry) {
    let definitions: Vec<_> = schema
        .input()
        .definitions
        .iter()
        .map(|entry| entry.address())
        .collect();
    let mut text = format!(
        "names game=\"{}\" version=\"{}\" {{\n",
        schema.namespace().game(),
        schema.namespace().version()
    );
    for (index, address) in definitions.iter().enumerate() {
        text.push_str(&format!(
            " {} \"d_{index}\" key=\"{}\"\n",
            address.kind().as_str(),
            address.key()
        ));
    }
    for (index, entry) in schema.input().slots.iter().enumerate() {
        let address = entry.address();
        let owner = address.declaration();
        let owner_index = definitions
            .iter()
            .position(|definition| {
                definition.kind() == owner.kind()
                    && definition.namespace() == owner.namespace()
                    && definition.key() == owner.key()
            })
            .unwrap();
        text.push_str(&format!(
            " {} \"s_{index}\" key=\"{}\" owner=\"d_{owner_index}\"\n",
            address.kind().as_str(),
            address.key()
        ));
    }
    text.push_str("}\n");
    let symbols = load_names(&text, schema, Default::default()).unwrap();
    (text, symbols)
}

// The source payload can only be extracted from actual checked owners. Loading
// the assembled artifacts also exercises the public reader used by consumers.
fn authored_evaluation(input: &OwnedReleaseInput) -> Option<OwnedReleaseEvaluationContent> {
    input.evaluation.as_ref()?;
    let staged = assemble_owned_release(input.clone(), Default::default()).unwrap();
    load_owned_release_artifacts(&files(&staged), Default::default())
        .unwrap()
        .into_evaluation()
        .map(OwnedReleaseEvaluationContent::from_checked)
}

struct DslRelease {
    source: Sources,
    expected: OwnedReleaseInput,
    packets: Vec<String>,
    expanded_bytes: usize,
    components: usize,
}

impl DslRelease {
    fn new(input: &OwnedReleaseInput, compressed_components: bool) -> Self {
        Self::with_evaluation_content(input, authored_evaluation(input), compressed_components)
    }

    fn with_evaluation_content(
        input: &OwnedReleaseInput,
        evaluation: Option<OwnedReleaseEvaluationContent>,
        compressed_components: bool,
    ) -> Self {
        let schema =
            OwnedDefinitionSchemaPackage::new(input.recipe.schema.clone(), Default::default())
                .unwrap();
        let (names, symbols) = fixture_names(&schema);
        let mut expected = input.clone();
        let mut bodies = BTreeMap::<String, String>::new();
        let mut packet_texts = Vec::new();
        let mut expanded_bytes = 0;
        for (index, owner) in input.recipe.rules.owners.iter().enumerate() {
            if owner.programs.members.is_empty() {
                continue;
            }
            let (kind, name) = match &owner.owner {
                SchemaSubject::Definition(address) => (
                    "definition",
                    symbols
                        .definitions()
                        .find(|(_, target)| *target == address)
                        .unwrap()
                        .0,
                ),
                SchemaSubject::Slot(address) => (
                    "slot",
                    symbols
                        .slots()
                        .find(|(_, target)| *target == address)
                        .unwrap()
                        .0,
                ),
            };
            let mut packet =
                format!("packet \"fixture-{index}\" {{\n append {kind}=\"{name}\" {{\n");
            for (program_index, program) in owner.programs.members.iter().enumerate() {
                let raised = raise_program(program, &symbols, Default::default()).unwrap();
                let parsed = parse_program(&raised.source, Default::default()).unwrap();
                let lowered = lower_program(
                    &parsed,
                    &schema,
                    &symbols,
                    &input.recipe.rules.tables,
                    Default::default(),
                )
                .unwrap();
                expected.recipe.rules.owners[index].programs.members[program_index] =
                    lowered.program;
                expanded_bytes += raised.source.len();
                let next = bodies.len();
                let path = bodies
                    .entry(raised.source)
                    .or_insert_with(|| format!("rules/source-{next}.rule"));
                packet.push_str(&format!("  program \"{}\" source=\"{path}\"\n", program.id));
            }
            packet.push_str(" }\n}\n");
            packet_texts.push((format!("rules/packet-{index}.kdl"), packet));
        }
        expected.evaluation = evaluation.as_ref().map(|content| {
            let recipe =
                CheckedOwnedRecipe::new(expected.recipe.clone(), Default::default()).unwrap();
            content
                .clone()
                .into_input(&recipe, Default::default())
                .unwrap()
        });
        let mut metadata = expected.clone();
        for owner in &mut metadata.recipe.rules.owners {
            owner.programs.members.clear();
        }
        let mut source =
            Sources::with_evaluation_content(&metadata, evaluation.as_ref(), compressed_components);
        let components = source.files.len();
        source.add_plain("rules/names.kdl", names);
        let mut packets = Vec::new();
        for (path, packet) in packet_texts {
            source.declare(&format!(
                " rule-packet \"{path}\" names=\"rules/names.kdl\"\n"
            ));
            source.add_plain(&path, packet);
            packets.push(path);
        }
        for (text, path) in bodies {
            source.declare(&format!(" rule-source \"{path}\"\n"));
            source.add_plain(&path, text);
        }
        Self {
            source,
            expected,
            packets,
            expanded_bytes,
            components,
        }
    }

    fn source_for(&self, id: &str) -> String {
        let header = format!("program \"{id}\" ");
        self.source
            .files
            .iter()
            .find(|(path, bytes)| {
                path.ends_with(".rule") && std::str::from_utf8(bytes).unwrap().starts_with(&header)
            })
            .unwrap()
            .0
            .clone()
    }
}

#[test]
fn declared_kdl_and_dsl_build_a_complete_release_with_the_independent_metric() {
    for compressed in [false, true] {
        let fixture = DslRelease::new(&fixture().input, compressed);
        let expected =
            assemble_owned_release(fixture.expected.clone(), Default::default()).unwrap();
        let manifest =
            parse_release_manifest(&fixture.source.manifest, Default::default()).unwrap();
        assert_eq!(manifest.sources().len(), fixture.source.files.len());
        assert_eq!(
            manifest
                .sources()
                .iter()
                .filter(|source| source.path() == "rules/names.kdl")
                .count(),
            1
        );
        let mut sources = fixture.source.sources();
        sources.reverse();
        let actual = build_release(
            &manifest,
            &sources,
            ReleaseAllocationBase::Bootstrap,
            Default::default(),
        )
        .unwrap();
        assert!(actual.name_allocation().predecessor().is_none());
        assert!(actual.name_allocation().names().input().bindings.is_empty());
        assert_eq!(actual.release().input(), expected.input());
        assert_eq!(files(actual.release()), files(&expected));
        assert_eq!(actual.encoded_bytes(), fixture.source.encoded_bytes());
        assert_eq!(actual.decoded_bytes(), fixture.source.decoded_bytes);
        assert_dsl_metric(actual.release());
    }
}

#[test]
fn dsl_sources_and_names_have_exact_inventory_and_cannot_borrow_component_roles() {
    let fixture = DslRelease::new(&fixture().input, false);
    let manifest = parse_release_manifest(&fixture.source.manifest, Default::default()).unwrap();
    let sources = fixture.source.sources();
    let program_path = fixture.source_for("action-final");
    for path in [
        "rules/names.kdl",
        fixture.packets[0].as_str(),
        program_path.as_str(),
    ] {
        let missing: Vec<_> = sources
            .iter()
            .copied()
            .filter(|source| source.path != path)
            .collect();
        assert!(
            build_release(
                &manifest,
                &missing,
                ReleaseAllocationBase::Bootstrap,
                Default::default()
            )
            .is_err(),
            "missing {path}"
        );
    }
    let mut extra = sources;
    extra.push(ReleaseSource {
        path: "rules/extra.rule",
        bytes: b"program \"extra\" context actor {}",
    });
    assert!(
        build_release(
            &manifest,
            &extra,
            ReleaseAllocationBase::Bootstrap,
            Default::default()
        )
        .is_err()
    );

    let mut unused = fixture.source.clone();
    unused.declare(" rule-source \"rules/unused.rule\"\n");
    unused.add_plain(
        "rules/unused.rule",
        "program \"unused\" context actor {}".into(),
    );
    assert!(
        unused.build(Default::default()).is_err(),
        "declared but unused rule source"
    );

    let packet = &fixture.packets[0];
    let first_path = fixture
        .source
        .text(packet)
        .split("source=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    for target in [
        "rules/not-declared.rule",
        "schema.json",
        "rules/names.kdl",
        packet.as_str(),
    ] {
        let mut changed = fixture.source.clone();
        changed.replace_plain(
            packet,
            fixture.source.text(packet).replace(
                &format!("source=\"{first_path}\""),
                &format!("source=\"{target}\""),
            ),
        );
        assert!(
            changed.build(Default::default()).is_err(),
            "packet borrowed {target}"
        );
    }
    for declaration in [
        " rule-source \"schema.json\"\n",
        " rule-source \"rules/names.kdl\"\n",
        " rule-packet \"schema.json\" names=\"rules/names.kdl\"\n",
    ] {
        let mut changed = fixture.source.clone();
        changed.declare(declaration);
        assert!(
            parse_release_manifest(&changed.manifest, Default::default()).is_err(),
            "{declaration}"
        );
    }
}

#[test]
fn packets_refuse_raw_programs_and_never_fall_back_after_dsl_errors() {
    for compressed in [false, true] {
        invalid(
            Sources::new(&fixture().input, compressed).build(Default::default()),
            "release requires a program-free rules constituent",
        );
    }
    let fixture = DslRelease::new(&fixture().input, false);
    let mut mixed = fixture.source.clone();
    mixed.replace_plain(
        "rules.json",
        serde_json::to_string(&fixture.expected.recipe.rules).unwrap(),
    );
    invalid(
        mixed.build(Default::default()),
        "release requires a program-free rules constituent",
    );

    let path = fixture.source_for("action-final");
    let text = fixture.source.text(&path);
    let measurement = text
        .lines()
        .find(|line| line.contains("effect \"measurement\""))
        .unwrap();
    assert!(measurement.contains("derive(current, "));
    let stat = measurement
        .split("derive(current, ")
        .nth(1)
        .unwrap()
        .split(',')
        .next()
        .unwrap();
    let unit_name = fixture
        .source
        .text("rules/names.kdl")
        .lines()
        .find(|line| line.trim_start().starts_with("unit "))
        .unwrap()
        .split('"')
        .nth(1)
        .unwrap();
    for (target, expected) in [
        (
            "missing_stat",
            poe_optimizer_authoring::DiagnosticCode::UnknownSymbol,
        ),
        (
            unit_name,
            poe_optimizer_authoring::DiagnosticCode::TypeMismatch,
        ),
    ] {
        let mut changed = fixture.source.clone();
        let bad = measurement.replace(
            &format!("derive(current, {stat},"),
            &format!("derive(current, {target},"),
        );
        changed.replace_plain(&path, text.replace(measurement, &bad));
        match changed.build(Default::default()) {
            Err(ReleaseBuildError::Packet { source, .. }) => {
                assert_eq!(source.diagnostic.unwrap().code, expected)
            }
            Err(other) => panic!("wrong rejection for {bad}: {other}"),
            Ok(_) => panic!("accepted {bad}"),
        }
    }

    let value = measurement
        .rsplit(", ")
        .next()
        .unwrap()
        .trim_end_matches(");");
    let binding = text
        .lines()
        .find(|line| line.trim_start().starts_with(&format!("let {value} = ")))
        .unwrap();
    for invalid in [
        text.replace(
            measurement,
            &measurement.replace("derive(current, ", "derive(modifier, "),
        ),
        text.replace(binding, &format!("    let {value} = true;")),
    ] {
        let mut unauthorized = fixture.source.clone();
        unauthorized.replace_plain(&path, invalid);
        assert!(matches!(
            unauthorized.build(Default::default()),
            Err(ReleaseBuildError::Assembly(OwnedReleaseError::Recipe(
                poe_optimizer_import::owned_recipe::OwnedRecipeError::Compile(_)
            )))
        ));
    }

    // A syntactically valid alias still has to resolve against the exact schema.
    let mut changed = fixture.source.clone();
    let names = fixture.source.text("rules/names.kdl");
    let corrupt = names.replacen("key=\"def.", "key=\"unknown.", 1);
    assert_ne!(names, corrupt);
    changed.replace_plain("rules/names.kdl", corrupt);
    assert!(changed.build(Default::default()).is_err());
}

#[test]
fn release_packets_preserve_duplicate_membership_and_source_header_rejections() {
    let fixture = DslRelease::new(&fixture().input, false);
    let duplicate_path = "rules/duplicate-packet.kdl";
    let packet = fixture.source.text(&fixture.packets[0]);
    let duplicate = packet.replacen("packet \"fixture-", "packet \"duplicate-fixture-", 1);
    assert_ne!(packet, duplicate);
    let mut changed = fixture.source.clone();
    changed.declare(&format!(
        " rule-packet \"{duplicate_path}\" names=\"rules/names.kdl\"\n"
    ));
    changed.add_plain(duplicate_path, duplicate);
    match changed.build(Default::default()) {
        Err(ReleaseBuildError::Packet { path, source }) => {
            assert_eq!(path, duplicate_path);
            assert_eq!(
                source.kind,
                poe_optimizer_authoring::PacketErrorKind::DuplicateProgram
            );
            assert_eq!(
                source.location,
                poe_optimizer_authoring::PacketLocation::Member {
                    append: 0,
                    program: 0,
                }
            );
        }
        Err(other) => panic!("wrong duplicate-program rejection: {other}"),
        Ok(_) => panic!("accepted duplicate owner-qualified programs across packets"),
    }

    let source_path = fixture.source_for("action-final");
    let packet_path = fixture
        .packets
        .iter()
        .find(|path| {
            fixture
                .source
                .text(path)
                .contains(&format!("source=\"{source_path}\""))
        })
        .unwrap();
    let original = fixture.source.text(&source_path);
    let mismatched = original.replacen(
        "program \"action-final\" ",
        "program \"different-header\" ",
        1,
    );
    assert_ne!(original, mismatched);
    let mut changed = fixture.source.clone();
    changed.replace_plain(&source_path, mismatched);
    match changed.build(Default::default()) {
        Err(ReleaseBuildError::Packet { path, source }) => {
            assert_eq!(&path, packet_path);
            assert_eq!(
                source.kind,
                poe_optimizer_authoring::PacketErrorKind::SourceProgramMismatch
            );
        }
        Err(other) => panic!("wrong source-header rejection: {other}"),
        Ok(_) => panic!("accepted a source header that differs from packet membership"),
    }
}

#[test]
fn composed_rules_reject_sealed_evaluation_sources_and_require_expected_release_identity() {
    let dsl = DslRelease::new(&fixture().input, false);
    let expected = assemble_owned_release(dsl.expected.clone(), Default::default()).unwrap();
    let mut bound = dsl.source.clone();
    bound.manifest = bound.manifest.replacen(
        "release {",
        &format!("release expected_input=\"{}\" {{", expected.receipt().input),
        1,
    );
    assert_eq!(
        bound.build(Default::default()).unwrap().release().receipt(),
        expected.receipt()
    );
    bound.manifest = bound
        .manifest
        .replace(&expected.receipt().input.to_string(), &"0".repeat(64));
    invalid(
        bound.build(Default::default()),
        "assembled input identity differs from expected_input",
    );

    let original = fixture().input.evaluation.as_ref().unwrap();
    let support = original.support.as_ref().unwrap();
    let original_stages = &support.stages;
    assert_ne!(
        original_stages.rules,
        dsl.expected
            .evaluation
            .as_ref()
            .unwrap()
            .support
            .as_ref()
            .unwrap()
            .stages
            .rules
    );
    for (path, sealed) in [
        (
            "metrics.json",
            serde_json::to_string(&original.metrics).unwrap(),
        ),
        (
            "support-stages.json",
            serde_json::to_string(&support.stages).unwrap(),
        ),
        (
            "support-preparation.json",
            serde_json::to_string(&support.preparation).unwrap(),
        ),
        (
            "support-inputs.json",
            serde_json::to_string(&support.inputs).unwrap(),
        ),
        (
            "support-receiving.json",
            serde_json::to_string(&support.receiving).unwrap(),
        ),
        (
            "support-outputs.json",
            serde_json::to_string(support.outputs.as_ref().unwrap()).unwrap(),
        ),
    ] {
        let mut stale = dsl.source.clone();
        stale.replace_plain(path, sealed);
        assert!(
            matches!(
                stale.build(Default::default()),
                Err(ReleaseBuildError::Json { path: actual, .. }) if actual == path
            ),
            "sealed source {path}"
        );
    }
}

#[test]
fn authored_evaluation_reserves_the_source_validation_budget_before_source_checks() {
    use poe_optimizer_import::owned_release::check_release_source_dependencies;

    let dsl = DslRelease::new(&fixture().input, false);
    let evaluation = authored_evaluation(&fixture().input).unwrap();
    let limits = ReleaseBuildLimits::default();
    let mut remaining = limits.release.max_validation_entries;
    evaluation
        .charge_validation_entries(limits.release, &mut remaining)
        .unwrap();
    let evaluation_entries = limits.release.max_validation_entries - remaining;
    assert!(evaluation_entries > 0);

    let mut shell = dsl.expected.clone();
    shell.schema_version = OWNED_RELEASE_VERSION;
    shell.evaluation = None;
    for owner in &mut shell.recipe.rules.owners {
        owner.programs.members.clear();
    }
    let source_check = |max_validation_entries| {
        check_release_source_dependencies(
            &shell,
            poe_optimizer_import::owned_release::OwnedReleaseLimits {
                max_validation_entries,
                ..limits.release
            },
        )
    };
    // Calibrate the source-only prefix independently of the builder's aggregate
    // charge. Only the budget changes; every original dependency remains valid.
    let (mut low, mut high) = (1, limits.release.max_validation_entries);
    while low < high {
        let middle = low + (high - low) / 2;
        match source_check(middle) {
            Ok(_) => high = middle,
            Err(OwnedReleaseError::Limit("validation entries")) => low = middle + 1,
            Err(error) => panic!("unexpected source check: {error}"),
        }
    }
    let source_entries = low;
    assert!(source_entries > 1);
    source_check(source_entries).unwrap();
    assert!(matches!(
        source_check(source_entries - 1),
        Err(OwnedReleaseError::Limit("validation entries"))
    ));

    // A stale original rule header is rejected after aggregate preflight. At
    // one-short budget the earlier failure proves the evaluation reservation
    // already constrained source checking, before any final assembly can run.
    let mut source = dsl.source;
    let mut rules: RulePackageInput = serde_json::from_str(source.text("rules.json")).unwrap();
    rules.definitions.content_sha256 = "1".repeat(64);
    source.replace_plain("rules.json", serde_json::to_string(&rules).unwrap());
    let with_budget = |max_validation_entries| ReleaseBuildLimits {
        release: poe_optimizer_import::owned_release::OwnedReleaseLimits {
            max_validation_entries,
            ..limits.release
        },
        ..limits
    };
    assert!(matches!(
        source.build(with_budget(source_entries + evaluation_entries)),
        Err(ReleaseBuildError::Assembly(OwnedReleaseError::Recipe(
            poe_optimizer_import::owned_recipe::OwnedRecipeError::Rules(
                poe_optimizer_data::owned_rules::RuleStorageError::Binding
            )
        )))
    ));
    for budget in [source_entries + evaluation_entries - 1, evaluation_entries] {
        assert!(
            matches!(
                source.build(with_budget(budget)),
                Err(ReleaseBuildError::Assembly(OwnedReleaseError::Limit(
                    "validation entries"
                )))
            ),
            "combined budget {budget}"
        );
    }
}

#[test]
fn shared_rule_sources_charge_each_occurrence_without_coalescing_distinct_bodies() {
    let mut input = fixture().input.clone();
    let mut evaluation = authored_evaluation(&input).unwrap();
    let candidates: Vec<_> = input
        .recipe
        .rules
        .owners
        .iter()
        .enumerate()
        .filter_map(|(index, owner)| {
            owner
                .programs
                .members
                .first()
                .map(|program| (index, program.context))
        })
        .collect();
    let (first, second, context) = candidates
        .iter()
        .find_map(|(first, context)| {
            candidates
                .iter()
                .find(|(second, other)| second != first && other == context)
                .map(|(second, _)| (*first, *second, *context))
        })
        .unwrap();
    let literal = |id: &str, value| RuleProgram {
        id: OwnedDefinitionKey::new(id).unwrap(),
        context,
        reads: vec![],
        nodes: vec![RuleNode {
            id: OwnedDefinitionKey::new("marker").unwrap(),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(value),
            },
        }],
        effects: vec![],
    };
    for index in [first, second] {
        let owner = &input.recipe.rules.owners[index];
        let stages = &mut evaluation.support.as_mut().unwrap().stages.programs.members;
        let placement = stages
            .iter()
            .find(|row| row.owner == owner.owner && row.program == owner.programs.members[0].id)
            .unwrap()
            .clone();
        for id in ["shared-marker", "distinct-marker"] {
            let mut marker = placement.clone();
            marker.program = OwnedDefinitionKey::new(id).unwrap();
            stages.push(marker);
        }
        input.recipe.rules.owners[index]
            .programs
            .members
            .push(literal("shared-marker", true));
        input.recipe.rules.owners[index]
            .programs
            .members
            .push(literal("distinct-marker", index == first));
    }
    let fixture = DslRelease::with_evaluation_content(&input, Some(evaluation), false);
    let source_bodies: Vec<_> = fixture
        .source
        .files
        .iter()
        .filter(|(path, _)| path.ends_with(".rule"))
        .map(|(_, bytes)| std::str::from_utf8(bytes).unwrap())
        .collect();
    assert_eq!(
        source_bodies
            .iter()
            .filter(|text| text.starts_with("program \"shared-marker\" "))
            .count(),
        1
    );
    assert_eq!(
        source_bodies
            .iter()
            .filter(|text| text.starts_with("program \"distinct-marker\" "))
            .count(),
        2
    );
    let actual = fixture.source.build(Default::default()).unwrap();
    // Stage rows are an unordered declaration inventory. Compare through the
    // same canonical constituent constructors as the packet-free expected path;
    // its program bodies were lowered independently of packet composition.
    let expected = assemble_owned_release(fixture.expected.clone(), Default::default()).unwrap();
    assert_eq!(actual.release().input(), expected.input());
    assert_dsl_metric(actual.release());
    assert!(fixture.expanded_bytes > source_bodies.iter().map(|text| text.len()).sum::<usize>());
    let exact = ReleaseBuildLimits {
        max_expanded_rule_bytes: fixture.expanded_bytes,
        ..Default::default()
    };
    assert!(fixture.source.build(exact).is_ok());
    assert!(matches!(
        fixture.source.build(ReleaseBuildLimits {
            max_expanded_rule_bytes: fixture.expanded_bytes - 1,
            ..exact
        }),
        Err(ReleaseBuildError::Limit(_))
    ));
}

#[test]
fn aggregate_packet_and_ir_budgets_apply_across_the_complete_release() {
    let fixture = DslRelease::new(&fixture().input, false);
    assert!(fixture.packets.len() > 1);
    let limits = ReleaseBuildLimits {
        max_components: fixture.components,
        max_rule_packets: fixture.packets.len(),
        max_sources: fixture.source.files.len(),
        max_expanded_rule_bytes: fixture.expanded_bytes,
        max_encoded_bytes: fixture.source.encoded_bytes(),
        max_decoded_bytes: fixture.source.decoded_bytes,
        ..Default::default()
    };
    assert!(fixture.source.build(limits).is_ok());
    let manifest = parse_release_manifest(&fixture.source.manifest, limits).unwrap();
    let supplied = fixture.source.sources();
    for (stricter, message) in [
        (
            ReleaseBuildLimits {
                max_components: limits.max_components - 1,
                ..limits
            },
            "component count",
        ),
        (
            ReleaseBuildLimits {
                max_rule_packets: limits.max_rule_packets - 1,
                ..limits
            },
            "rule packet count",
        ),
    ] {
        limited(
            build_release(
                &manifest,
                &supplied,
                ReleaseAllocationBase::Bootstrap,
                stricter,
            ),
            message,
        );
    }
    for smaller in [
        ReleaseBuildLimits {
            max_components: limits.max_components - 1,
            ..limits
        },
        ReleaseBuildLimits {
            max_rule_packets: limits.max_rule_packets - 1,
            ..limits
        },
        ReleaseBuildLimits {
            max_sources: limits.max_sources - 1,
            ..limits
        },
        ReleaseBuildLimits {
            max_encoded_bytes: limits.max_encoded_bytes - 1,
            ..limits
        },
        ReleaseBuildLimits {
            max_decoded_bytes: limits.max_decoded_bytes - 1,
            ..limits
        },
        ReleaseBuildLimits {
            max_expanded_rule_bytes: limits.max_expanded_rule_bytes - 1,
            ..limits
        },
    ] {
        assert!(matches!(
            fixture.source.build(smaller),
            Err(ReleaseBuildError::Limit(_))
        ));
    }
    let programs: Vec<_> = fixture
        .expected
        .recipe
        .rules
        .owners
        .iter()
        .flat_map(|owner| &owner.programs.members)
        .collect();
    let totals = [
        programs.len(),
        programs.iter().map(|program| program.reads.len()).sum(),
        programs.iter().map(|program| program.nodes.len()).sum(),
        programs.iter().map(|program| program.effects.len()).sum(),
    ];
    for (field, total) in totals.into_iter().enumerate() {
        assert!(total > 1);
        let mut exact = limits;
        let bound = match field {
            0 => &mut exact.release.recipe.compile.max_programs,
            1 => &mut exact.release.recipe.compile.max_reads,
            2 => &mut exact.release.recipe.compile.max_nodes,
            _ => &mut exact.release.recipe.compile.max_effects,
        };
        *bound = total;
        assert!(fixture.source.build(exact).is_ok());
        match field {
            0 => exact.release.recipe.compile.max_programs -= 1,
            1 => exact.release.recipe.compile.max_reads -= 1,
            2 => exact.release.recipe.compile.max_nodes -= 1,
            _ => exact.release.recipe.compile.max_effects -= 1,
        }
        assert!(matches!(
            fixture.source.build(exact),
            Err(ReleaseBuildError::Limit(_))
        ));
    }
}

#[test]
fn plain_and_gzip_constituents_build_the_same_dsl_release_and_metric() {
    for compressed in [false, true] {
        let dsl = DslRelease::new(&fixture().input, compressed);
        let expected = assemble_owned_release(dsl.expected, Default::default()).unwrap();
        let source = dsl.source;
        let manifest = parse_release_manifest(&source.manifest, Default::default()).unwrap();
        assert!(manifest.sources().iter().all(|spec| spec.encoding()
            == if compressed && spec.path().ends_with(".json.gz") {
                SourceEncoding::Gzip
            } else {
                SourceEncoding::Plain
            }));
        let mut supplied = source.sources();
        supplied.reverse();
        let actual = build_release(
            &manifest,
            &supplied,
            ReleaseAllocationBase::Bootstrap,
            Default::default(),
        )
        .unwrap();
        assert_eq!(actual.encoded_bytes(), source.encoded_bytes());
        assert_eq!(actual.decoded_bytes(), source.decoded_bytes);
        assert_eq!(actual.release().input(), expected.input());
        assert_eq!(actual.release().receipt(), expected.receipt());
        assert_eq!(files(actual.release()), files(&expected));
        assert_dsl_metric(actual.release());
    }
}

#[test]
fn optional_evaluation_groups_are_explicit_and_complete() {
    for mode in 0..3 {
        let mut input = fixture().input.clone();
        match mode {
            0 => {
                input.evaluation = None;
                input.schema_version = OWNED_RELEASE_VERSION;
            }
            1 => input.evaluation.as_mut().unwrap().support = None,
            _ => {
                input
                    .evaluation
                    .as_mut()
                    .unwrap()
                    .support
                    .as_mut()
                    .unwrap()
                    .outputs = None
            }
        }
        let dsl = DslRelease::new(&input, false);
        let expected = assemble_owned_release(dsl.expected, Default::default()).unwrap();
        assert_eq!(
            files(dsl.source.build(Default::default()).unwrap().release()),
            files(&expected)
        );
    }
    let source = DslRelease::new(&fixture().input, false).source;
    for component in [
        "metrics",
        "support-stages",
        "support-preparation",
        "support-inputs",
        "support-receiving",
    ] {
        let text = source
            .manifest
            .lines()
            .filter(|line| !line.trim_start().starts_with(&format!("{component} ")))
            .collect::<Vec<_>>()
            .join("\n");
        invalid(
            parse_release_manifest(&text, Default::default()),
            "incomplete support/evaluation component group",
        );
    }
}

#[test]
fn supplied_source_inventory_is_exact_without_fallbacks() {
    let source = DslRelease::new(&fixture().input, false).source;
    let manifest = parse_release_manifest(&source.manifest, Default::default()).unwrap();
    let supplied = source.sources();
    invalid(
        build_release(
            &manifest,
            &supplied[1..],
            ReleaseAllocationBase::Bootstrap,
            Default::default(),
        ),
        "source inventory differs",
    );
    let mut changed = supplied.clone();
    changed.push(ReleaseSource {
        path: "extra.json",
        bytes: b"{}",
    });
    invalid(
        build_release(
            &manifest,
            &changed,
            ReleaseAllocationBase::Bootstrap,
            Default::default(),
        ),
        "source inventory differs",
    );
    let mut changed = supplied.clone();
    changed[0] = changed[1];
    invalid(
        build_release(
            &manifest,
            &changed,
            ReleaseAllocationBase::Bootstrap,
            Default::default(),
        ),
        "duplicate supplied source",
    );
    let mut changed = supplied.clone();
    changed[0].path = "unlisted.json";
    invalid(
        build_release(
            &manifest,
            &changed,
            ReleaseAllocationBase::Bootstrap,
            Default::default(),
        ),
        "missing declared source",
    );
    let mut changed = supplied;
    changed[0].path = "../outside.json";
    invalid(
        build_release(
            &manifest,
            &changed,
            ReleaseAllocationBase::Bootstrap,
            Default::default(),
        ),
        "invalid supplied source path",
    );
}

#[test]
fn encoded_decoded_and_source_count_budgets_have_exact_boundaries() {
    for compressed in [false, true] {
        let source = DslRelease::new(&fixture().input, compressed).source;
        let limits = ReleaseBuildLimits {
            max_encoded_bytes: source.encoded_bytes(),
            max_decoded_bytes: source.decoded_bytes,
            max_sources: source.files.len(),
            ..Default::default()
        };
        let exact = source.build(limits).unwrap();
        assert_eq!(exact.encoded_bytes(), limits.max_encoded_bytes);
        assert_eq!(exact.decoded_bytes(), limits.max_decoded_bytes);
        limited(
            source.build(ReleaseBuildLimits {
                max_encoded_bytes: limits.max_encoded_bytes - 1,
                ..limits
            }),
            "encoded bytes",
        );
        limited(
            source.build(ReleaseBuildLimits {
                max_decoded_bytes: limits.max_decoded_bytes - 1,
                ..limits
            }),
            "decoded bytes",
        );
        limited(
            parse_release_manifest(
                &source.manifest,
                ReleaseBuildLimits {
                    max_sources: limits.max_sources - 1,
                    ..limits
                },
            ),
            "source count",
        );
        let manifest = parse_release_manifest(&source.manifest, limits).unwrap();
        limited(
            build_release(
                &manifest,
                &source.sources(),
                ReleaseAllocationBase::Bootstrap,
                ReleaseBuildLimits {
                    max_sources: limits.max_sources - 1,
                    ..limits
                },
            ),
            "source count",
        );
    }
    let text = DslRelease::new(&fixture().input, false).source.manifest;
    for limits in [
        ReleaseBuildLimits {
            max_sources: 0,
            ..Default::default()
        },
        ReleaseBuildLimits {
            max_encoded_bytes: 0,
            ..Default::default()
        },
        ReleaseBuildLimits {
            max_decoded_bytes: 0,
            ..Default::default()
        },
        ReleaseBuildLimits {
            max_encoded_bytes: ReleaseBuildLimits::default().max_encoded_bytes + 1,
            ..Default::default()
        },
    ] {
        invalid(
            parse_release_manifest(&text, limits),
            "invalid release source limits",
        );
    }
}

#[test]
fn gzip_requires_one_complete_member_without_trailing_data() {
    for case in 0..5 {
        let mut source = DslRelease::new(&fixture().input, true).source;
        let (path, bytes) = source
            .files
            .iter_mut()
            .find(|(path, _)| path == "metrics.json.gz")
            .unwrap();
        let expected_path = path.clone();
        match case {
            0 => {
                let crc = bytes.len() - 8;
                bytes[crc] ^= 1;
            }
            1 => {
                bytes.truncate(bytes.len() - 1);
            }
            2 => bytes.extend_from_slice(b"trailing"),
            3 => bytes.extend(gzip(b"{}")),
            _ => bytes.clear(),
        }
        match source.build(Default::default()) {
            Err(ReleaseBuildError::Compression { path, .. })
                if case == 0 || case == 1 || case == 4 =>
            {
                assert_eq!(path, expected_path)
            }
            Err(ReleaseBuildError::Invalid(reason)) if case == 2 || case == 3 => assert_eq!(
                reason,
                "gzip source contains trailing bytes or another member"
            ),
            Err(other) => panic!("case {case}: {other}"),
            Ok(_) => panic!("case {case} unexpectedly succeeded"),
        }
    }
}

#[test]
fn constituent_json_is_strict_and_reports_its_source() {
    for compressed in [false, true] {
        for suffix in [b" {}".as_slice(), b" trailing".as_slice()] {
            let mut source = DslRelease::new(&fixture().input, compressed).source;
            let target = if compressed {
                "metrics.json.gz"
            } else {
                "metrics.json"
            };
            let (_, bytes) = source
                .files
                .iter_mut()
                .find(|(path, _)| path == target)
                .unwrap();
            let mut json = if compressed {
                let mut json = Vec::new();
                GzDecoder::new(bytes.as_slice())
                    .read_to_end(&mut json)
                    .unwrap();
                json
            } else {
                bytes.clone()
            };
            json.extend_from_slice(suffix);
            *bytes = if compressed { gzip(&json) } else { json };
            match source.build(Default::default()) {
                Err(ReleaseBuildError::Json { path, .. }) => assert_eq!(path, target),
                Err(other) => panic!("unexpected error: {other}"),
                Ok(_) => panic!("trailing JSON was accepted"),
            }
        }
    }
}

#[test]
fn manifest_rejects_paths_unknown_fields_and_duplicate_components_or_queries() {
    let source = DslRelease::new(&fixture().input, false).source;
    for path in [
        "",
        "/absolute.json",
        "../escape.json",
        "nested/../escape.json",
        "./same.json",
        "a//b.json",
        "a/",
        "C:/outside.json",
    ] {
        let text = source
            .manifest
            .replace("\"schema.json\"", &format!("\"{path}\""));
        invalid(
            parse_release_manifest(&text, Default::default()),
            "source must be a portable relative path",
        );
    }
    for (old, new, reason) in [
        (
            "release {",
            "release extra=\"no\" {",
            "invalid release root fields",
        ),
        (
            "schema \"schema.json\" encoding=\"plain\"",
            "schema \"schema.json\" encoding=\"zstd\"",
            "unknown source encoding",
        ),
        (
            "schema \"schema.json\" encoding=\"plain\"",
            "schema \"schema.json\" encoding=\"json\"",
            "unknown source encoding",
        ),
        (
            "schema \"schema.json\"",
            "schema \"schema.json\" extra=\"no\"",
            "invalid source declaration fields",
        ),
        (
            "schema \"schema.json\"",
            "unknown \"schema.json\"",
            "unknown source component",
        ),
        (
            "schema \"schema.json\"",
            "schema \"registry.json\"",
            "source path used more than once",
        ),
        (
            "}\n",
            "schema \"second-schema.json\"\n}\n",
            "duplicate source component",
        ),
        (
            "}\n",
            "queries \"same\" source=\"one.json\"\nqueries \"same\" source=\"two.json\"\n}\n",
            "invalid or duplicate query set name",
        ),
        (
            "}\n",
            "queries \"Upper\" source=\"one.json\"\n}\n",
            "invalid or duplicate query set name",
        ),
        (
            "}\n",
            "queries \"-bad\" source=\"one.json\"\n}\n",
            "invalid query set key",
        ),
        (
            "}\n",
            "queries \"_bad\" source=\"one.json\"\n}\n",
            "invalid query set key",
        ),
        ("}\n", "queries \"valid\"\n}\n", "queries require a source"),
    ] {
        invalid(
            parse_release_manifest(&source.manifest.replace(old, new), Default::default()),
            reason,
        );
    }
    let text = source
        .manifest
        .lines()
        .filter(|line| !line.trim_start().starts_with("schema "))
        .collect::<Vec<_>>()
        .join("\n");
    invalid(
        parse_release_manifest(&text, Default::default()),
        "missing required source component",
    );
    let text = source.manifest.replace(
        "}\n",
        "queries \"one\" source=\"one.json\"\nqueries \"two\" source=\"two.json\"\n}\n",
    );
    let mut limits = ReleaseBuildLimits::default();
    limits.release.max_query_sets = 1;
    limited(parse_release_manifest(&text, limits), "query sets");
}

#[test]
fn expected_identity_and_actual_metric_authority_are_checked_after_decode() {
    let dsl = DslRelease::new(&fixture().input, false);
    let expected = assemble_owned_release(dsl.expected, Default::default()).unwrap();
    let mut source = dsl.source;
    source.manifest = source.manifest.replacen(
        "release {",
        &format!("release expected_input=\"{}\" {{", expected.receipt().input),
        1,
    );
    let manifest = parse_release_manifest(&source.manifest, Default::default()).unwrap();
    assert_eq!(manifest.expected_input(), Some(expected.receipt().input));
    assert_eq!(
        source
            .build(Default::default())
            .unwrap()
            .release()
            .receipt(),
        expected.receipt()
    );
    source.manifest = source
        .manifest
        .replace(&expected.receipt().input.to_string(), &"0".repeat(64));
    invalid(
        source.build(Default::default()),
        "assembled input identity differs from expected_input",
    );

    let mut source = DslRelease::new(&fixture().input, false).source;
    let mut metrics: poe_optimizer_data::owned_metrics::MetricMappingContent =
        serde_json::from_str(source.text("metrics.json")).unwrap();
    metrics.bindings[0].metric = DefId::new(
        fixture().input.recipe.schema.namespace.clone(),
        OwnedDefinitionKey::new("missing-metric").unwrap(),
    );
    source.replace_plain("metrics.json", serde_json::to_string(&metrics).unwrap());
    match source.build(Default::default()) {
        Err(ReleaseBuildError::Assembly(OwnedReleaseError::Evaluation(_))) => {}
        Err(other) => panic!("unexpected error: {other}"),
        Ok(_) => panic!("an unregistered metric bypassed the existing checker"),
    }
}

#[test]
fn tracked_dsl_release_preserves_historical_content_and_only_reviewed_additions() {
    let root = tracked_root();
    let source = tracked_sources();
    let reference_bytes = fs::read(root.join("reference-release.json")).unwrap();
    let reference: OwnedReleaseReceipt = serde_json::from_slice(&reference_bytes).unwrap();
    let manifest = parse_release_manifest(&source.manifest, Default::default()).unwrap();
    let owned = &source.files;
    let actual = tracked_release();
    let release = actual.release();
    assert_eq!(manifest.expected_input(), Some(release.receipt().input));
    assert_ne!(release.receipt().input, reference.input);
    let current_artifacts: BTreeMap<_, _> = release.artifacts().collect();

    // Historical IR remains a test reference only. Authenticate the exact new
    // producer, 69 Boolean channels and 350 support-source domains before
    // reconstructing old commitments through maintained APIs.
    let retained = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/retained-rules.json.gz"),
    )
    .unwrap();
    let mut bytes = Vec::new();
    let maximum = ReleaseBuildLimits::default().max_decoded_bytes;
    GzDecoder::new(retained.as_slice())
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= maximum);
    let rule_reference = reference
        .artifacts
        .iter()
        .find(|row| row.file == "rules.json")
        .unwrap();
    assert_eq!(bytes.len(), rule_reference.bytes);
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        rule_reference.sha256
    );
    let historical: RulePackageInput = serde_json::from_slice(&bytes).unwrap();
    fn bootstrap<T: serde::de::DeserializeOwned>(source: &Sources, path: &str) -> T {
        let encoded = &source
            .files
            .iter()
            .find(|(name, _)| name == path)
            .unwrap()
            .1;
        let mut bytes = Vec::new();
        GzDecoder::new(encoded.as_slice())
            .take(ReleaseBuildLimits::default().max_decoded_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() <= ReleaseBuildLimits::default().max_decoded_bytes);
        serde_json::from_slice(&bytes).unwrap()
    }
    let old_schema = OwnedDefinitionSchemaPackage::new(
        bootstrap(source, "bootstrap/schema.json.gz"),
        Default::default(),
    )
    .unwrap();
    let old_registry = poe_optimizer_import::owned_mapping::OwnedIdRegistrySnapshot::new(
        bootstrap(source, "bootstrap/registry.json.gz"),
        Default::default(),
    )
    .unwrap();
    let allocation = actual.name_allocation();
    assert!(
        allocation.predecessor().is_none(),
        "the content-test fixture bootstraps its ledger; CLI publication uses a checked predecessor"
    );
    assert_eq!(
        allocation
            .names()
            .input()
            .bindings
            .iter()
            .map(|binding| binding.name.clone())
            .collect::<BTreeSet<_>>(),
        initial_skill_types::stat_names()
    );
    let additions: Vec<_> = allocation
        .names()
        .input()
        .bindings
        .iter()
        .map(|binding| binding.target.clone())
        .collect();
    assert_eq!(additions.len(), 69);
    let mut prior_schema = release.input().recipe.schema.clone();
    let before_count = prior_schema.definitions.len();
    prior_schema
        .definitions
        .retain(|row| !additions.contains(&SchemaSubject::Definition(row.address())));
    assert_eq!(before_count - prior_schema.definitions.len(), 69);
    assert_eq!(
        &prior_schema,
        old_schema.input(),
        "all existing definitions, slots and metadata"
    );
    old_registry
        .validate_successor(allocation.names().registry().snapshot())
        .unwrap();
    let current_registry = allocation.names().registry().input();
    let previous = old_registry.input();
    assert_eq!(
        &current_registry.entries[..previous.entries.len()],
        previous.entries.as_slice()
    );
    assert_eq!(
        current_registry.entries[previous.entries.len()..]
            .iter()
            .map(|row| row.target.clone())
            .collect::<Vec<_>>(),
        additions
    );
    assert_eq!(current_registry.entries.len(), previous.entries.len() + 69);
    let parts = ReleaseSourceParts::from_checked(
        poe_optimizer_import::owned_release::check_release_source_dependencies(
            release.input(),
            Default::default(),
        )
        .unwrap(),
    )
    .unwrap();
    let mut restored = release.input().clone();
    restored.recipe.rules = parts.rules.into_input(&old_schema);
    let admitted_owner = initial_skill_types::owner();
    let mut removed = 0;
    for row in &mut restored.recipe.rules.owners {
        row.programs.members.retain(|program| {
            let admitted =
                row.owner == admitted_owner && program.id.as_str() == initial_skill_types::PROGRAM;
            removed += usize::from(admitted);
            !admitted
        });
    }
    assert_eq!(removed, 1, "exact admitted producer address");
    let domains = &mut restored
        .recipe
        .rules
        .support_discovery
        .as_mut()
        .unwrap()
        .providers;
    let prior_domains = &historical.support_discovery.as_ref().unwrap().providers;
    let additions: Vec<_> = sniper_support_domains::declarations()
        .into_iter()
        .chain(root_support_domains::declarations())
        .chain(usage_support_domains::declarations())
        .chain(attribute_support_domains::declarations())
        .chain(minion_support_domains::declarations())
        .chain(view_support_domains::declarations())
        .chain(socket_support_domains::declarations())
        .chain(gigantic_support_domains::declarations())
        .chain(structural_support_domains::declarations())
        .chain(class_support_domains::declarations())
        .chain(player_support_domains::declarations())
        .collect();
    assert_eq!(domains.len(), prior_domains.len() + additions.len());
    for added in &additions {
        assert!(prior_domains.iter().all(|row| row.owner != added.owner));
        // The checker canonically sorts source domains by typed subject.
        // Remove only each exact reviewed row, independent of authoring order.
        let positions: Vec<_> = domains
            .iter()
            .enumerate()
            .filter_map(|(index, row)| (row.owner == added.owner).then_some(index))
            .collect();
        assert_eq!(positions.len(), 1);
        assert_eq!(domains.remove(positions[0]), *added);
    }
    assert_eq!(domains, prior_domains);
    assert_eq!(restored.recipe.rules.owners.len(), historical.owners.len());
    for (current, original) in restored
        .recipe
        .rules
        .owners
        .iter_mut()
        .zip(&historical.owners)
    {
        assert_eq!(current.owner, original.owner, "exact retained owner order");
        assert_eq!(
            current.programs.members.len(),
            original.programs.members.len()
        );
        for (current, original) in current
            .programs
            .members
            .iter()
            .zip(&original.programs.members)
        {
            assert_eq!(
                current.id, original.id,
                "exact retained program membership order"
            );
        }
        current
            .programs
            .members
            .clone_from(&original.programs.members);
    }
    assert_eq!(
        restored.recipe.rules, historical,
        "all non-program rule metadata"
    );
    restored.recipe.registry = old_registry.input().clone();
    restored.recipe.schema = prior_schema;
    restored.recipe.routing = poe_optimizer_data::owned_routing::OwnedActionRouting::bind_new(
        parts.routing,
        &old_schema,
        Default::default(),
    )
    .unwrap()
    .input()
    .clone();
    let imported = parts
        .imports
        .into_input(&old_registry, &old_schema, Default::default())
        .unwrap();
    restored.mapping = imported.mapping;
    restored.roles = imported.roles;
    restored.normalization = imported.normalization;
    restored.rewards = imported.rewards;
    restored.items = imported.items;
    restored.item_source = imported.item_source;
    restored.tree = imported.tree;
    let restored = assemble_owned_release(restored, Default::default()).unwrap();
    assert_eq!(restored.receipt(), &reference);
    let artifacts: BTreeMap<_, _> = restored.artifacts().collect();
    assert_eq!(artifacts.len(), reference.artifacts.len() + 1);
    assert_eq!(current_artifacts.len(), artifacts.len());
    assert_eq!(artifacts["release.json"], reference_bytes);
    assert_ne!(current_artifacts["release.json"], reference_bytes);
    assert!(
        reference
            .artifacts
            .iter()
            .any(|row| row.file == "manifest.json")
    );
    for row in &reference.artifacts {
        let bytes = artifacts[row.file.as_str()];
        assert_eq!(bytes.len(), row.bytes, "{} length", row.file);
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            row.sha256,
            "{} digest",
            row.file
        );
        if row.file.starts_with("queries-") {
            assert_eq!(
                current_artifacts[row.file.as_str()],
                bytes,
                "unchanged query content: {}",
                row.file
            );
        } else {
            assert_ne!(
                current_artifacts[row.file.as_str()],
                bytes,
                "new declaration/dependency binding: {}",
                row.file
            );
        }
    }
    let mut decoded_bytes = 0;
    for (spec, (_, encoded)) in manifest.sources().iter().zip(owned) {
        match spec.encoding() {
            SourceEncoding::Plain => decoded_bytes += encoded.len(),
            SourceEncoding::Gzip => {
                let mut decoded = Vec::new();
                GzDecoder::new(encoded.as_slice())
                    .take(maximum as u64 + 1)
                    .read_to_end(&mut decoded)
                    .unwrap();
                assert!(decoded.len() <= maximum);
                decoded_bytes += decoded.len();
            }
        }
    }
    assert_eq!(
        actual.encoded_bytes(),
        owned.iter().map(|(_, bytes)| bytes.len()).sum::<usize>()
    );
    assert_eq!(actual.decoded_bytes(), decoded_bytes);
    assert_eq!(release.input().provenance, reference.provenance);
    assert!(
        release.evaluation().is_none(),
        "the retained source release makes no evaluation coverage claim"
    );
}

fn cli(manifest: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-author"))
        .args(["release", "build", "--bootstrap"])
        .arg(manifest)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

fn directory_files(path: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            (
                entry.file_name().into_string().unwrap(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

fn declared_files(path: &Path, source: &Sources) -> BTreeMap<String, Vec<u8>> {
    source
        .files
        .iter()
        .map(|(name, _)| name.as_str())
        .chain(std::iter::once("release.kdl"))
        .map(|name| (name.into(), fs::read(path.join(name)).unwrap()))
        .collect()
}

fn directory_entries(path: &Path) -> BTreeSet<std::ffi::OsString> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect()
}

fn checkpoint_files(path: &Path) -> BTreeMap<String, Vec<u8>> {
    assert_eq!(
        directory_entries(path),
        ["checkpoint.json", "names.json", "runtime"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect()
    );
    let mut output = BTreeMap::from([
        (
            "checkpoint.json".into(),
            fs::read(path.join("checkpoint.json")).unwrap(),
        ),
        (
            "names.json".into(),
            fs::read(path.join("names.json")).unwrap(),
        ),
    ]);
    output.extend(
        directory_files(&path.join("runtime"))
            .into_iter()
            .map(|(name, bytes)| (format!("runtime/{name}"), bytes)),
    );
    output
}

fn assert_checkpoint_runtime(path: &Path, expected: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    checkpoint_files(path);
    assert_eq!(directory_files(&path.join("runtime")), *expected);
    // The nested runtime remains independently loadable with its closed inventory.
    let checked = load_owned_release_artifacts(expected, Default::default()).unwrap();
    let names: poe_optimizer_authoring::NamesInput =
        serde_json::from_slice(&fs::read(path.join("names.json")).unwrap()).unwrap();
    assert_eq!(
        names.registry,
        checked.recipe().registry().identity().unwrap()
    );
    fs::read(path.join("checkpoint.json")).unwrap()
}

fn assert_checkpoint_success(result: &Output, receipt: &[u8]) {
    let parsed: poe_optimizer_authoring::AuthoringCheckpointReceipt =
        serde_json::from_slice(receipt).unwrap();
    let identity = parsed.identity(Default::default()).unwrap();
    assert_eq!(
        result.stderr,
        format!("checkpoint identity: {identity}\n").as_bytes()
    );
    let mut stdout = receipt.to_vec();
    stdout.push(b'\n');
    assert_eq!(result.stdout, stdout);
}

#[test]
fn cli_builds_declared_dsl_and_rejected_dsl_leaves_no_publication() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("dsl inputs λ");
    fs::create_dir(&input).unwrap();
    let dsl = DslRelease::new(&fixture().input, false);
    for (path, bytes) in &dsl.source.files {
        let target = input.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    let manifest = input.join("release.kdl");
    fs::write(&manifest, &dsl.source.manifest).unwrap();
    let initial_sources = declared_files(&input, &dsl.source);
    let output = temp.path().join("dsl release μ");
    let expected = assemble_owned_release(dsl.expected.clone(), Default::default()).unwrap();
    let result = cli(&manifest, &output);
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt = assert_checkpoint_runtime(&output, &files(&expected));
    assert_checkpoint_success(&result, &receipt);
    let published = checkpoint_files(&output);
    assert_eq!(declared_files(&input, &dsl.source), initial_sources);

    let rule = dsl.source_for("action-final");
    fs::write(
        input.join(&rule),
        "program \"action-final\" context action { let invalid = ; }",
    )
    .unwrap();
    let failed_output = temp.path().join("must not publish");
    let before = directory_entries(temp.path());
    let failed = cli(&manifest, &failed_output);
    assert_eq!(failed.status.code(), Some(2));
    assert!(failed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("invalid rule packet"));
    assert!(!failed_output.exists());
    assert_eq!(directory_entries(temp.path()), before);
    assert_eq!(checkpoint_files(&output), published);
}

#[test]
fn cli_build_publishes_exact_artifacts_and_failures_leave_no_output_or_staging() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("release inputs λ");
    fs::create_dir(&input).unwrap();
    let dsl = DslRelease::new(&fixture().input, true);
    let source = dsl.source;
    for (path, bytes) in &source.files {
        let target = input.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    let manifest = input.join("release.kdl");
    fs::write(&manifest, &source.manifest).unwrap();
    let initial_sources = declared_files(&input, &source);
    let output = temp.path().join("published release μ");
    let expected = assemble_owned_release(dsl.expected, Default::default()).unwrap();
    let expected_files = files(&expected);
    let result = cli(&manifest, &output);
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt = assert_checkpoint_runtime(&output, &expected_files);
    assert_checkpoint_success(&result, &receipt);
    let published = checkpoint_files(&output);
    assert_eq!(declared_files(&input, &source), initial_sources);
    let published_entries = directory_entries(temp.path());

    let repeated = cli(&manifest, &output);
    assert_eq!(repeated.status.code(), Some(2));
    assert!(repeated.stdout.is_empty());
    assert!(String::from_utf8_lossy(&repeated.stderr).contains("cannot publish"));
    assert_eq!(checkpoint_files(&output), published);
    assert_eq!(directory_entries(temp.path()), published_entries);

    for (text, message) in [
        (
            "release {\n unknown \"oops.json\"\n}\n".into(),
            "unknown source component",
        ),
        (
            source.manifest.replacen(
                "release {",
                &format!("release expected_input=\"{}\" {{", "0".repeat(64)),
                1,
            ),
            "assembled input identity differs from expected_input",
        ),
    ] {
        fs::write(&manifest, text).unwrap();
        let failed_output = temp.path().join("must not exist ν");
        let before = directory_entries(temp.path());
        let result = cli(&manifest, &failed_output);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!failed_output.exists());
        assert_eq!(directory_entries(temp.path()), before);
        assert_eq!(checkpoint_files(&output), published);
    }
    fs::write(&manifest, &source.manifest).unwrap();
    assert_eq!(declared_files(&input, &source), initial_sources);
}

#[cfg(unix)]
#[test]
fn cli_refuses_a_source_symlink_outside_the_manifest_directory() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    let source = DslRelease::new(&fixture().input, false).source;
    for (path, bytes) in &source.files {
        let target = input.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        if path == "metrics.json" {
            let outside = temp.path().join("outside-metrics.json");
            fs::write(&outside, bytes).unwrap();
            std::os::unix::fs::symlink(&outside, target).unwrap();
        } else {
            fs::write(target, bytes).unwrap();
        }
    }
    let manifest = input.join("release.kdl");
    fs::write(&manifest, source.manifest).unwrap();
    let output = temp.path().join("output");
    let before = directory_entries(temp.path());
    let result = cli(&manifest, &output);
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("source resolves outside the manifest directory")
    );
    assert!(!output.exists());
    assert_eq!(directory_entries(temp.path()), before);
}
