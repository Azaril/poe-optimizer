//! Replay complete positive public calls using a source-authenticated template.
//! Keep the original unadmitted diagnostic oracle separate and unchanged.
use super::*;
use poe_optimizer_data::source_program::{SourceProgramConstructor, SourceTableAllocation};
use poe_optimizer_pob::source_programs::attach_reserved_string_templates;

pub(super) struct Replay {
    session: ProgramSession,
    parser: SessionValue,
    cache: SessionValue,
    lookup: SessionValue,
    metadata: Json,
    seed: SourceProgramConstructor,
    source_function: Function,
    source_template: Table,
}
impl Replay {
    pub(super) fn new(pair: &Pair, witness: &ConstructorDiagnosticWitness) -> Self {
        let sources = copy_parity::original_sources(pair);
        witness.verify_unchanged().unwrap();
        let lowered =
            attach_reserved_string_templates(&sources, pair.lowered.clone(), &[witness]).unwrap();
        let sites = &lowered.catalog().constructors().unwrap().sites;
        let original_sites = pair.compiled.catalog().constructors().unwrap().sites.len();
        assert_eq!(sites.len(), original_sites + 1);
        let site = sites
            .iter()
            .find(|site| {
                site.callback == witness.report().callback
                    && site.expression == witness.report().expression
            })
            .unwrap();
        let SourceTableAllocation::DuplicateReservedStrings { keys } = &site.allocation else {
            panic!("actual template must grant only reserved-string traversal");
        };
        let metadata = json!({"site":site,"reserved_key_count":keys.len(),
            "existing_constructor_sites_retained":original_sites,
            "scope":"new native session from original coherent input; source-bound template order only; no final values, parser results or runtime source host injected"});
        let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
        let (session, roots) = compiled
            .session_from_input(pair.observed.input(), limits())
            .unwrap();
        let root = |name: &str| roots[pair.observed.root_index(name).unwrap()].clone();
        Self {
            session,
            parser: root("original.parser"),
            cache: root("public.cache"),
            lookup: root("probe.lookup"),
            metadata,
            seed: site.clone(),
            source_function: witness.function().clone(),
            source_template: witness.template().unwrap().clone(),
        }
    }
    fn key(&mut self, key: &Value) -> SessionValue {
        self.session
            .borrow(&observation::capture(std::slice::from_ref(key)))
            .unwrap()
            .remove(0)
    }
    fn row(&mut self, key: &SessionValue) -> SessionValue {
        self.session
            .invoke_callable(&self.lookup, &[self.cache.clone(), key.clone()])
            .unwrap()
            .remove(0)
    }
    fn compare(&mut self, source: &[Value], native: &[SessionValue], label: &str) -> Json {
        let actual = observation::canonical(self.session.snapshot(native).unwrap().graph());
        assert_eq!(
            actual,
            observation::canonical(&observation::capture(source)),
            "{label}"
        );
        actual
    }
    pub(super) fn positive(
        &mut self,
        reference: (&Function, &Table),
        key: &Value,
        source_row: &Value,
        source_miss: &[Value],
        witness: &ConstructorDiagnosticWitness,
    ) -> Json {
        let (parser, cache) = reference;
        witness.verify_unchanged().unwrap();
        assert_eq!(witness.function(), &self.source_function);
        assert_eq!(witness.template(), Some(&self.source_template));
        let report = witness.report();
        assert_eq!(report.callback, self.seed.callback);
        assert_eq!(report.expression, self.seed.expression);
        assert_eq!(report.provenance, self.seed.provenance);
        assert_eq!(report.bytecode_sha256, self.seed.bytecode_sha256);
        assert_eq!(report.instruction.pc, self.seed.bytecode_pc);
        assert_eq!(report.instruction.word, self.seed.instruction);
        let SourceTableAllocation::DuplicateReservedStrings { keys } = &self.seed.allocation else {
            unreachable!();
        };
        assert_eq!(report.template_rows.len(), keys.len());
        for (row, key) in report.template_rows.iter().zip(keys) {
            assert_eq!(row.key, ConstructorTemplateValue::Bytes(key.clone()));
            assert_eq!(row.value, ConstructorTemplateValue::SelfMarker);
        }
        assert_eq!(cache.raw_get::<Value>(key.clone()).unwrap(), *source_row);
        let argument = self.key(key);
        let before = self.row(&argument);
        self.compare(
            &[Value::Nil],
            &[before],
            "new reserved replay starts as cache miss",
        );
        let false_arg = self
            .session
            .borrow(&ProgramValueGraph {
                values: vec![ProgramValue::Boolean(false)],
                tables: vec![],
            })
            .unwrap()
            .remove(0);
        let args = [argument.clone(), false_arg];
        let native_miss = self
            .session
            .invoke_callable(&self.parser, &args)
            .unwrap_or_else(|error| {
                panic!("reserved-template complete public miss {key:?}: {error}")
            });
        let native_row = self.row(&argument);
        let mut source = vec![source_row.clone()];
        source.extend_from_slice(source_miss);
        let mut native = vec![native_row.clone()];
        native.extend(native_miss.clone());
        let miss = self.compare(
            &source,
            &native,
            "reserved complete public miss/cache aliases",
        );

        let source_hit = parser
            .call::<MultiValue>((key.clone(), false))
            .unwrap()
            .into_vec();
        let native_hit = self.session.invoke_callable(&self.parser, &args).unwrap();
        let current_row = self.row(&argument);
        // A joint snapshot preserves both independent result copies and the
        // retained cache row identity across calls, including nested aliases.
        source.push(cache.raw_get::<Value>(key.clone()).unwrap());
        source.extend(source_hit);
        native.push(current_row);
        native.extend(native_hit);
        let history = self.compare(
            &source,
            &native,
            "reserved miss/hit/retained row and copies",
        );
        witness.verify_unchanged().unwrap();
        let allocations = self.session.allocations();
        json!({"complete_miss":true,"complete_hit":true,"miss":miss,"history":history,
            "metadata":self.metadata,"steps":self.session.steps(),
            "pattern_steps":self.session.pattern_steps(),"allocations":{"values":allocations.values,"bytes":allocations.bytes,"tables":allocations.tables},
            "source_template_still_rooted_and_unchanged":true,
            "complete_public_parser":false,"complete_native_build":false})
    }
}
