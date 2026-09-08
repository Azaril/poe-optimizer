//! Host-side byte acquisition for portable native data packages.
use poe_optimizer_data::game_data::{
    GameDataLoader, LoadLimits, TrustPolicy, bundled_package_sha256,
};
use poe_optimizer_native::{CompiledGameData, HostClock, NativeBackend};
use sha2::{Digest, Sha256};
use std::{error::Error, fs::File, io::Read, path::PathBuf, sync::Arc};

#[derive(clap::Args, Default)]
pub(crate) struct DataArgs {
    /// Native game-data JSON package. Omit to use the reviewed packaged default.
    #[arg(long)]
    pub data: Option<PathBuf>,
    /// Expected SHA-256 from an external review; package claims cannot supply trust.
    #[arg(long, requires = "data")]
    pub data_sha256: Option<String>,
}
impl DataArgs {
    #[cfg(feature = "pob")]
    pub fn is_selected(&self) -> bool {
        self.data.is_some() || self.data_sha256.is_some()
    }
    pub fn load(&self) -> Result<Arc<CompiledGameData>, Box<dyn Error>> {
        let Some(path) = &self.data else {
            return Ok(CompiledGameData::bundled()?);
        };
        let limits = LoadLimits::default();
        if !std::fs::metadata(path)?.is_file() {
            return Err("Game-data input must be a regular file".into());
        }
        let mut bytes = Vec::new();
        File::open(path)?
            .take(limits.max_bytes as u64 + 1)
            .read_to_end(&mut bytes)?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let policy = if let Some(expected) = &self.data_sha256 {
            TrustPolicy::Reviewed {
                expected_sha256: expected.clone(),
            }
        } else if digest == bundled_package_sha256() {
            TrustPolicy::Reviewed {
                expected_sha256: bundled_package_sha256().into(),
            }
        } else {
            TrustPolicy::AllowCustom
        };
        let snapshot = GameDataLoader::from_bytes(&bytes, &policy, &limits)?;
        Ok(Arc::new(CompiledGameData::compile(Arc::new(snapshot))?))
    }
    pub fn backend(&self) -> Result<NativeBackend, Box<dyn Error>> {
        Ok(NativeBackend::with_data(self.load()?, HostClock)?)
    }
}
