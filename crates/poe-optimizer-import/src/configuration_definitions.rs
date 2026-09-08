//! Data-bound lookup of authored configuration, before load migrations or effects.
//! A definition or option match is evidence about source metadata, never admission.
use crate::configuration::{ConfigurationProjection, ConfigurationRecordIndex, ScalarInput};
use poe_optimizer_core::{data::DataIdentity, options::Scalar};
use poe_optimizer_data::{
    configuration::{
        ConfigDefinitionCatalog, ConfigInitialState, ConfigScalarKind, ConfigWidgetKind,
    },
    game_data::{DataTrust, GameDataSnapshot},
};
use serde::Serialize;
use std::ops::Range;

/// Bound expansion when a custom catalog contains many same-key definition rows.
pub const MAX_DEFINITION_MATCHES: usize = 65_536;
/// Bound both work and emitted option indexes for repeated list definitions.
pub const MAX_OPTION_COMPARISONS: usize = 65_536;

#[derive(Debug, thiserror::Error)]
#[error("configuration definition lookup: {0}")]
pub struct DefinitionLookupError(String);

#[derive(Debug, Clone, Serialize)]
pub struct DefinitionMatch {
    definition_id: String,
    source_table_index: u32,
    source_variable_order: u32,
    widget: ConfigWidgetKind,
    scalar_kind_matches: bool,
    /// None for non-list widgets. Empty means no exact authored option match.
    option_indices: Option<Vec<u32>>,
}
impl DefinitionMatch {
    pub fn definition_id(&self) -> &str {
        &self.definition_id
    }
    pub fn scalar_kind_matches(&self) -> bool {
        self.scalar_kind_matches
    }
    pub fn option_indices(&self) -> Option<&[u32]> {
        self.option_indices.as_deref()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScalarDefinitionLookup {
    name: String,
    value: Scalar,
    source_range: Range<usize>,
    value_source_range: Range<usize>,
    definitions: Vec<DefinitionMatch>,
}
impl ScalarDefinitionLookup {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn value(&self) -> &Scalar {
        &self.value
    }
    pub fn definitions(&self) -> &[DefinitionMatch] {
        &self.definitions
    }
    pub fn source_range(&self) -> Range<usize> {
        self.source_range.clone()
    }
    pub fn is_unknown_key(&self) -> bool {
        self.definitions.is_empty()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DefinitionRecord {
    Input {
        index: usize,
        lookup: ScalarDefinitionLookup,
    },
    Placeholder {
        index: usize,
        lookup: ScalarDefinitionLookup,
    },
    Block {
        index: usize,
        source_range: Range<usize>,
    },
    Unknown {
        index: usize,
        element_name: String,
        source_range: Range<usize>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigSetDefinitionLookup {
    id: u32,
    selected: bool,
    records: Vec<DefinitionRecord>,
}
impl ConfigSetDefinitionLookup {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn is_selected(&self) -> bool {
        self.selected
    }
    pub fn records(&self) -> &[DefinitionRecord] {
        &self.records
    }
}

/// Immutable evidence binds one exact source document and one validated data snapshot.
/// Initial defaults are catalog creation state, before authored writes and callbacks.
#[derive(Debug, Clone, Serialize)]
pub struct ConfigurationDefinitionLookup {
    schema_version: u32,
    interpretation: &'static str,
    source_xml_sha256: String,
    data: DataIdentity,
    data_trust: DataTrust,
    catalog_schema_version: u32,
    catalog_definition_count: usize,
    initial_defaults_before_load: ConfigInitialState,
    option_membership_is_import_whitelist: bool,
    effective_configuration: &'static str,
    game_mechanics: &'static str,
    callbacks: &'static str,
    sets: Vec<ConfigSetDefinitionLookup>,
}
impl ConfigurationDefinitionLookup {
    pub fn source_xml_sha256(&self) -> &str {
        &self.source_xml_sha256
    }
    pub fn data(&self) -> &DataIdentity {
        &self.data
    }
    pub fn sets(&self) -> &[ConfigSetDefinitionLookup] {
        &self.sets
    }
    pub fn initial_defaults_before_load(&self) -> &ConfigInitialState {
        &self.initial_defaults_before_load
    }
}

fn lookup_scalar(
    input: &ScalarInput<'_>,
    catalog: &ConfigDefinitionCatalog,
    total_matches: &mut usize,
    option_comparisons: &mut usize,
) -> Result<ScalarDefinitionLookup, DefinitionLookupError> {
    let mut definitions = Vec::new();
    for definition in catalog.definitions_for_key(input.name()) {
        *total_matches += 1;
        if *total_matches > MAX_DEFINITION_MATCHES {
            return Err(DefinitionLookupError(
                "matched definition expansion exceeds limit".into(),
            ));
        }
        let option_indices = if matches!(definition.widget, ConfigWidgetKind::List) {
            *option_comparisons += definition.options.len();
            if *option_comparisons > MAX_OPTION_COMPARISONS {
                return Err(DefinitionLookupError(
                    "option comparison expansion exceeds limit".into(),
                ));
            }
            Some(
                definition
                    .options
                    .iter()
                    .filter(|option| option.matches(input.value()))
                    .map(|option| option.index)
                    .collect(),
            )
        } else {
            None
        };
        definitions.push(DefinitionMatch {
            definition_id: definition.id.clone(),
            source_table_index: definition.source_table_index,
            source_variable_order: definition.source_variable_order,
            widget: definition.widget,
            scalar_kind_matches: definition
                .scalar_kinds
                .contains(&ConfigScalarKind::of(input.value())),
            option_indices,
        });
    }
    Ok(ScalarDefinitionLookup {
        name: input.name().into(),
        value: input.value().clone(),
        source_range: input.source_range(),
        value_source_range: input.value_source().range(),
        definitions,
    })
}

/// Enrich all authored sets without selecting a different build or changing any scalar.
/// Unknown keys, unlisted strings and kind mismatches are retained as lookup evidence.
pub fn lookup_definitions(
    projection: &ConfigurationProjection<'_>,
    snapshot: &GameDataSnapshot,
) -> Result<ConfigurationDefinitionLookup, DefinitionLookupError> {
    let catalog = snapshot.configuration();
    let mut total_matches = 0;
    let mut option_comparisons = 0;
    let mut sets = Vec::with_capacity(projection.sets().len());
    for set in projection.sets() {
        let mut records = Vec::with_capacity(set.records_in_source_order().len());
        for &record in set.records_in_source_order() {
            records.push(match record {
                ConfigurationRecordIndex::Input(index) => DefinitionRecord::Input {
                    index,
                    lookup: lookup_scalar(
                        &set.inputs()[index],
                        catalog,
                        &mut total_matches,
                        &mut option_comparisons,
                    )?,
                },
                ConfigurationRecordIndex::Placeholder(index) => DefinitionRecord::Placeholder {
                    index,
                    lookup: lookup_scalar(
                        &set.placeholders()[index],
                        catalog,
                        &mut total_matches,
                        &mut option_comparisons,
                    )?,
                },
                ConfigurationRecordIndex::Block(index) => DefinitionRecord::Block {
                    index,
                    source_range: set.blocks()[index].source_range(),
                },
                ConfigurationRecordIndex::Unknown(index) => DefinitionRecord::Unknown {
                    index,
                    element_name: set.unknown_records()[index].element_name().into(),
                    source_range: set.unknown_records()[index].source_range(),
                },
            });
        }
        sets.push(ConfigSetDefinitionLookup {
            id: set.id(),
            selected: set.id() == projection.active_set_id(),
            records,
        });
    }
    Ok(ConfigurationDefinitionLookup {
        schema_version: 1,
        interpretation: "authored_configuration_definition_lookup",
        source_xml_sha256: projection.source_sha256().into(),
        data: snapshot.identity().clone(),
        data_trust: snapshot.trust().clone(),
        catalog_schema_version: catalog.data().schema_version,
        catalog_definition_count: catalog.definitions().len(),
        initial_defaults_before_load: catalog.initial_state(),
        option_membership_is_import_whitelist: false,
        effective_configuration: "not_evaluated",
        game_mechanics: "not_evaluated",
        callbacks: "not_executed",
        sets,
    })
}
