//! Explicit, source-bound permission to dispatch packaged typed programs.
//! Bindings detect stale data; evidence text is a provenance claim governed by
//! the enclosing package TrustPolicy, never a cryptographic parity attestation.
use super::*;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserProgramPayload {
    pub data: ParserProgramData,
    #[serde(deserialize_with = "unique_map")]
    pub admissions: BTreeMap<ParserCallbackId, ParserProgramAdmission>,
}
impl Default for ParserProgramPayload {
    fn default() -> Self {
        Self {
            data: ParserProgramData {
                schema_version: PARSER_PROGRAM_SCHEMA_VERSION,
                programs: Vec::new(),
                callbacks: BTreeMap::new(),
            },
            admissions: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserProgramRole {
    Special,
    Helper,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserProgramAdmission {
    pub role: ParserProgramRole,
    pub owner_sha256: String,
    pub program_sha256: String,
    pub source: ParserProgramProvenance,
    pub evidence: String,
}
impl ParserProgramAdmission {
    /// Bind an explicit authored permission to exact definitions and IR. This
    /// does not assert that the supplied evidence proves original-source parity.
    /// The complete payload is structurally/binding checked when its owner loads.
    pub fn bind(
        owner: &ModifierParserData,
        program: &ParserProgram,
        role: ParserProgramRole,
        evidence: impl Into<String>,
    ) -> ParserProgramResult<Self> {
        let evidence = evidence.into();
        check_evidence(&evidence)?;
        Ok(Self {
            role,
            owner_sha256: owner
                .definition_sha256()
                .map_err(|e| failure(e.to_string()))?,
            program_sha256: program.sha256()?,
            source: program.provenance.clone(),
            evidence,
        })
    }
}

/// The full structurally validated library, plus injected dispatch permissions,
/// sharing the exact owner Arc. The owner stores only serialized data, not this
/// object, so retaining this catalog cannot form an Arc cycle.
#[derive(Debug, Clone)]
pub struct ParserAdmittedProgramCatalog {
    programs: ParserProgramCatalog,
    owner_sha256: String,
}
impl ParserAdmittedProgramCatalog {
    pub fn new(owner: &ModifierParserCatalog) -> ParserProgramResult<Self> {
        let (required, owner_sha256) = check_payload(owner.data())?;
        let programs = ParserProgramCatalog {
            source: crate::source_program::SourceProgramCatalog::from_verified_parser_payload(
                owner.clone(),
                required,
            ),
        };
        Ok(Self {
            programs,
            owner_sha256,
        })
    }
    pub fn programs(&self) -> &ParserProgramCatalog {
        &self.programs
    }
    pub fn admission(&self, callback: ParserCallbackId) -> Option<&ParserProgramAdmission> {
        self.programs
            .owner()
            .data()
            .programs
            .admissions
            .get(&callback)
    }
    pub fn is_special(&self, callback: ParserCallbackId) -> bool {
        self.admission(callback)
            .is_some_and(|a| a.role == ParserProgramRole::Special)
    }
    pub fn is_admitted(&self, callback: ParserCallbackId) -> bool {
        self.admission(callback).is_some()
    }
    pub fn is_bound_to(&self, owner: &ModifierParserCatalog) -> bool {
        self.programs.is_bound_to(owner)
    }
    pub fn owner_sha256(&self) -> &str {
        &self.owner_sha256
    }
}

impl ParserProgramPayload {
    pub(crate) fn validate_owner(&self, owner: &ModifierParserData) -> ParserProgramResult<()> {
        check_payload(owner).map(|_| ())
    }
}
fn check_payload(
    owner: &ModifierParserData,
) -> ParserProgramResult<(BTreeSet<ParserProgramCapability>, String)> {
    let payload = &owner.programs;
    let required = validate::validate(&payload.data, owner)?;
    if payload.admissions.len() > payload.data.programs.len() {
        return Err(failure("more admissions than programs"));
    }
    let owner_sha256 = owner
        .definition_sha256()
        .map_err(|e| failure(e.to_string()))?;
    let special_table =
        &owner.tables[owner.dictionaries[&ParserDictionary::Special].0 as usize - 1];
    let specials: BTreeSet<_> = special_table
        .fields
        .values()
        .filter_map(|value| match value {
            ParserValue::Callback(id) => Some(*id),
            _ => None,
        })
        .collect();
    let mut evidence_bytes = 0usize;
    for (callback, admission) in &payload.admissions {
        check_evidence(&admission.evidence)?;
        evidence_bytes = evidence_bytes
            .checked_add(admission.evidence.len())
            .filter(|&bytes| bytes <= 1024 * 1024)
            .ok_or_else(|| failure("aggregate admission evidence bound"))?;
        let id = payload
            .data
            .callbacks
            .get(callback)
            .ok_or_else(|| failure("admission has no source program"))?;
        let program = &payload.data.programs[id.0 as usize - 1];
        if admission.owner_sha256 != owner_sha256
            || admission.program_sha256 != program.sha256()?
            || admission.source != program.provenance
        {
            return Err(failure("stale admission source, IR or owner identity"));
        }
        if admission.role == ParserProgramRole::Special && !specials.contains(callback) {
            return Err(failure(
                "Special admission is not a final Special dictionary callback",
            ));
        }
        // All static edges are checked, including currently dead source branches.
        for binding in &program.bindings {
            if let ParserProgramBinding::CapturedCallback {
                callback: callee, ..
            } = binding
                && (!payload.admissions.contains_key(callee)
                    || !payload.data.callbacks.contains_key(callee))
            {
                return Err(failure(
                    "admitted program has an unadmitted captured helper",
                ));
            }
        }
    }
    Ok((required, owner_sha256))
}

fn failure(message: impl Into<String>) -> ParserProgramError {
    validate::error(ParserProgramErrorKind::Binding, None, None, message)
}
fn check_evidence(value: &str) -> ParserProgramResult<()> {
    if value.is_empty() || value.len() > 4096 || value.contains('\0') {
        return Err(failure("invalid admission evidence identifier"));
    }
    Ok(())
}
fn digest_json(value: &impl Serialize) -> std::result::Result<String, serde_json::Error> {
    struct Sink(Sha256);
    impl std::io::Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut sink = Sink(Sha256::new());
    serde_json::to_writer(&mut sink, value)?;
    Ok(format!("{:x}", sink.0.finalize()))
}
impl ParserProgram {
    /// Deterministic digest of complete serialized IR, including source mapping.
    /// This helper does not replace structural verification of authored data.
    pub fn sha256(&self) -> ParserProgramResult<String> {
        digest_json(self).map_err(|e| failure(format!("program serialization: {e}")))
    }
}

// Explicit borrowed projection avoids cloning the large graph or accidentally
// including admissions in their own identity. New definition fields must enter
// this view; program/admission payload is the sole excluded field.
#[derive(Serialize)]
struct Definitions<'a> {
    schema_version: u32,
    source: &'a ItemLoadingSource,
    dictionaries: &'a BTreeMap<ParserDictionary, ParserTableId>,
    tables: &'a [ParserTable],
    callbacks: &'a [ParserCallback],
    factories: &'a BTreeMap<ParserCallbackId, ParserFactoryDisposition>,
    helpers: &'a BTreeMap<String, ParserCallbackId>,
    declarations: &'a [ParserDeclaration],
    dynamic_dependencies: &'a ParserDependencies,
    policy: &'a ParserPolicy,
    capability: ParserCapability,
}
impl ModifierParserData {
    fn definition_view(&self) -> Definitions<'_> {
        Definitions {
            schema_version: self.schema_version,
            source: &self.source,
            dictionaries: &self.dictionaries,
            tables: &self.tables,
            callbacks: &self.callbacks,
            factories: &self.factories,
            helpers: &self.helpers,
            declarations: &self.declarations,
            dynamic_dependencies: &self.dynamic_dependencies,
            policy: &self.policy,
            capability: self.capability,
        }
    }
    /// Exact legacy definition serialization, excluding only program payload.
    pub fn definition_bytes(&self) -> std::result::Result<Vec<u8>, GameDataError> {
        serde_json::to_vec(&self.definition_view()).map_err(|e| GameDataError(e.to_string()))
    }
    /// Hash the borrowed definition projection without allocating its JSON bytes.
    pub fn definition_sha256(&self) -> std::result::Result<String, GameDataError> {
        digest_json(&self.definition_view()).map_err(|e| GameDataError(e.to_string()))
    }
}
