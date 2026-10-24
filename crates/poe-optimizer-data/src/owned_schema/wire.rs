//! Private typed storage projection. Local IDs never acquire a namespace until
//! the entire compact input has passed its collection and expansion preflight.
use super::{
    OwnedSchemaLimits, Result, SchemaPackageError, SchemaPackageErrorKind, SchemaPackageInput,
    invalid,
};
use poe_optimizer_core::{owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, DeserializeOwned},
    ser::{SerializeSeq, SerializeStruct},
};
use std::{fmt, marker::PhantomData};

mod types;

const WIRE_VERSION: u32 = super::OWNED_SCHEMA_WIRE_VERSION;

trait Preflight {
    fn preflight(&self, budget: &mut Budget) -> Result;
}

struct Budget {
    limits: OwnedSchemaLimits,
    entries: usize,
    reference_bytes: usize,
    namespace_bytes: usize,
}
impl Budget {
    fn collection(&mut self, length: usize) -> Result {
        if length > self.limits.max_collection_entries {
            return invalid("wire.collection", SchemaPackageErrorKind::LimitExceeded);
        }
        self.entries = self
            .entries
            .checked_add(length)
            .filter(|total| *total <= self.limits.max_entries)
            .ok_or_else(|| SchemaPackageError::Invalid {
                path: "wire.entries".into(),
                kind: SchemaPackageErrorKind::LimitExceeded,
            })?;
        Ok(())
    }

    fn reference(&mut self, kind: DefinitionKind, key: &OwnedDefinitionKey) -> Result {
        // Core symbols contain only unescaped ASCII. This is the exact encoded
        // size of this expanded DefId, including the repeated root namespace.
        let bytes = self.namespace_bytes
            + kind.as_str().len()
            + key.as_str().len()
            + r#"{"kind":"","namespace":,"key":""}"#.len();
        self.reference_bytes = self
            .reference_bytes
            .checked_add(bytes)
            .filter(|total| *total <= self.limits.max_semantic_bytes)
            .ok_or(SchemaPackageError::SemanticTooLarge {
                maximum: self.limits.max_semantic_bytes,
            })?;
        Ok(())
    }
}

trait Project: Sized {
    type Wire: DeserializeOwned + Preflight;

    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>;
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self>;
}

struct View<'a, T> {
    value: &'a T,
    namespace: &'a GameVersionNamespace,
}
impl<'a, T> View<'a, T> {
    fn new(value: &'a T, namespace: &'a GameVersionNamespace) -> Self {
        Self { value, namespace }
    }
}
impl<T: Project> Serialize for View<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        Project::serialize(self.value, self.namespace, serializer)
    }
}

impl<T: Preflight> Preflight for Vec<T> {
    fn preflight(&self, budget: &mut Budget) -> Result {
        budget.collection(self.len())?;
        for value in self {
            value.preflight(budget)?;
        }
        Ok(())
    }
}
impl<T: Project> Project for Vec<T> {
    type Wire = Vec<T::Wire>;

    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.len()))?;
        for value in self {
            sequence.serialize_element(&View::new(value, namespace))?;
        }
        sequence.end()
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        wire.into_iter()
            .map(|value| T::expand(value, namespace))
            .collect()
    }
}

impl<T: Preflight> Preflight for Box<T> {
    fn preflight(&self, budget: &mut Budget) -> Result {
        self.as_ref().preflight(budget)
    }
}
impl<T: Project> Project for Box<T> {
    type Wire = Box<T::Wire>;

    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        Project::serialize(self.as_ref(), namespace, serializer)
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        Ok(Box::new(T::expand(*wire, namespace)?))
    }
}

impl<T: Preflight> Preflight for Option<T> {
    fn preflight(&self, budget: &mut Budget) -> Result {
        if let Some(value) = self {
            value.preflight(budget)?;
        }
        Ok(())
    }
}
impl<T: Project> Project for Option<T> {
    type Wire = Option<T::Wire>;

    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Some(value) => serializer.serialize_some(&View::new(value, namespace)),
            None => serializer.serialize_none(),
        }
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        wire.map(|value| T::expand(value, namespace)).transpose()
    }
}

struct LocalId<K> {
    key: OwnedDefinitionKey,
    marker: PhantomData<fn() -> K>,
}
impl<'de, K: DefinitionDomain> Deserialize<'de> for LocalId<K> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct Visitor<K>(PhantomData<fn() -> K>);
        impl<K: DefinitionDomain> de::Visitor<'_> for Visitor<K> {
            type Value = LocalId<K>;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "a local {}:key reference", K::KIND.as_str())
            }
            fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<Self::Value, E> {
                let (kind, key) = value
                    .split_once(':')
                    .ok_or_else(|| E::custom("local reference must contain kind:key"))?;
                if kind != K::KIND.as_str() {
                    return Err(E::custom(format!(
                        "expected local definition kind {}, received {kind}",
                        K::KIND.as_str()
                    )));
                }
                Ok(LocalId {
                    key: OwnedDefinitionKey::new(key).map_err(E::custom)?,
                    marker: PhantomData,
                })
            }
        }
        deserializer.deserialize_str(Visitor::<K>(PhantomData))
    }
}
impl<K: DefinitionDomain> Preflight for LocalId<K> {
    fn preflight(&self, budget: &mut Budget) -> Result {
        budget.reference(K::KIND, &self.key)
    }
}
impl<K: DefinitionDomain> Project for DefId<K> {
    type Wire = LocalId<K>;

    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        if self.namespace() != namespace {
            return Err(serde::ser::Error::custom(
                "foreign namespace in local schema reference",
            ));
        }
        struct Local<'a, K: DefinitionDomain>(&'a DefId<K>);
        impl<K: DefinitionDomain> fmt::Display for Local<'_, K> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}:{}", K::KIND.as_str(), self.0.key())
            }
        }
        serializer.collect_str(&Local(self))
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        Ok(Self::new(namespace.clone(), wire.key))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuantityWire {
    value: f64,
    unit: LocalId<UnitDefinition>,
}
impl Preflight for QuantityWire {
    fn preflight(&self, budget: &mut Budget) -> Result {
        self.unit.preflight(budget)
    }
}
impl Project for FiniteQuantity {
    type Wire = QuantityWire;

