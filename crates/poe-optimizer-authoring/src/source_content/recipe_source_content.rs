//! Rule source payload whose envelope was authenticated before extraction.
use poe_optimizer_core::{
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{
        ContributionQuery, DefinitionRules, EffectApplicationRule, ExistingActorRuleApplication,
        IntegerRuleTable, OWNED_RULE_PACKAGE_VERSION, RulePackageInput, StatReceiver,
        SupportDiscoveryInput,
    },
    owned_schema::{DeclaredSet, DefinitionSchemaIndex},
};
use serde::{Deserialize, Deserializer, Serialize};

/// Authored rule semantics without current version, namespace or schema bindings.
/// The source checker authenticates only the original envelope; these declarations
/// and programs remain unvalidated until ordinary final storage and compilation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSourceContent {
    pub release: OwnedDefinitionKey,
    pub semantics_version: OwnedDefinitionKey,
    pub operations_version: OwnedDefinitionKey,
    pub tables: Vec<IntegerRuleTable>,
    pub owners: Vec<DefinitionRules>,
    pub receivers: DeclaredSet<StatReceiver>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_applications: Option<DeclaredSet<EffectApplicationRule>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution_queries: Option<DeclaredSet<ContributionQuery>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_existing_actor_rules"
    )]
    pub existing_actor_rules: Option<DeclaredSet<ExistingActorRuleApplication>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_discovery: Option<SupportDiscoveryInput>,
}

fn present_existing_actor_rules<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<DeclaredSet<ExistingActorRuleApplication>>, D::Error> {
    DeclaredSet::deserialize(deserializer).map(Some)
}

impl RuleSourceContent {
    /// Called only after bounding the original input and checking its envelope.
    pub(super) fn from_checked_header(input: RulePackageInput) -> Self {
        let RulePackageInput {
            schema_version: _,
            namespace: _,
            release,
            semantics_version,
            operations_version,
            definitions: _,
            tables,
            owners,
            receivers,
            effect_applications,
            contribution_queries,
            existing_actor_rules,
            support_discovery,
        } = input;
        Self {
            release,
            semantics_version,
            operations_version,
            tables,
            owners,
            receivers,
            effect_applications,
            contribution_queries,
            existing_actor_rules,
            support_discovery,
        }
    }

    /// Construct a raw input for source lowering and ordinary final validation.
    /// This neither validates programs nor completes any declared coverage.
    pub fn into_input<I: DefinitionSchemaIndex>(self, schema: &I) -> RulePackageInput {
        RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: schema.namespace().clone(),
            release: self.release,
            semantics_version: self.semantics_version,
            operations_version: self.operations_version,
            definitions: schema.identity().clone(),
            tables: self.tables,
            owners: self.owners,
            receivers: self.receivers,
            effect_applications: self.effect_applications,
            contribution_queries: self.contribution_queries,
            existing_actor_rules: self.existing_actor_rules,
            support_discovery: self.support_discovery,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poe_optimizer_core::{
        owned_definitions::{ActorDefId, GameVersionNamespace},
        owned_rules::RuleProgram,
        owned_schema::{DefinitionAddress, RuleEntityKind, SchemaFacet, SchemaGap, SchemaSubject},
    };
    use poe_optimizer_data::{
        owned_rules::{OwnedRulePackage, RuleStorageLimits},
        owned_schema::{
            OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, OwnedSchemaLimits,
            SchemaPackageInput,
        },
    };

    fn key(value: &str) -> OwnedDefinitionKey {
        OwnedDefinitionKey::new(value).unwrap()
    }

    fn schema(release: &str) -> OwnedDefinitionSchemaPackage {
        OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: GameVersionNamespace::new("test", "v1").unwrap(),
                release: key(release),
                semantics_version: key("v1"),
                definitions: vec![],
                slots: vec![],
            },
            OwnedSchemaLimits::default(),
        )
        .unwrap()
    }

    fn raw(schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
        let owner = SchemaSubject::Definition(DefinitionAddress::Actor(ActorDefId::new(
            schema.namespace().clone(),
            key("undeclared-actor"),
        )));
        let program = RuleProgram {
            id: key("duplicate-unvalidated-program"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![],
            effects: vec![],
        };
        RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: schema.namespace().clone(),
            release: key("rules"),
            semantics_version: key("unvalidated-semantics"),
            operations_version: key("unvalidated-operations"),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: vec![DefinitionRules {
                owner: owner.clone(),
                programs: DeclaredSet::partial(
                    vec![program.clone(), program],
                    vec![SchemaGap {
                        subject: owner,
                        facet: SchemaFacet::GameRules,
                        code: key("unconverted"),
                    }],
                ),
            }],
            receivers: DeclaredSet::complete(vec![]),
            effect_applications: Some(DeclaredSet::complete(vec![])),
            contribution_queries: Some(DeclaredSet::complete(vec![])),
            existing_actor_rules: Some(DeclaredSet::complete(vec![])),
            support_discovery: Some(SupportDiscoveryInput { providers: vec![] }),
        }
    }

    #[test]
    fn rule_source_preserves_unvalidated_payload_and_only_constructs_new_headers() {
        let original_schema = schema("original");
        let original = raw(&original_schema);
        let content = RuleSourceContent::from_checked_header(original.clone());
        assert_eq!(content.clone().into_input(&original_schema), original);
        let next = schema("next");
        assert_ne!(next.identity(), original_schema.identity());
        let constructed = content.into_input(&next);
        let mut expected = original;
        expected.definitions = next.identity().clone();
        assert_eq!(constructed, expected);
        // Extraction is not rule validation and cannot turn incomplete or
        // malformed authored semantics into an executable package.
        assert!(OwnedRulePackage::new(constructed, &next, RuleStorageLimits::default()).is_err());
    }

    #[test]
    fn rule_source_keeps_optional_wire_contract_and_rejects_supplied_headers() {
        let schema = schema("schema");
        let mut original = raw(&schema);
        for present in [true, false] {
            if !present {
                original.effect_applications = None;
                original.contribution_queries = None;
                original.existing_actor_rules = None;
                original.support_discovery = None;
            }
            let content = RuleSourceContent::from_checked_header(original.clone());
            let mut wire = serde_json::to_value(&content).unwrap();
            let mut expected = serde_json::to_value(&original).unwrap();
            for header in ["schema_version", "namespace", "definitions"] {
                expected.as_object_mut().unwrap().remove(header);
                assert!(wire.get(header).is_none());
                let mut supplied = wire.clone();
                supplied[header] = serde_json::Value::Null;
                assert!(serde_json::from_value::<RuleSourceContent>(supplied).is_err());
            }
            assert_eq!(wire, expected);
            assert_eq!(
                serde_json::from_value::<RuleSourceContent>(wire.clone()).unwrap(),
                content
            );
            wire["existing_actor_rules"] = serde_json::Value::Null;
            assert!(serde_json::from_value::<RuleSourceContent>(wire).is_err());
        }
    }
}
