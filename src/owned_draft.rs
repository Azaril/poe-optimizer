//! Host I/O for partial owned documents; Core owns validation and finalization.
use poe_optimizer_core::{
    build_identity::{DraftIssueId, InstanceId},
    owned_build::{OwnedDocument, encode_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_draft::{
        DraftFinalization, DraftIssue, DraftLimits, EvaluationSelection,
        OWNED_DRAFT_SCHEMA_VERSION, decode_draft, encode_draft,
    },
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    error::Error,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// An owned draft JSON envelope.
    input: PathBuf,
    /// Explicit build, scenario and query preset selection JSON.
    #[arg(long)]
    selection: Option<PathBuf>,
    /// Save a complete selected owned request to a new file.
    #[arg(long, requires = "selection")]
    owned_output: Option<PathBuf>,
    /// Save the checked draft to a new file; existing files are preserved.
    #[arg(long)]
    draft_output: Option<PathBuf>,
}

fn read_bounded(path: &Path, maximum: usize, kind: &str) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{kind} input exceeds {maximum} bytes"),
        ));
    }
    Ok(bytes)
}

// Presentation-only indexes over Core's exact diagnostics. No source-name
// classification, selection inference or extra numerical authority is added.
#[derive(Serialize)]
struct IssueSummary<'a> {
    issue_count: usize,
    by_owner: Vec<OwnerIssues>,
    by_code: Vec<CodeCount<'a>>,
}
#[derive(Serialize)]
struct OwnerIssues {
    owner: Option<InstanceId>,
    issue_ids: Vec<DraftIssueId>,
}
#[derive(Serialize)]
struct CodeCount<'a> {
    code: &'a OwnedDefinitionKey,
    count: usize,
}
fn summarize_issues(issues: &[DraftIssue]) -> IssueSummary<'_> {
    let mut owners: BTreeMap<_, Vec<_>> = BTreeMap::new();
    let mut codes = BTreeMap::new();
    // Input is already bounded by Core's DraftLimits. Each issue is indexed once
    // in each grouping, preserving validation order within a given owner.
    for issue in issues {
        owners.entry(issue.owner).or_default().push(issue.id);
        *codes.entry(&issue.code).or_default() += 1;
    }
    IssueSummary {
        issue_count: issues.len(),
        by_owner: owners
            .into_iter()
            .map(|(owner, issue_ids)| OwnerIssues { owner, issue_ids })
            .collect(),
        by_code: codes
            .into_iter()
            .map(|(code, count)| CodeCount { code, count })
            .collect(),
    }
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = DraftLimits::default();
    let bytes = read_bounded(&args.input, limits.input.max_wire_bytes, "draft")?;
    let draft = decode_draft(&bytes, limits)?;
    let validation = draft.validate_limits(limits)?;
    let checked = encode_draft(&draft, limits)?;
    let draft_digest = draft.digest(limits.input.max_wire_bytes)?;
    let finalization = args
        .selection
        .as_ref()
        .map(|path| -> Result<_, Box<dyn Error>> {
            let bytes = read_bounded(path, limits.input.max_wire_bytes, "selection")?;
            // Decode the strict DTO directly so duplicate fields remain errors.
            let selection: EvaluationSelection = serde_json::from_slice(&bytes)?;
            Ok(draft.finalize_selection(selection, limits)?)
        })
        .transpose()?;
    let owned = if args.owned_output.is_some() {
        match &finalization {
            Some(DraftFinalization::Ready(ready)) => Some(encode_owned(
                &OwnedDocument::Request(Box::new(ready.request().clone())),
                limits.input,
            )?),
            Some(DraftFinalization::Pending { .. }) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "selected draft remains pending; cannot write a complete owned request",
                )
                .into());
            }
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "owned output requires an explicit selection",
                )
                .into());
            }
        }
    } else {
        None
    };
    let issue_summary = summarize_issues(&validation.issues);
    let selected_issue_summary = finalization.as_ref().map(|result| match result {
        DraftFinalization::Pending { issues, .. } => summarize_issues(issues),
        DraftFinalization::Ready(_) => summarize_issues(&[]),
    });
    // Prepare every semantic result before any requested file can be created.
    let report = serde_json::json!({
        "schema_version": 2,
        "document_kind": "draft",
        "owned_draft_schema_version": OWNED_DRAFT_SCHEMA_VERSION,
        "draft_digest": draft_digest,
        "checked_bytes": checked.len(),
        "draft_output": &args.draft_output,
        "owned_output": &args.owned_output,
        "issue_count": validation.issues.len(),
        "issues": validation.issues,
        "issue_summary": issue_summary,
        "selected_issue_summary": selected_issue_summary,
        "finalization": finalization,
        "verification": {
            "structure": "valid",
            "definitions": "not_bound",
            "legality": "not_checked",
            "calculation": "not_run"
        }
    });
    let report = serde_json::to_vec_pretty(&report)?;
    if let Some(path) = &args.draft_output {
        super::write_new(path, &checked)?;
    }
    if let (Some(path), Some(bytes)) = (&args.owned_output, &owned) {
        super::write_new(path, bytes)?;
    }
    let mut stdout = io::stdout().lock();
    stdout.write_all(&report)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