    fn serialize<S: Serializer>(
        &self,
        namespace: &GameVersionNamespace,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut record = serializer.serialize_struct("FiniteQuantity", 2)?;
        record.serialize_field("value", &self.value())?;
        record.serialize_field("unit", &View::new(self.unit(), namespace))?;
        record.end()
    }
    fn expand(wire: Self::Wire, namespace: &GameVersionNamespace) -> Result<Self> {
        Self::new(wire.value, UnitDefId::expand(wire.unit, namespace)?)
            .map_err(|error| SchemaPackageError::Json(de::Error::custom(error)))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireInput {
    wire_version: u32,
    schema_version: u32,
    namespace: GameVersionNamespace,
    release: OwnedDefinitionKey,
    semantics_version: OwnedDefinitionKey,
    definitions: <Vec<DefinitionDescriptor> as Project>::Wire,
    slots: <Vec<SlotDescriptor> as Project>::Wire,
}

pub(super) fn decode(bytes: &[u8], limits: OwnedSchemaLimits) -> Result<SchemaPackageInput> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(SchemaPackageError::TooLarge {
            maximum: limits.max_wire_bytes,
        });
    }
    // serde_json's recursion limit remains enabled. All typed keys are validated
    // here, but no local reference has copied the namespace yet.
    let wire: WireInput = serde_json::from_slice(bytes)?;
    if wire.wire_version != WIRE_VERSION {
        return Err(SchemaPackageError::UnsupportedWireVersion(
            wire.wire_version,
        ));
    }
    if wire.schema_version != super::OWNED_SCHEMA_PACKAGE_VERSION {
        return Err(SchemaPackageError::UnsupportedVersion(wire.schema_version));
    }
    let mut budget = Budget {
        limits,
        entries: 0,
        reference_bytes: 0,
        namespace_bytes: serde_json::to_vec(&wire.namespace)?.len(),
    };
    wire.definitions.preflight(&mut budget)?;
    wire.slots.preflight(&mut budget)?;
    let definitions = Vec::<DefinitionDescriptor>::expand(wire.definitions, &wire.namespace)?;
    let slots = Vec::<SlotDescriptor>::expand(wire.slots, &wire.namespace)?;
    Ok(SchemaPackageInput {
        schema_version: wire.schema_version,
        namespace: wire.namespace,
        release: wire.release,
        semantics_version: wire.semantics_version,
        definitions,
        slots,
    })
}

pub(super) struct SchemaWireView<'a> {
    input: &'a SchemaPackageInput,
    additions: &'a [DefinitionDescriptor],
}
impl<'a> SchemaWireView<'a> {
    pub(super) fn new(
        input: &'a SchemaPackageInput,
        additions: &'a [DefinitionDescriptor],
    ) -> Self {
        Self { input, additions }
    }
}
impl Serialize for SchemaWireView<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        struct Definitions<'a> {
            input: &'a SchemaPackageInput,
            additions: &'a [DefinitionDescriptor],
        }
        impl Serialize for Definitions<'_> {
            fn serialize<S: Serializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                let length = self
                    .input
                    .definitions
                    .len()
                    .checked_add(self.additions.len())
                    .ok_or_else(|| serde::ser::Error::custom("schema definition count overflow"))?;
                let mut sequence = serializer.serialize_seq(Some(length))?;
                for descriptor in self.input.definitions.iter().chain(self.additions) {
                    sequence.serialize_element(&View::new(descriptor, &self.input.namespace))?;
                }
                sequence.end()
            }
        }
        let mut record = serializer.serialize_struct("SchemaPackageWire", 7)?;
        record.serialize_field("wire_version", &WIRE_VERSION)?;
        record.serialize_field("schema_version", &self.input.schema_version)?;
        record.serialize_field("namespace", &self.input.namespace)?;
        record.serialize_field("release", &self.input.release)?;
        record.serialize_field("semantics_version", &self.input.semantics_version)?;
        record.serialize_field(
            "definitions",
            &Definitions {
                input: self.input,
                additions: self.additions,
            },
        )?;
        record.serialize_field(
            "slots",
            &View::new(&self.input.slots, &self.input.namespace),
        )?;
        record.end()
    }
}
