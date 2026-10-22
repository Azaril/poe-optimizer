//! Bounded, transactional assembly of existing Core rule ownership.
//!
//! Sources are injected text. This layer neither opens files nor grants rule
//! authority: the complete result must still pass the Engine package compiler.
use crate::{
    Diagnostic, LoweredProgram, MAX_PARSE_DEPTH, ProgramLowerLimits, ProgramParseLimits,
    SymbolLimits, SymbolRegistry, lower_program, parse_program,
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_definitions::{GameVersionNamespace, OwnedDefinitionKey},
    owned_rules::{IntegerRuleTable, RulePackageInput, RuleProgram},
    owned_schema::{DefinitionAddress, DefinitionSchemaIndex, SchemaSubject, SlotAddress},
};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PacketManifest {
    pub id: OwnedDefinitionKey,
    pub appends: Vec<PacketAppend>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PacketAppend {
    pub owner: SchemaSubject,
    pub programs: Vec<PacketProgram>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PacketProgram {
    pub id: OwnedDefinitionKey,
    /// An exact virtual source label, not a filesystem instruction.
    pub source: String,
}

#[derive(Clone, Copy, Debug)]
pub struct RuleSource<'a> {
    pub name: &'a str,
    pub text: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct PacketLimits {
    pub parse: ProgramParseLimits,
    pub lower: ProgramLowerLimits,
    pub max_sources: usize,
    /// Sum of all injected source text bytes, including comments and whitespace.
    pub max_source_bytes: usize,
    pub max_name_bytes: usize,
    pub max_symbols: usize,
    pub max_appends: usize,
    /// Expanded membership counts: reusing a source charges every occurrence.
    pub max_programs: usize,
    /// Expanded source arena size bounds operand/target lists as well as nodes.
    pub max_syntax_nodes: usize,
    pub max_nodes: usize,
    pub max_reads: usize,
    pub max_effects: usize,
    /// Bounds the existing inventory inspected before transactional application.
    pub max_base_owners: usize,
    pub max_base_programs: usize,
}

impl Default for PacketLimits {
    fn default() -> Self {
        Self {
            parse: ProgramParseLimits::default(),
            lower: ProgramLowerLimits::default(),
            max_sources: 4096,
            max_source_bytes: 16 * 1024 * 1024,
            max_name_bytes: 256,
            max_symbols: 8192,
            max_appends: 4096,
            max_programs: 16 * 1024,
            max_syntax_nodes: 256 * 1024,
            max_nodes: 256 * 1024,
            max_reads: 64 * 1024,
            max_effects: 64 * 1024,
            max_base_owners: 16 * 1024,
            max_base_programs: 64 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketLimit {
    Sources,
    SourceBytes,
    NameBytes,
    Symbols,
    Tables,
    Appends,
    Programs,
    SyntaxNodes,
    Nodes,
    Reads,
    Effects,
    BaseOwners,
    BasePrograms,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketLocation {
    Packet,
    Source(usize),
    Append(usize),
    Member { append: usize, program: usize },
    BaseOwner(usize),
    BaseProgram { owner: usize, program: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketErrorKind {
    InvalidLimits,
    EmptyPacket,
    EmptyAppend,
    LimitExceeded(PacketLimit),
    InvalidIdentity,
    IdentityMismatch,
    NamespaceMismatch,
    InvalidSourceName,
    DuplicateSource,
    MissingSource,
    UnusedSource,
    SourceProgramMismatch,
    InvalidSymbols,
    UnknownOwner,
    DuplicateOwner,
    DuplicateProgram,
    SourceDiagnostic,
}

/// Locations refer to caller-provided vectors. A source diagnostic retains its
/// original byte span; no error copies arbitrary source labels or text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PacketError {
    pub kind: PacketErrorKind,
    pub location: PacketLocation,
    pub diagnostic: Option<Diagnostic>,
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {:?}", self.kind, self.location)?;
        if let Some(diagnostic) = &self.diagnostic {
            write!(f, ": {diagnostic}")?;
        }
        Ok(())
    }
}

impl std::error::Error for PacketError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.diagnostic
            .as_ref()
            .map(|diagnostic| diagnostic as &dyn std::error::Error)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoweredPacketProgram {
    pub source: String,
    pub lowered: LoweredProgram,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredPacketAppend {
    pub owner: SchemaSubject,
    /// Indices into `LoweredPacket::programs`, in manifest membership order.
    pub programs: Vec<usize>,
}

/// Unchecked Core programs plus explicit membership additions. Fields are
/// immutable so application can rely on the bounded, validated assembly plan.
#[derive(Debug)]
pub struct LoweredPacket {
    id: OwnedDefinitionKey,
    definitions: DataIdentity,
    namespace: GameVersionNamespace,
    programs: Vec<LoweredPacketProgram>,
    appends: Vec<LoweredPacketAppend>,
    uses: Vec<usize>,
    limits: PacketLimits,
}

fn error(kind: PacketErrorKind, location: PacketLocation) -> PacketError {
    PacketError {
        kind,
        location,
        diagnostic: None,
    }
}

fn cap(
    value: usize,
    maximum: usize,
    kind: PacketLimit,
    at: PacketLocation,
) -> Result<(), PacketError> {
    if value > maximum {
        Err(error(PacketErrorKind::LimitExceeded(kind), at))
    } else {
        Ok(())
    }
}

fn charge(
    total: &mut usize,
    amount: usize,
    maximum: usize,
    kind: PacketLimit,
    at: PacketLocation,
) -> Result<(), PacketError> {
    *total = total
        .checked_add(amount)
        .filter(|value| *value <= maximum)
        .ok_or_else(|| error(PacketErrorKind::LimitExceeded(kind), at))?;
    Ok(())
}

fn expanded(
    total: &mut usize,
    amount: usize,
    uses: usize,
    maximum: usize,
    kind: PacketLimit,
    at: PacketLocation,
) -> Result<(), PacketError> {
    let amount = amount
        .checked_mul(uses)
        .ok_or_else(|| error(PacketErrorKind::LimitExceeded(kind), at))?;
    charge(total, amount, maximum, kind, at)
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SubjectRef<'a> {
    Definition(&'a DefinitionAddress),
    Slot(&'a SlotAddress),
}

impl<'a> From<&'a SchemaSubject> for SubjectRef<'a> {
    fn from(value: &'a SchemaSubject) -> Self {
        match value {
            SchemaSubject::Definition(address) => Self::Definition(address),
            SchemaSubject::Slot(address) => Self::Slot(address),
        }
    }
}

fn check_owner<I: DefinitionSchemaIndex>(
    owner: &SchemaSubject,
    index: &I,
    at: PacketLocation,
) -> Result<(), PacketError> {
    // Reuse SymbolRegistry's sealed typed lookup checks, including a slot's
    // exact declaring owner. This establishes existence, never rule permission.
    let limits = SymbolLimits {
        max_symbols: 1,
        max_name_bytes: 5,
    };
    let result = match owner {
        SchemaSubject::Definition(address) => {
            SymbolRegistry::new(index, &[("owner", address.clone())], &[], limits)
        }
        SchemaSubject::Slot(address) => {
            SymbolRegistry::new(index, &[], &[("owner", address.clone())], limits)
        }
    };
    result
        .map(|_| ())
        .map_err(|_| error(PacketErrorKind::UnknownOwner, at))
}

fn validate_limits(limits: PacketLimits) -> Result<(), PacketError> {
    if [
        limits.max_sources,
        limits.max_source_bytes,
        limits.max_name_bytes,
        limits.max_symbols,
        limits.max_appends,
        limits.max_programs,
        limits.max_syntax_nodes,
        limits.max_nodes,
        limits.max_reads,
        limits.max_effects,
        limits.max_base_owners,
        limits.max_base_programs,
        limits.lower.formulas.max_syntax_nodes,
        limits.lower.formulas.max_nodes,
        limits.lower.formulas.max_statements,
        limits.lower.bindings.max_bindings,
        limits.lower.bindings.max_name_bytes,
    ]
    .contains(&0)
        || limits.parse.expressions.max_depth > MAX_PARSE_DEPTH
    {
        Err(error(
            PacketErrorKind::InvalidLimits,
            PacketLocation::Packet,
        ))
    } else {
        Ok(())
    }
}

/// Lower each supplied source once, then retain explicit owner-local membership.
/// Every supplied source must be used; repeated references under distinct owners
/// are allowed and charged at their expanded size. No package is mutated here.
pub fn lower_packet<I: DefinitionSchemaIndex>(
    manifest: &PacketManifest,
    sources: &[RuleSource<'_>],
    index: &I,
    symbols: &SymbolRegistry,
    tables: &[IntegerRuleTable],
    limits: PacketLimits,
) -> Result<LoweredPacket, PacketError> {
    validate_limits(limits)?;
    if manifest.appends.is_empty() {
        return Err(error(PacketErrorKind::EmptyPacket, PacketLocation::Packet));
    }
    cap(
        sources.len(),
        limits.max_sources,
        PacketLimit::Sources,
        PacketLocation::Packet,
    )?;
    cap(
        manifest.appends.len(),
        limits.max_appends,
        PacketLimit::Appends,
        PacketLocation::Packet,
    )?;
    cap(
        tables.len(),
        limits.lower.bindings.max_bindings,
        PacketLimit::Tables,
        PacketLocation::Packet,
    )?;
    let mut symbol_count = 0;
    for name in symbols
        .definitions()
        .map(|(name, _)| name)
        .chain(symbols.slots().map(|(name, _)| name))
    {
        charge(
            &mut symbol_count,
            1,
            limits.max_symbols,
            PacketLimit::Symbols,
            PacketLocation::Packet,
        )?;
        cap(
            name.len(),
            limits.max_name_bytes,
            PacketLimit::NameBytes,
            PacketLocation::Packet,
        )?;
    }
    symbols
        .validate_against(index)
        .map_err(|_| error(PacketErrorKind::InvalidSymbols, PacketLocation::Packet))?;
    index
        .identity()
        .validate()
        .map_err(|_| error(PacketErrorKind::InvalidIdentity, PacketLocation::Packet))?;

    let mut source_indices = BTreeMap::new();
    let mut source_bytes = 0;
    for (position, source) in sources.iter().enumerate() {
        let at = PacketLocation::Source(position);
        cap(
            source.name.len(),
            limits.max_name_bytes,
            PacketLimit::NameBytes,
            at,
        )?;
        if source.name.is_empty() {
            return Err(error(PacketErrorKind::InvalidSourceName, at));
        }
        charge(
            &mut source_bytes,
            source.text.len(),
            limits.max_source_bytes,
            PacketLimit::SourceBytes,
            at,
        )?;
        if source_indices.insert(source.name, position).is_some() {
            return Err(error(PacketErrorKind::DuplicateSource, at));
        }
    }
    let mut uses = vec![0usize; sources.len()];
    let mut expected_ids = vec![None; sources.len()];
    let mut members = BTreeMap::new();
    let mut program_count = 0;
    // Bound the whole manifest before allocating a resolved membership vector.
    for (position, append) in manifest.appends.iter().enumerate() {
        let at = PacketLocation::Append(position);
        if append.programs.is_empty() {
            return Err(error(PacketErrorKind::EmptyAppend, at));
        }
        charge(
            &mut program_count,
            append.programs.len(),
            limits.max_programs,
            PacketLimit::Programs,
            at,
        )?;
    }
    let mut appends = Vec::with_capacity(manifest.appends.len());
    for (position, append) in manifest.appends.iter().enumerate() {
        check_owner(&append.owner, index, PacketLocation::Append(position))?;
        let mut programs = Vec::with_capacity(append.programs.len());
        for (member, program) in append.programs.iter().enumerate() {
            let at = PacketLocation::Member {
                append: position,
                program: member,
            };
            cap(
                program.source.len(),
                limits.max_name_bytes,
                PacketLimit::NameBytes,
                at,
            )?;
            if members
                .insert((SubjectRef::from(&append.owner), &program.id), ())
                .is_some()
            {
                return Err(error(PacketErrorKind::DuplicateProgram, at));
            }
            let source = source_indices
                .get(program.source.as_str())
                .copied()
                .ok_or_else(|| error(PacketErrorKind::MissingSource, at))?;
            if expected_ids[source].is_some_and(|id| id != &program.id) {
                return Err(error(PacketErrorKind::SourceProgramMismatch, at));
            }
            expected_ids[source] = Some(&program.id);
            uses[source] += 1; // The total was checked against max_programs above.
            programs.push(source);
        }
        appends.push(LoweredPacketAppend {
            owner: append.owner.clone(),
            programs,
        });
    }
    for (position, uses) in uses.iter().enumerate() {
        if *uses == 0 {
            return Err(error(
                PacketErrorKind::UnusedSource,
                PacketLocation::Source(position),
            ));
        }
    }
    let mut programs = Vec::with_capacity(sources.len());
    let (mut syntax_nodes, mut nodes, mut reads, mut effects) = (0, 0, 0, 0);
    for (position, source) in sources.iter().enumerate() {
        let at = PacketLocation::Source(position);
        let diagnostic = |diagnostic| PacketError {
            kind: PacketErrorKind::SourceDiagnostic,
            location: at,
            diagnostic: Some(diagnostic),
        };
        let parsed = parse_program(source.text, limits.parse).map_err(diagnostic)?;
        if parsed.text(parsed.id()) != expected_ids[position].map(|id| id.as_str()) {
            return Err(error(PacketErrorKind::SourceProgramMismatch, at));
        }
        expanded(
            &mut syntax_nodes,
            parsed.nodes().len(),
            uses[position],
            limits.max_syntax_nodes,
            PacketLimit::SyntaxNodes,
            at,
        )?;
        let lowered =
            lower_program(&parsed, index, symbols, tables, limits.lower).map_err(diagnostic)?;
        expanded(
            &mut nodes,
            lowered.program.nodes.len(),
            uses[position],
            limits.max_nodes,
            PacketLimit::Nodes,
            at,
        )?;
        expanded(
            &mut reads,
            lowered.program.reads.len(),
            uses[position],
            limits.max_reads,
            PacketLimit::Reads,
            at,
        )?;
        expanded(
            &mut effects,
            lowered.program.effects.len(),
            uses[position],
            limits.max_effects,
            PacketLimit::Effects,
            at,
        )?;
        programs.push(LoweredPacketProgram {
            source: source.name.to_owned(),
            lowered,
        });
    }
    Ok(LoweredPacket {
        id: manifest.id.clone(),
        definitions: index.identity().clone(),
        namespace: index.namespace().clone(),
        programs,
        appends,
        uses,
        limits,
    })
}

impl LoweredPacket {
    pub fn id(&self) -> &OwnedDefinitionKey {
        &self.id
    }
    pub fn definitions(&self) -> &DataIdentity {
        &self.definitions
    }
    pub fn namespace(&self) -> &GameVersionNamespace {
        &self.namespace
    }
    pub fn programs(&self) -> &[LoweredPacketProgram] {
        &self.programs
    }
    pub fn appends(&self) -> &[LoweredPacketAppend] {
        &self.appends
    }

    /// Append transactionally to existing owners. Existing members, closure/gaps,
    /// ordering and every unrelated package field are preserved. No owner is
    /// created and no coverage declaration is completed. All fallible structural
    /// checks finish before the first mutation; Engine validation is still required.
    pub fn apply_to(self, target: &mut RulePackageInput) -> Result<(), PacketError> {
        if target.namespace != self.namespace {
            return Err(error(
                PacketErrorKind::NamespaceMismatch,
                PacketLocation::Packet,
            ));
        }
        if target.definitions != self.definitions {
            return Err(error(
                PacketErrorKind::IdentityMismatch,
                PacketLocation::Packet,
            ));
        }
        cap(
            target.owners.len(),
            self.limits.max_base_owners,
            PacketLimit::BaseOwners,
            PacketLocation::Packet,
        )?;
        let mut base_programs = 0;
        for (owner, row) in target.owners.iter().enumerate() {
            charge(
                &mut base_programs,
                row.programs.members.len(),
                self.limits.max_base_programs,
                PacketLimit::BasePrograms,
                PacketLocation::BaseOwner(owner),
            )?;
        }
        let mut owners = BTreeMap::new();
        let mut members = BTreeMap::new();
        for (owner, row) in target.owners.iter().enumerate() {
            let key = SubjectRef::from(&row.owner);
            if owners.insert(key, owner).is_some() {
                return Err(error(
                    PacketErrorKind::DuplicateOwner,
                    PacketLocation::BaseOwner(owner),
                ));
            }
            for (program, value) in row.programs.members.iter().enumerate() {
                if members.insert((key, &value.id), ()).is_some() {
                    return Err(error(
                        PacketErrorKind::DuplicateProgram,
                        PacketLocation::BaseProgram { owner, program },
                    ));
                }
            }
        }
        let mut destinations = Vec::with_capacity(self.appends.len());
        for (append, row) in self.appends.iter().enumerate() {
            let key = SubjectRef::from(&row.owner);
            let owner = owners.get(&key).copied().ok_or_else(|| {
                error(
                    PacketErrorKind::UnknownOwner,
                    PacketLocation::Append(append),
                )
            })?;
            for (program, source) in row.programs.iter().enumerate() {
                let id = &self.programs[*source].lowered.program.id;
                if members.contains_key(&(key, id)) {
                    return Err(error(
                        PacketErrorKind::DuplicateProgram,
                        PacketLocation::Member { append, program },
                    ));
                }
            }
            destinations.push(owner);
        }
        // Stop borrowing the target before constructing the bounded move plan.
        drop(members);
        drop(owners);
        let mut programs: Vec<Option<RuleProgram>> = self
            .programs
            .into_iter()
            .map(|source| Some(source.lowered.program))
            .collect();
        let mut uses = self.uses;
        let mut additions = Vec::with_capacity(self.appends.len());
        for (append, destination) in self.appends.into_iter().zip(destinations) {
            let mut values = Vec::with_capacity(append.programs.len());
            for source in append.programs {
                uses[source] -= 1;
                let value = if uses[source] == 0 {
                    programs[source].take().expect("validated final source use")
                } else {
                    programs[source]
                        .as_ref()
                        .expect("validated repeated source use")
                        .clone()
                };
                values.push(value);
            }
            additions.push((destination, values));
        }
        // There are no Result-producing operations beyond this commit point.
        for (owner, programs) in additions {
            target.owners[owner].programs.members.extend(programs);
        }
        Ok(())
    }
}
