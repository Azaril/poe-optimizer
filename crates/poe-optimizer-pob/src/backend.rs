//! PoB adapter implementing the backend-independent calculation contract.
use poe_optimizer_core::evaluation::*;
use std::{path::PathBuf, time::Duration};

pub struct PobBackend {
    executable: PathBuf,
    source_root: PathBuf,
}
impl PobBackend {
    pub fn new(executable: PathBuf, source_root: PathBuf) -> Self {
        Self {
            executable,
            source_root,
        }
    }
}
impl CalculationBackend for PobBackend {
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            id: "pob-poe2-mlua".into(),
            build_formats: vec![BuildFormat::PathOfBuilding2Xml],
            metrics: crate::metrics::catalog(),
            full_build_evaluation: true,
            skill_selection: true,
            encounter_overrides: true,
        }
    }
    fn calculate(
        &self,
        request: &EvaluationRequest,
        budget: EvaluationBudget,
    ) -> Result<EvaluationResult, EvaluationError> {
        let snapshot = crate::supervisor::evaluate_with_options(
            &self.executable,
            &self.source_root,
            &request.build.content,
            &request.options,
            Duration::from_millis(budget.timeout_ms),
        )
        .map_err(map_supervisor_error)?;

        let measurements = crate::metrics::measurements(&snapshot);
        // Raw numeric fields and Lua runtime metadata are an opaque diagnostic sidecar.
        let diagnostic = serde_json::to_string(&snapshot).map_err(|error| {
            EvaluationError::new(EvaluationErrorKind::BackendContract, error.to_string())
        })?;
        Ok(EvaluationResult {
            backend: BackendIdentity {
                id: "pob-poe2-mlua".into(),
                implementation_version: env!("CARGO_PKG_VERSION").into(),
                rules_revision: snapshot.runtime.upstream_revision,
                source_fingerprint: snapshot.runtime.source_hash,
                adapter_fingerprint: snapshot.runtime.adapter_hash,
            },
            build: snapshot.build,
            context: snapshot.context,
            coverage: snapshot.coverage,
            measurements,
            exports: vec![BuildDocument {
                format: BuildFormat::PathOfBuilding2Xml,
                content: snapshot.export_xml,
            }],
            warnings: snapshot.warnings,
            elapsed_ms: snapshot.elapsed_ms,
            diagnostic_only: true,
            attachments: vec![DiagnosticAttachment {
                media_type: "application/vnd.poe-optimizer.pob-snapshot+json;version=2".into(),
                content: diagnostic,
            }],
        })
    }
}
fn supervisor_error_kind(error: &crate::supervisor::SupervisorErrorKind) -> EvaluationErrorKind {
    use crate::supervisor::SupervisorErrorKind as Supervisor;
    match error {
        Supervisor::InvalidTimeout
        | Supervisor::InputTooLarge
        | Supervisor::OutputTooLarge {
            stage: "request", ..
        } => EvaluationErrorKind::InvalidRequest,
        Supervisor::Timeout { .. } => EvaluationErrorKind::Timeout,
        Supervisor::Serialize(_)
        | Supervisor::OutputTooLarge { .. }
        | Supervisor::IncompleteLine { .. }
        | Supervisor::InvalidJson { .. }
        | Supervisor::ProtocolVersion { .. }
        | Supervisor::RequestId { .. }
        | Supervisor::TrailingOutput
        | Supervisor::MessageOrder => EvaluationErrorKind::BackendContract,
        Supervisor::Worker { code, .. } if code == "protocol_version" => {
            EvaluationErrorKind::BackendContract
        }
        Supervisor::Path { .. }
        | Supervisor::Spawn(_)
        | Supervisor::ThreadSpawn { .. }
        | Supervisor::Io { .. }
        | Supervisor::Exit { .. }
        | Supervisor::Worker { .. }
        | Supervisor::ThreadPanicked => EvaluationErrorKind::CalculationFailed,
    }
}

fn map_supervisor_error(error: crate::supervisor::SupervisorError) -> EvaluationError {
    EvaluationError::new(supervisor_error_kind(&error.kind), error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supervisor::{SupervisorError, SupervisorErrorKind as Supervisor};

    #[test]
    fn supervisor_failures_keep_machine_readable_classification() {
        let cases = [
            (
                Supervisor::InvalidTimeout,
                EvaluationErrorKind::InvalidRequest,
            ),
            (
                Supervisor::InputTooLarge,
                EvaluationErrorKind::InvalidRequest,
            ),
            (
                Supervisor::OutputTooLarge {
                    stage: "request",
                    limit: 1,
                },
                EvaluationErrorKind::InvalidRequest,
            ),
            (
                Supervisor::Timeout {
                    timeout: Duration::from_millis(1),
                },
                EvaluationErrorKind::Timeout,
            ),
            (
                Supervisor::OutputTooLarge {
                    stage: "response",
                    limit: 1,
                },
                EvaluationErrorKind::BackendContract,
            ),
            (
                Supervisor::IncompleteLine { stage: "hello" },
                EvaluationErrorKind::BackendContract,
            ),
            (
                Supervisor::InvalidJson {
                    stage: "response",
                    source: serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
                },
                EvaluationErrorKind::BackendContract,
            ),
            (
                Supervisor::ProtocolVersion {
                    stage: "response",
                    actual: 999,
                },
                EvaluationErrorKind::BackendContract,
            ),
            (
                Supervisor::RequestId { actual: 99 },
                EvaluationErrorKind::BackendContract,
            ),
            (
                Supervisor::TrailingOutput,
                EvaluationErrorKind::BackendContract,
            ),
            (
                Supervisor::MessageOrder,
                EvaluationErrorKind::BackendContract,
            ),
            (
                Supervisor::Worker {
                    code: "protocol_version".into(),
                    message: "wrong request version".into(),
                },
                EvaluationErrorKind::BackendContract,
            ),
            (
                Supervisor::Worker {
                    code: "evaluation_failed".into(),
                    message: "upstream calculation failed".into(),
                },
                EvaluationErrorKind::CalculationFailed,
            ),
            (
                Supervisor::Spawn(std::io::Error::other("start failed")),
                EvaluationErrorKind::CalculationFailed,
            ),
            (
                Supervisor::ThreadPanicked,
                EvaluationErrorKind::CalculationFailed,
            ),
        ];
        for (failure, expected) in cases {
            let result = map_supervisor_error(failure.into());
            assert_eq!(result.kind, expected, "{}", result.message);
            assert!(!result.message.is_empty());
        }
    }

    #[test]
    fn timeout_translation_retains_diagnostics_and_cleanup_error() {
        let error = SupervisorError {
            kind: Supervisor::Timeout {
                timeout: Duration::from_millis(25),
            },
            stderr: "last worker diagnostic".into(),
            stderr_truncated: true,
            cleanup_error: Some("cleanup failed".into()),
        };
        let mapped = map_supervisor_error(error);
        assert_eq!(mapped.kind, EvaluationErrorKind::Timeout);
        assert!(mapped.message.contains("last worker diagnostic"));
        assert!(mapped.message.contains("truncated"));
        assert!(mapped.message.contains("cleanup failed"));
    }
}
