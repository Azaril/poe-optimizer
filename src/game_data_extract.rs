//! Host-side publication for the optional pinned source-data exporter.
use clap::Args as ClapArgs;
use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(ClapArgs)]
pub struct Args {
    #[arg(long, default_value = "vendor/path-of-building-poe2")]
    pub pob: PathBuf,
    /// Includes source verification, process startup, extraction and result validation.
    #[arg(long, default_value_t = 30)]
    pub timeout_seconds: u64,
    /// New canonical package file; also writes <output>.extraction.json evidence.
    #[arg(long)]
    pub output: PathBuf,
}

fn preflight(path: &Path) -> io::Result<PathBuf> {
    let identity = crate::destination_identity(path)?;
    match fs::symlink_metadata(path) {
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Output already exists",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(identity),
        Err(error) => Err(error),
    }
}

pub fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let mut evidence_name = args.output.as_os_str().to_os_string();
    evidence_name.push(".extraction.json");
    let evidence_output = PathBuf::from(evidence_name);
    let package_destination = preflight(&args.output)?;
    if package_destination == preflight(&evidence_output)? {
        return Err("Package and extraction evidence need distinct output paths".into());
    }
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(args.timeout_seconds))
        .ok_or("Extraction timeout is too large")?;
    let extracted = poe_optimizer_pob::game_data_worker::extract_game_data(
        &std::env::current_exe()?,
        &args.pob,
        deadline.saturating_duration_since(Instant::now()),
    )?;
    let bytes = extracted.package.canonical_bytes()?;
    let snapshot =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())?;
    let evidence_bytes = serde_json::to_vec_pretty(&extracted.evidence)?;
    let report = serde_json::json!({
        "schema_version": 1,
        "status": "extracted_game_data",
        "package_sha256": snapshot.identity().content_sha256,
        "package_bytes": bytes.len(),
        "data": snapshot.identity(),
        "section_sha256": extracted.package.manifest.section_sha256,
        "output": args.output,
        "evidence_output": evidence_output,
        "evidence": extracted.evidence,
    });
    let report = serde_json::to_string_pretty(&report)?;
    if Instant::now() >= deadline {
        return Err("Game-data extraction deadline exceeded before publication".into());
    }
    crate::write_new(&args.output, &bytes)?;
    crate::write_new(&evidence_output, &evidence_bytes)?;
    println!("{report}");
    Ok(())
}
