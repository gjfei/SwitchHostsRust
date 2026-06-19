use crate::hosts_apply::aggregate::aggregate_selected_content;
use crate::hosts_apply::cmd_runner::run_after_apply;
use crate::hosts_apply::elevation::{ElevationBackend, SystemElevation};
use crate::hosts_apply::history::append_history;
use crate::hosts_apply::target::HostsTarget;
use crate::hosts_apply::write::write_hosts;
use crate::hosts_apply::error::ApplyError;
use crate::storage::config::AppConfig;
use crate::storage::manifest::Manifest;
use crate::storage::paths::AppPaths;

pub struct ApplyPipeline<'a> {
    pub paths: &'a AppPaths,
    pub config: &'a AppConfig,
    pub elevation: &'a dyn ElevationBackend,
}

impl<'a> ApplyPipeline<'a> {
    pub fn default_elevation() -> SystemElevation {
        SystemElevation
    }

    pub fn apply(&self, manifest: &Manifest, target: &HostsTarget) -> Result<ApplyResult, ApplyError> {
        let content = aggregate_selected_content(
            &manifest.root,
            self.paths,
            self.config.remove_duplicate_records,
        )?;

        let written = write_hosts(
            target,
            &content,
            &self.config.write_mode,
            self.elevation,
        )?;

        if written {
            append_history(
                &self.paths.histories_dir,
                &content,
                self.config.history_limit,
            )?;
            run_after_apply(&self.config.cmd_after_hosts_apply, &self.paths.histories_dir)?;
        }

        Ok(ApplyResult {
            written,
            target_path: target.path(),
            content,
        })
    }
}

#[derive(Debug)]
pub struct ApplyResult {
    pub written: bool,
    pub target_path: std::path::PathBuf,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hosts_apply::elevation::MockElevation;
    use crate::hosts_apply::target::HostsTarget;
    use crate::storage::entries;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn full_apply_pipeline() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        paths.ensure_layout().unwrap();
        entries::write_entry(&paths.entries_dir, "1", "127.0.0.1 pip.test\n").unwrap();

        let manifest = Manifest {
            root: json!([{ "id": "1", "type": "local", "on": true }])
                .as_array()
                .cloned()
                .unwrap(),
            ..Default::default()
        };
        let config = AppConfig::default();
        let target = HostsTarget::File(paths.dev_test_hosts.clone());
        let pipeline = ApplyPipeline {
            paths: &paths,
            config: &config,
            elevation: &MockElevation,
        };
        let result = pipeline.apply(&manifest, &target).unwrap();
        assert!(result.written);
        assert!(std::fs::read_to_string(result.target_path)
            .unwrap()
            .contains("pip.test"));
    }
}
