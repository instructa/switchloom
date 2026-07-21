use crate::contracts::*;
use crate::error::{Result, ResultContext};
use crate::{bail, product_error};
use crate::{config::*, registry::*, routing::*};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const CODEX_CONFIG_PATH: &str = ".codex/config.toml";
pub(crate) const MANIFEST_PATH: &str = ".model-routing/manifest.json";
pub(crate) const TRANSACTION_JOURNAL: &str = "journal.json";
thread_local! {
    pub(crate) static TRANSACTION_FAIL_AFTER_WRITES: Cell<usize> = const { Cell::new(0) };
    pub(crate) static TRANSACTION_FAIL_JOURNAL_REPLACE_AFTER: Cell<usize> = const { Cell::new(0) };
    pub(crate) static TRANSACTION_RETURN_JOURNAL_ERROR_AFTER: Cell<usize> = const { Cell::new(0) };
    pub(crate) static TRANSACTION_RETURN_STAGED_RENAME_ERROR_AFTER: Cell<usize> = const { Cell::new(0) };
    pub(crate) static TRANSACTION_FAIL_RESTORE: Cell<bool> = const { Cell::new(false) };
}

#[cfg(test)]
#[path = "tests/lifecycle.rs"]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleReport {
    pub action: String,
    pub bundle_id: Option<String>,
    pub repository: String,
    pub artifacts: Vec<LifecycleArtifactReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleArtifactReport {
    pub path: String,
    pub mode: String,
    pub status: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<String>,
}

pub struct PreparedSetupLifecycle {
    bundle: RoutingBundleV1,
    bundle_input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagedManifest {
    pub(crate) schema_version: u32,
    pub(crate) bundle_id: String,
    pub(crate) bundle_sha256: String,
    pub(crate) artifacts: Vec<ManagedArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) previous: Option<ManagedSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagedArtifact {
    pub(crate) path: String,
    pub(crate) sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagedSnapshot {
    pub(crate) bundle_id: String,
    pub(crate) bundle_sha256: String,
    pub(crate) artifacts: Vec<ManagedArtifact>,
}

pub fn preview_setup_config_file(repository: &Path, config_file: &Path) -> Result<LifecycleReport> {
    let spec = read_setup_config_file(config_file)?;
    preview_setup(repository, &spec)
}

pub fn preview_setup_recipe(repository: &Path, recipe: &str) -> Result<LifecycleReport> {
    let spec = setup_spec_from_recipe(recipe)?;
    preview_setup(repository, &spec)
}

pub fn preview_saved_setup(repository: &Path) -> Result<LifecycleReport> {
    let spec = read_saved_setup_config(repository)?;
    preview_setup(repository, &spec)
}

pub fn apply_setup_config_file(repository: &Path, config_file: &Path) -> Result<LifecycleReport> {
    let spec = read_setup_config_file(config_file)?;
    apply_setup(repository, &spec)
}

pub fn apply_setup_recipe(repository: &Path, recipe: &str) -> Result<LifecycleReport> {
    let spec = setup_spec_from_recipe(recipe)?;
    apply_setup(repository, &spec)
}

pub fn apply_saved_setup(repository: &Path) -> Result<LifecycleReport> {
    let spec = read_saved_setup_config(repository)?;
    apply_setup(repository, &spec)
}

pub fn update_setup_config_file(repository: &Path, config_file: &Path) -> Result<LifecycleReport> {
    let spec = read_setup_config_file(config_file)?;
    update_setup(repository, &spec)
}

pub fn update_setup_recipe(repository: &Path, recipe: &str) -> Result<LifecycleReport> {
    let spec = setup_spec_from_recipe(recipe)?;
    update_setup(repository, &spec)
}

pub fn update_saved_setup(repository: &Path) -> Result<LifecycleReport> {
    let spec = read_saved_setup_config(repository)?;
    update_setup(repository, &spec)
}

pub fn prepare_setup_config_file(config_file: &Path) -> Result<PreparedSetupLifecycle> {
    prepare_setup_lifecycle(&read_setup_config_file(config_file)?)
}

pub fn prepare_setup_recipe(recipe: &str) -> Result<PreparedSetupLifecycle> {
    prepare_setup_lifecycle(&setup_spec_from_recipe(recipe)?)
}

pub fn prepare_saved_setup(repository: &Path) -> Result<PreparedSetupLifecycle> {
    prepare_setup_lifecycle(&read_saved_setup_config(repository)?)
}

pub fn preview_prepared_setup(
    repository: &Path,
    prepared: &PreparedSetupLifecycle,
) -> Result<LifecycleReport> {
    preview_bundle(repository, &prepared.bundle)
}

pub fn apply_prepared_setup(
    repository: &Path,
    prepared: &PreparedSetupLifecycle,
    confirmed_preview: &LifecycleReport,
) -> Result<LifecycleReport> {
    let current_preview = preview_prepared_setup(repository, prepared)?;
    if !same_lifecycle_plan(&current_preview, confirmed_preview) {
        bail!("repository state changed after preview; rerun preview/apply and confirm again");
    }
    apply_bundle_json(
        Path::new(&confirmed_preview.repository),
        &prepared.bundle,
        &prepared.bundle_input,
    )
}

pub fn preview_bundle_file(repository: &Path, bundle_file: &Path) -> Result<LifecycleReport> {
    let bundle = read_bundle_file(bundle_file)?;
    preview_bundle(repository, &bundle)
}

pub fn apply_bundle_file(repository: &Path, bundle_file: &Path) -> Result<LifecycleReport> {
    let bundle_input = fs::read_to_string(bundle_file)
        .with_context(|| format!("failed to read bundle `{}`", bundle_file.display()))?;
    let bundle = validate_bundle_json(&bundle_input)?;
    apply_bundle_json(repository, &bundle, &bundle_input)
}

pub(crate) fn apply_bundle_json(
    repository: &Path,
    bundle: &RoutingBundleV1,
    bundle_input: &str,
) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let planned = plan_artifacts(&repository, bundle, None)?;
    ensure_apply_is_safe(&planned)?;
    let manifest = manifest_from_bundle(bundle, sha256(bundle_input.as_bytes()), None);
    commit_transaction(&repository, &planned, &manifest)?;
    Ok(report_from_plan(
        "apply",
        &repository,
        Some(&bundle.bundle_id),
        &planned,
    ))
}

pub fn update_bundle_file(repository: &Path, bundle_file: &Path) -> Result<LifecycleReport> {
    let bundle_input = fs::read_to_string(bundle_file)
        .with_context(|| format!("failed to read bundle `{}`", bundle_file.display()))?;
    let bundle = validate_bundle_json(&bundle_input)?;
    update_bundle_json(repository, &bundle, &bundle_input)
}

pub(crate) fn update_bundle_json(
    repository: &Path,
    bundle: &RoutingBundleV1,
    bundle_input: &str,
) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let current = read_manifest(&repository)?
        .ok_or_else(|| product_error!("no model-routing manifest found"))?;
    let planned = plan_artifacts(&repository, bundle, Some(&current))?;
    ensure_update_is_safe(&planned)?;
    let manifest = manifest_from_plan(
        &bundle.bundle_id,
        sha256(bundle_input.as_bytes()),
        &planned,
        Some(snapshot_from_manifest(&current)),
    );
    commit_transaction(&repository, &planned, &manifest)?;
    Ok(report_from_plan(
        "update",
        &repository,
        Some(&bundle.bundle_id),
        &planned,
    ))
}

pub fn status_repository(repository: &Path) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let Some(manifest) = read_manifest(&repository)? else {
        return Ok(LifecycleReport {
            action: "status".to_string(),
            bundle_id: None,
            repository: repository.display().to_string(),
            artifacts: Vec::new(),
        });
    };
    let mut reports = Vec::new();
    for artifact in &manifest.artifacts {
        let target = resolve_repository_target(&repository, &artifact.path)?;
        let status = status_for_managed_artifact(&target, artifact)?;
        reports.push(LifecycleArtifactReport {
            path: artifact.path.clone(),
            mode: "managed".to_string(),
            status: status.to_string(),
            sha256: artifact.sha256.clone(),
            repair: repair_for_status(status),
        });
    }
    Ok(LifecycleReport {
        action: "status".to_string(),
        bundle_id: Some(manifest.bundle_id),
        repository: repository.display().to_string(),
        artifacts: reports,
    })
}

pub fn uninstall_repository(repository: &Path) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let manifest = read_manifest(&repository)?
        .ok_or_else(|| product_error!("no model-routing manifest found"))?;
    let mut reports = Vec::new();
    for artifact in &manifest.artifacts {
        let target = resolve_repository_target(&repository, &artifact.path)?;
        let status = uninstall_managed_artifact(&target, artifact)?;
        reports.push(LifecycleArtifactReport {
            path: artifact.path.clone(),
            mode: "managed".to_string(),
            status: status.to_string(),
            sha256: artifact.sha256.clone(),
            repair: repair_for_status(status),
        });
    }
    let residual_artifacts = manifest
        .artifacts
        .iter()
        .zip(reports.iter())
        .filter(|(_, report)| report.status != "removed")
        .map(|(artifact, _)| ManagedArtifact {
            path: artifact.path.clone(),
            sha256: artifact.sha256.clone(),
            content: artifact.content.clone(),
        })
        .collect::<Vec<_>>();
    if residual_artifacts.is_empty() {
        remove_manifest(&repository)?;
    } else {
        let residual = ManagedManifest {
            schema_version: 1,
            bundle_id: manifest.bundle_id.clone(),
            bundle_sha256: manifest.bundle_sha256.clone(),
            artifacts: residual_artifacts,
            previous: manifest.previous.clone(),
        };
        write_manifest_file(&repository, &residual)?;
    }
    Ok(LifecycleReport {
        action: "uninstall".to_string(),
        bundle_id: Some(manifest.bundle_id),
        repository: repository.display().to_string(),
        artifacts: reports,
    })
}

pub fn rollback_repository(repository: &Path) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let manifest = read_manifest(&repository)?
        .ok_or_else(|| product_error!("no model-routing manifest found"))?;
    let previous = manifest
        .previous
        .clone()
        .ok_or_else(|| product_error!("no rollback snapshot found"))?;
    let planned = plan_rollback_artifacts(&repository, &manifest, &previous)?;
    ensure_update_is_safe(&planned)?;
    let rollback_manifest = manifest_from_plan(
        &previous.bundle_id,
        previous.bundle_sha256.clone(),
        &planned,
        None,
    );
    commit_transaction(&repository, &planned, &rollback_manifest)?;
    Ok(report_from_plan(
        "rollback",
        &repository,
        Some(&rollback_manifest.bundle_id),
        &planned,
    ))
}

pub(crate) fn read_bundle_file(bundle_file: &Path) -> Result<RoutingBundleV1> {
    let input = fs::read_to_string(bundle_file)
        .with_context(|| format!("failed to read bundle `{}`", bundle_file.display()))?;
    validate_bundle_json(&input)
}

pub(crate) fn read_setup_config_file(config_file: &Path) -> Result<SetupSpecV1> {
    let input = fs::read_to_string(config_file)
        .with_context(|| format!("failed to read setup config `{}`", config_file.display()))?;
    setup_spec_from_toml(&input)
}

pub(crate) fn read_saved_setup_config(repository: &Path) -> Result<SetupSpecV1> {
    let repository = canonicalize_existing_repository(repository)?;
    let config_path = repository.join(SETUP_CONFIG_PATH);
    read_setup_config_file(&config_path)
}

pub(crate) fn preview_setup(repository: &Path, spec: &SetupSpecV1) -> Result<LifecycleReport> {
    let prepared = prepare_setup_lifecycle(spec)?;
    preview_bundle(repository, &prepared.bundle)
}

pub(crate) fn apply_setup(repository: &Path, spec: &SetupSpecV1) -> Result<LifecycleReport> {
    let prepared = prepare_setup_lifecycle(spec)?;
    apply_bundle_json(repository, &prepared.bundle, &prepared.bundle_input)
}

pub(crate) fn update_setup(repository: &Path, spec: &SetupSpecV1) -> Result<LifecycleReport> {
    let prepared = prepare_setup_lifecycle(spec)?;
    update_bundle_json(repository, &prepared.bundle, &prepared.bundle_input)
}

pub(crate) fn prepare_setup_lifecycle(spec: &SetupSpecV1) -> Result<PreparedSetupLifecycle> {
    let normalized_config = setup_spec_to_canonical_toml(spec)?;
    let mut bundle = compile_setup_spec(spec)?;
    bundle.artifacts.push(bundle_artifact(SourceArtifact {
        path: SETUP_CONFIG_PATH.to_string(),
        media_type: "application/toml".to_string(),
        mode: "replace".to_string(),
        content: normalized_config,
    }));
    bundle
        .artifacts
        .sort_by(|left, right| left.path.cmp(&right.path));
    validate_bundle(&bundle)?;
    let mut bundle_input = serde_json::to_string_pretty(&bundle)?;
    bundle_input.push('\n');
    Ok(PreparedSetupLifecycle {
        bundle,
        bundle_input,
    })
}

pub(crate) fn same_lifecycle_plan(left: &LifecycleReport, right: &LifecycleReport) -> bool {
    left.action == right.action
        && left.bundle_id == right.bundle_id
        && left.repository == right.repository
        && left.artifacts == right.artifacts
}

#[derive(Debug)]
pub(crate) struct PlannedArtifact {
    pub(crate) path: String,
    pub(crate) target: PathBuf,
    pub(crate) mode: String,
    pub(crate) content: Option<String>,
    pub(crate) managed_content: Option<String>,
    pub(crate) sha256: String,
    pub(crate) status: String,
}

pub(crate) fn preview_bundle(
    repository: &Path,
    bundle: &RoutingBundleV1,
) -> Result<LifecycleReport> {
    let repository = canonicalize_existing_repository(repository)?;
    recover_pending_transactions(&repository)?;
    let planned = plan_artifacts(&repository, bundle, None)?;
    Ok(report_from_plan(
        "preview",
        &repository,
        Some(&bundle.bundle_id),
        &planned,
    ))
}

pub(crate) fn plan_artifacts(
    repository: &Path,
    bundle: &RoutingBundleV1,
    current_manifest: Option<&ManagedManifest>,
) -> Result<Vec<PlannedArtifact>> {
    let mut seen_targets = BTreeSet::new();
    let mut planned = Vec::new();
    for artifact in &bundle.artifacts {
        let target = resolve_repository_target(repository, &artifact.path)?;
        let key = target.display().to_string();
        if !seen_targets.insert(key) {
            bail!("duplicate resolved artifact target `{}`", artifact.path);
        }
        let managed_entry = current_manifest.and_then(|manifest| {
            manifest
                .artifacts
                .iter()
                .find(|managed| managed.path == artifact.path)
        });
        if artifact.path == CODEX_CONFIG_PATH {
            planned.push(plan_codex_config_artifact(
                repository,
                artifact,
                target,
                managed_entry,
            )?);
            continue;
        }
        let status = if target.exists() {
            let metadata = fs::symlink_metadata(&target)
                .with_context(|| format!("failed to inspect `{}`", target.display()))?;
            if metadata.file_type().is_symlink() {
                bail!("artifact target `{}` is a symlink", artifact.path);
            }
            let current = fs::read(&target)
                .with_context(|| format!("failed to read `{}`", target.display()))?;
            let current_sha = sha256(&current);
            if current_sha == artifact.sha256 {
                "unchanged"
            } else if let Some(managed) = managed_entry {
                if current_sha == managed.sha256 {
                    "update"
                } else {
                    "preserved-modified"
                }
            } else {
                "conflict"
            }
        } else {
            ensure_parent_is_safe(repository, &target)?;
            "create"
        };
        planned.push(PlannedArtifact {
            path: artifact.path.clone(),
            target,
            mode: artifact.mode.clone(),
            content: Some(artifact.content.clone()),
            managed_content: Some(artifact.content.clone()),
            sha256: artifact.sha256.clone(),
            status: status.to_string(),
        });
    }
    if let Some(manifest) = current_manifest {
        for artifact in &manifest.artifacts {
            if bundle
                .artifacts
                .iter()
                .any(|bundle_artifact| bundle_artifact.path == artifact.path)
            {
                continue;
            }
            let target = resolve_repository_target(repository, &artifact.path)?;
            let (status, content) = if artifact.path == CODEX_CONFIG_PATH {
                let status = preserved_or_removed_status(&target, artifact)?;
                let content = if status == "removed" {
                    remove_managed_codex_config_entries(&target, artifact)?
                } else {
                    artifact.content.clone()
                };
                (status, content)
            } else {
                let status = preserved_or_removed_status(&target, artifact)?;
                let content = if status == "removed" {
                    None
                } else {
                    artifact.content.clone()
                };
                (status, content)
            };
            planned.push(PlannedArtifact {
                path: artifact.path.clone(),
                target,
                mode: "delete".to_string(),
                content,
                managed_content: artifact.content.clone(),
                sha256: artifact.sha256.clone(),
                status,
            });
        }
    }
    reject_parent_child_targets(&planned)?;
    Ok(planned)
}

pub(crate) fn ensure_apply_is_safe(planned: &[PlannedArtifact]) -> Result<()> {
    for artifact in planned {
        if artifact.status == "conflict" || artifact.status == "preserved-modified" {
            bail!(
                "artifact target `{}` already exists with different content",
                artifact.path
            );
        }
    }
    Ok(())
}

pub(crate) fn ensure_update_is_safe(planned: &[PlannedArtifact]) -> Result<()> {
    for artifact in planned {
        if artifact.status == "conflict" {
            bail!(
                "artifact target `{}` already exists with unmanaged content",
                artifact.path
            );
        }
    }
    Ok(())
}

pub(crate) fn reject_parent_child_targets(planned: &[PlannedArtifact]) -> Result<()> {
    for (index, left) in planned.iter().enumerate() {
        let left_relative = Path::new(&left.path);
        for right in planned.iter().skip(index + 1) {
            let right_relative = Path::new(&right.path);
            if left_relative.starts_with(right_relative)
                || right_relative.starts_with(left_relative)
            {
                bail!(
                    "artifact targets `{}` and `{}` have a parent-child collision",
                    left.path,
                    right.path
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn plan_rollback_artifacts(
    repository: &Path,
    current_manifest: &ManagedManifest,
    previous: &ManagedSnapshot,
) -> Result<Vec<PlannedArtifact>> {
    let mut planned = Vec::new();
    for artifact in &previous.artifacts {
        let content = artifact.content.clone().ok_or_else(|| {
            product_error!(
                "rollback artifact `{}` has no stored content",
                artifact.path
            )
        })?;
        let target = resolve_repository_target(repository, &artifact.path)?;
        let current = current_manifest
            .artifacts
            .iter()
            .find(|managed| managed.path == artifact.path);
        if artifact.path == CODEX_CONFIG_PATH {
            planned.push(plan_codex_config_rollback_artifact(
                repository, artifact, content, target, current,
            )?);
            continue;
        }
        let status = if target.exists() {
            let current_content = fs::read(&target)
                .with_context(|| format!("failed to read `{}`", target.display()))?;
            let current_sha = sha256(&current_content);
            if current_sha == artifact.sha256 {
                "unchanged"
            } else if let Some(managed) = current {
                if current_sha == managed.sha256 {
                    "rollback"
                } else {
                    "preserved-modified"
                }
            } else {
                "rollback"
            }
        } else {
            ensure_parent_is_safe(repository, &target)?;
            "create"
        };
        planned.push(PlannedArtifact {
            path: artifact.path.clone(),
            target,
            mode: "replace".to_string(),
            content: Some(content),
            managed_content: artifact.content.clone(),
            sha256: artifact.sha256.clone(),
            status: status.to_string(),
        });
    }
    for artifact in &current_manifest.artifacts {
        if previous
            .artifacts
            .iter()
            .any(|previous_artifact| previous_artifact.path == artifact.path)
        {
            continue;
        }
        let target = resolve_repository_target(repository, &artifact.path)?;
        let (status, content) = if artifact.path == CODEX_CONFIG_PATH {
            let status = preserved_or_removed_status(&target, artifact)?;
            let content = if status == "removed" {
                remove_managed_codex_config_entries(&target, artifact)?
            } else {
                artifact.content.clone()
            };
            (status, content)
        } else {
            let status = preserved_or_removed_status(&target, artifact)?;
            let content = if status == "removed" {
                None
            } else {
                artifact.content.clone()
            };
            (status, content)
        };
        planned.push(PlannedArtifact {
            path: artifact.path.clone(),
            target,
            mode: "delete".to_string(),
            content,
            managed_content: artifact.content.clone(),
            sha256: artifact.sha256.clone(),
            status,
        });
    }
    reject_parent_child_targets(&planned)?;
    Ok(planned)
}

pub(crate) fn plan_codex_config_artifact(
    repository: &Path,
    artifact: &BundleArtifact,
    target: PathBuf,
    managed_entry: Option<&ManagedArtifact>,
) -> Result<PlannedArtifact> {
    let (status, content) = if target.exists() {
        ensure_artifact_target_is_regular(&target, &artifact.path)?;
        let current = fs::read_to_string(&target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        if let Some(managed) = managed_entry {
            if codex_config_contains_managed_entries(&current, managed)? {
                if codex_config_has_unmanaged_conflict(&current, &artifact.content, Some(managed))?
                {
                    ("conflict".to_string(), Some(current))
                } else if managed.content.as_deref() == Some(artifact.content.as_str())
                    && codex_config_contains_desired_entries(&current, &artifact.content)?
                {
                    ("unchanged".to_string(), Some(current))
                } else {
                    (
                        "update".to_string(),
                        merge_codex_config_entries(
                            Some(&current),
                            Some(managed),
                            &artifact.content,
                        )?,
                    )
                }
            } else {
                ("preserved-modified".to_string(), Some(current))
            }
        } else if codex_config_has_unmanaged_conflict(&current, &artifact.content, None)? {
            ("conflict".to_string(), Some(current))
        } else if codex_config_contains_desired_entries(&current, &artifact.content)? {
            ("unchanged".to_string(), Some(current))
        } else {
            (
                "update".to_string(),
                merge_codex_config_entries(Some(&current), None, &artifact.content)?,
            )
        }
    } else {
        ensure_parent_is_safe(repository, &target)?;
        ("create".to_string(), Some(artifact.content.clone()))
    };
    Ok(PlannedArtifact {
        path: artifact.path.clone(),
        target,
        mode: artifact.mode.clone(),
        content,
        managed_content: Some(artifact.content.clone()),
        sha256: artifact.sha256.clone(),
        status,
    })
}

pub(crate) fn plan_codex_config_rollback_artifact(
    repository: &Path,
    artifact: &ManagedArtifact,
    desired_content: String,
    target: PathBuf,
    current: Option<&ManagedArtifact>,
) -> Result<PlannedArtifact> {
    let (status, content) = if target.exists() {
        ensure_artifact_target_is_regular(&target, &artifact.path)?;
        let current_text = fs::read_to_string(&target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        if let Some(managed) = current {
            if codex_config_contains_managed_entries(&current_text, managed)? {
                if codex_config_has_unmanaged_conflict(
                    &current_text,
                    &desired_content,
                    Some(managed),
                )? {
                    ("conflict".to_string(), Some(current_text))
                } else if managed.content.as_deref() == Some(desired_content.as_str())
                    && codex_config_contains_desired_entries(&current_text, &desired_content)?
                {
                    ("unchanged".to_string(), Some(current_text))
                } else {
                    (
                        "rollback".to_string(),
                        merge_codex_config_entries(
                            Some(&current_text),
                            Some(managed),
                            &desired_content,
                        )?,
                    )
                }
            } else {
                ("preserved-modified".to_string(), Some(current_text))
            }
        } else if codex_config_has_unmanaged_conflict(&current_text, &desired_content, None)? {
            ("conflict".to_string(), Some(current_text))
        } else {
            (
                "rollback".to_string(),
                merge_codex_config_entries(Some(&current_text), None, &desired_content)?,
            )
        }
    } else {
        ensure_parent_is_safe(repository, &target)?;
        ("create".to_string(), Some(desired_content.clone()))
    };
    Ok(PlannedArtifact {
        path: artifact.path.clone(),
        target,
        mode: "replace".to_string(),
        content,
        managed_content: Some(desired_content),
        sha256: artifact.sha256.clone(),
        status,
    })
}

pub(crate) fn ensure_artifact_target_is_regular(target: &Path, artifact_path: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(target)
        .with_context(|| format!("failed to inspect `{}`", target.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("artifact target `{artifact_path}` is a symlink");
    }
    if !metadata.is_file() {
        bail!("artifact target `{artifact_path}` is not a file");
    }
    Ok(())
}

pub(crate) fn preserved_or_removed_status(
    target: &Path,
    artifact: &ManagedArtifact,
) -> Result<String> {
    if !target.exists() {
        return Ok("missing".to_string());
    }
    let metadata = fs::symlink_metadata(target)
        .with_context(|| format!("failed to inspect `{}`", target.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("artifact target `{}` is a symlink", artifact.path);
    }
    if artifact.path == CODEX_CONFIG_PATH {
        let current = fs::read_to_string(target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        return if codex_config_contains_managed_entries(&current, artifact)? {
            Ok("removed".to_string())
        } else {
            Ok("preserved-modified".to_string())
        };
    }
    let current =
        fs::read(target).with_context(|| format!("failed to read `{}`", target.display()))?;
    if sha256(&current) == artifact.sha256 {
        Ok("removed".to_string())
    } else {
        Ok("preserved-modified".to_string())
    }
}

pub(crate) fn status_for_managed_artifact(
    target: &Path,
    artifact: &ManagedArtifact,
) -> Result<&'static str> {
    if !target.exists() {
        return Ok("missing");
    }
    let metadata = fs::symlink_metadata(target)
        .with_context(|| format!("failed to inspect `{}`", target.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("artifact target `{}` is a symlink", artifact.path);
    }
    if artifact.path == CODEX_CONFIG_PATH {
        let current = fs::read_to_string(target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        return if codex_config_contains_managed_entries(&current, artifact)? {
            Ok("managed")
        } else {
            Ok("modified")
        };
    }
    let content =
        fs::read(target).with_context(|| format!("failed to read `{}`", target.display()))?;
    if sha256(&content) == artifact.sha256 {
        Ok("managed")
    } else {
        Ok("modified")
    }
}

pub(crate) fn uninstall_managed_artifact(
    target: &Path,
    artifact: &ManagedArtifact,
) -> Result<&'static str> {
    if !target.exists() {
        return Ok("missing");
    }
    let metadata = fs::symlink_metadata(target)
        .with_context(|| format!("failed to inspect `{}`", target.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("artifact target `{}` is a symlink", artifact.path);
    }
    if artifact.path == CODEX_CONFIG_PATH {
        let current = fs::read_to_string(target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        if !codex_config_contains_managed_entries(&current, artifact)? {
            return Ok("preserved-modified");
        }
        match remove_managed_codex_config_entries(target, artifact)? {
            Some(content) => fs::write(target, content.as_bytes())
                .with_context(|| format!("failed to write `{}`", target.display()))?,
            None => fs::remove_file(target)
                .with_context(|| format!("failed to remove `{}`", target.display()))?,
        }
        return Ok("removed");
    }
    let content =
        fs::read(target).with_context(|| format!("failed to read `{}`", target.display()))?;
    if sha256(&content) != artifact.sha256 {
        Ok("preserved-modified")
    } else {
        fs::remove_file(target)
            .with_context(|| format!("failed to remove `{}`", target.display()))?;
        Ok("removed")
    }
}

pub(crate) fn codex_config_contains_managed_entries(
    current_content: &str,
    managed: &ManagedArtifact,
) -> Result<bool> {
    let managed_content = managed.content.as_deref().ok_or_else(|| {
        product_error!("managed artifact `{}` has no stored content", managed.path)
    })?;
    Ok(
        !codex_config_has_unmanaged_conflict(current_content, managed_content, None)?
            && codex_config_contains_desired_entries(current_content, managed_content)?,
    )
}

pub(crate) fn codex_config_contains_desired_entries(
    current_content: &str,
    desired_content: &str,
) -> Result<bool> {
    let current = codex_agent_entries(current_content)?;
    let desired = codex_agent_entries(desired_content)?;
    Ok(desired
        .iter()
        .all(|(name, desired_entry)| current.get(name) == Some(desired_entry)))
}

pub(crate) fn codex_config_has_unmanaged_conflict(
    current_content: &str,
    desired_content: &str,
    previously_managed: Option<&ManagedArtifact>,
) -> Result<bool> {
    let current = codex_agent_entries(current_content)?;
    let desired = codex_agent_entries(desired_content)?;
    let old_keys = previously_managed
        .and_then(|managed| managed.content.as_deref())
        .map(codex_agent_entry_names)
        .transpose()?
        .unwrap_or_default();
    Ok(desired.iter().any(|(name, desired_entry)| {
        !old_keys.contains(name)
            && current
                .get(name)
                .is_some_and(|entry| entry != desired_entry)
    }))
}

pub(crate) fn merge_codex_config_entries(
    current_content: Option<&str>,
    previously_managed: Option<&ManagedArtifact>,
    desired_content: &str,
) -> Result<Option<String>> {
    let mut root = match current_content {
        Some(content) => parse_toml_root(content)?,
        None => toml::value::Table::new(),
    };
    if let Some(managed) = previously_managed {
        let managed_content = managed.content.as_deref().ok_or_else(|| {
            product_error!("managed artifact `{}` has no stored content", managed.path)
        })?;
        remove_codex_agent_entries(&mut root, &codex_agent_entry_names(managed_content)?)?;
    }
    upsert_codex_agent_entries(&mut root, codex_agent_entries(desired_content)?)?;
    render_toml_root(root)
}

pub(crate) fn remove_managed_codex_config_entries(
    target: &Path,
    managed: &ManagedArtifact,
) -> Result<Option<String>> {
    let current = fs::read_to_string(target)
        .with_context(|| format!("failed to read `{}`", target.display()))?;
    let managed_content = managed.content.as_deref().ok_or_else(|| {
        product_error!("managed artifact `{}` has no stored content", managed.path)
    })?;
    let mut root = parse_toml_root(&current)?;
    remove_codex_agent_entries(&mut root, &codex_agent_entry_names(managed_content)?)?;
    render_toml_root(root)
}

pub(crate) fn parse_toml_root(content: &str) -> Result<toml::value::Table> {
    match toml::from_str::<toml::Value>(content)? {
        toml::Value::Table(table) => Ok(table),
        _ => bail!("Codex config must be a TOML table"),
    }
}

pub(crate) fn codex_agent_entry_names(content: &str) -> Result<BTreeSet<String>> {
    Ok(codex_agent_entries(content)?.into_keys().collect())
}

pub(crate) fn codex_agent_entries(content: &str) -> Result<BTreeMap<String, toml::Value>> {
    let root = parse_toml_root(content)?;
    let Some(agents) = root.get("agents") else {
        return Ok(BTreeMap::new());
    };
    let agents = agents
        .as_table()
        .ok_or_else(|| product_error!("Codex config `agents` must be a table"))?;
    Ok(agents
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect())
}

pub(crate) fn remove_codex_agent_entries(
    root: &mut toml::value::Table,
    names: &BTreeSet<String>,
) -> Result<()> {
    let Some(agents_value) = root.get_mut("agents") else {
        return Ok(());
    };
    let agents = agents_value
        .as_table_mut()
        .ok_or_else(|| product_error!("Codex config `agents` must be a table"))?;
    for name in names {
        agents.remove(name);
    }
    if agents.is_empty() {
        root.remove("agents");
    }
    Ok(())
}

pub(crate) fn upsert_codex_agent_entries(
    root: &mut toml::value::Table,
    entries: BTreeMap<String, toml::Value>,
) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    if !root.contains_key("agents") {
        root.insert(
            "agents".to_string(),
            toml::Value::Table(toml::value::Table::new()),
        );
    }
    let agents = root
        .get_mut("agents")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| product_error!("Codex config `agents` must be a table"))?;
    for (name, value) in entries {
        agents.insert(name, value);
    }
    Ok(())
}

pub(crate) fn render_toml_root(root: toml::value::Table) -> Result<Option<String>> {
    if root.is_empty() {
        return Ok(None);
    }
    let mut content = toml::to_string_pretty(&toml::Value::Table(root))?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    Ok(Some(content))
}

pub(crate) fn commit_transaction(
    repository: &Path,
    planned: &[PlannedArtifact],
    manifest: &ManagedManifest,
) -> Result<()> {
    let txn_root = repository.join(".model-routing").join(format!(
        "txn-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let stage_root = txn_root.join("stage");
    let backup_root = txn_root.join("backup");
    fs::create_dir_all(&stage_root)
        .with_context(|| format!("failed to create `{}`", stage_root.display()))?;
    fs::create_dir_all(&backup_root)
        .with_context(|| format!("failed to create `{}`", backup_root.display()))?;

    let mut writes = Vec::new();
    for (index, artifact) in planned.iter().enumerate() {
        if artifact.status == "unchanged"
            || artifact.status == "preserved-modified"
            || artifact.status == "missing"
        {
            continue;
        }
        let staged = match &artifact.content {
            Some(content) => {
                let staged = stage_root.join(format!("artifact-{index}"));
                fs::write(&staged, content.as_bytes())
                    .with_context(|| format!("failed to stage `{}`", artifact.path))?;
                Some(staged)
            }
            None => None,
        };
        writes.push(TransactionalWrite {
            label: artifact.path.clone(),
            target: artifact.target.clone(),
            staged,
            backup: backup_root.join(format!("artifact-{index}")),
            committed: false,
            backup_created: false,
            had_original: artifact.target.exists(),
        });
    }

    let manifest_path = repository.join(MANIFEST_PATH);
    let manifest_stage = stage_root.join("manifest.json");
    fs::write(&manifest_stage, serde_json::to_vec_pretty(manifest)?)
        .with_context(|| format!("failed to stage `{MANIFEST_PATH}`"))?;
    writes.push(TransactionalWrite {
        label: MANIFEST_PATH.to_string(),
        target: manifest_path.clone(),
        staged: Some(manifest_stage),
        backup: backup_root.join("manifest.json"),
        committed: false,
        backup_created: false,
        had_original: manifest_path.exists(),
    });

    write_transaction_journal(repository, &txn_root, &writes)?;
    let result = commit_writes(&mut writes);
    if let Err(error) = result {
        if let Err(rollback_error) = rollback_writes(&writes) {
            return Err(error).with_context(|| {
                format!(
                    "transaction rollback incomplete; retained `{}` for recovery: {rollback_error:#}",
                    txn_root.display()
                )
            });
        }
        fs::remove_dir_all(&txn_root)
            .with_context(|| format!("failed to remove `{}`", txn_root.display()))?;
        return Err(error);
    }
    fs::remove_dir_all(&txn_root)
        .with_context(|| format!("failed to remove `{}`", txn_root.display()))?;
    Ok(())
}

#[derive(Debug)]
pub(crate) struct TransactionalWrite {
    pub(crate) label: String,
    pub(crate) target: PathBuf,
    pub(crate) staged: Option<PathBuf>,
    pub(crate) backup: PathBuf,
    pub(crate) committed: bool,
    pub(crate) backup_created: bool,
    pub(crate) had_original: bool,
}

pub(crate) fn commit_writes(writes: &mut [TransactionalWrite]) -> Result<()> {
    let txn_root = writes
        .first()
        .and_then(|write| write.backup.parent())
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| product_error!("transaction has no writes"))?;
    let repository = txn_root
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| product_error!("transaction root is outside repository metadata"))?
        .to_path_buf();
    for index in 0..writes.len() {
        if let Some(parent) = writes[index].target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create `{}`", parent.display()))?;
        }
        if writes[index].target.exists() {
            fs::rename(&writes[index].target, &writes[index].backup)
                .with_context(|| format!("failed to backup `{}`", writes[index].label))?;
            writes[index].backup_created = true;
            write_transaction_journal(&repository, &txn_root, writes)?;
        }
        if let Some(staged) = &writes[index].staged {
            maybe_return_staged_rename_error()?;
            fs::rename(staged, &writes[index].target)
                .with_context(|| format!("failed to commit `{}`", writes[index].label))?;
        }
        writes[index].committed = true;
        write_transaction_journal(&repository, &txn_root, writes)?;
        maybe_fail_after_transaction_write()?;
    }
    Ok(())
}

pub(crate) fn rollback_writes(writes: &[TransactionalWrite]) -> Result<()> {
    for write in writes.iter().rev() {
        if !write.committed && !write.backup_created {
            continue;
        }
        maybe_fail_during_restore()?;
        if write.target.exists() {
            fs::remove_file(&write.target)
                .with_context(|| format!("failed to remove `{}`", write.target.display()))?;
        }
        if write.had_original {
            if let Some(parent) = write.target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create `{}`", parent.display()))?;
            }
            fs::rename(&write.backup, &write.target).with_context(|| {
                format!(
                    "failed to restore `{}` from `{}`",
                    write.target.display(),
                    write.backup.display()
                )
            })?;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TransactionJournal {
    pub(crate) schema_version: u32,
    pub(crate) writes: Vec<TransactionJournalWrite>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TransactionJournalWrite {
    pub(crate) label: String,
    pub(crate) target: String,
    pub(crate) staged: Option<String>,
    pub(crate) backup: String,
    pub(crate) committed: bool,
    #[serde(default)]
    pub(crate) backup_created: bool,
    pub(crate) had_original: bool,
}

pub(crate) fn write_transaction_journal(
    repository: &Path,
    txn_root: &Path,
    writes: &[TransactionalWrite],
) -> Result<()> {
    let journal = TransactionJournal {
        schema_version: 1,
        writes: writes
            .iter()
            .map(|write| {
                Ok(TransactionJournalWrite {
                    label: write.label.clone(),
                    target: repository_relative(repository, &write.target)?,
                    staged: write
                        .staged
                        .as_ref()
                        .map(|staged| repository_relative(repository, staged))
                        .transpose()?,
                    backup: repository_relative(repository, &write.backup)?,
                    committed: write.committed,
                    backup_created: write.backup_created,
                    had_original: write.had_original,
                })
            })
            .collect::<Result<Vec<_>>>()?,
    };
    let journal_path = txn_root.join(TRANSACTION_JOURNAL);
    let temp_path = txn_root.join(format!("{TRANSACTION_JOURNAL}.tmp"));
    fs::write(&temp_path, serde_json::to_vec_pretty(&journal)?).with_context(|| {
        format!(
            "failed to write transaction journal temp `{}`",
            temp_path.display()
        )
    })?;
    sync_file(&temp_path)?;
    maybe_return_journal_error()?;
    maybe_fail_during_journal_replace();
    fs::rename(&temp_path, &journal_path).with_context(|| {
        format!(
            "failed to replace transaction journal `{}`",
            journal_path.display()
        )
    })?;
    sync_directory(txn_root)?;
    Ok(())
}

pub(crate) fn recover_pending_transactions(repository: &Path) -> Result<()> {
    let metadata_dir = repository.join(".model-routing");
    if !metadata_dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(&metadata_dir)
        .with_context(|| format!("failed to read `{}`", metadata_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with("txn-") {
            continue;
        }
        recover_transaction(repository, &entry.path())?;
    }
    Ok(())
}

pub(crate) fn recover_transaction(repository: &Path, txn_root: &Path) -> Result<()> {
    let journal_path = txn_root.join(TRANSACTION_JOURNAL);
    if journal_path.exists() {
        let input = fs::read(&journal_path)
            .with_context(|| format!("failed to read `{}`", journal_path.display()))?;
        let journal: TransactionJournal = serde_json::from_slice(&input)
            .with_context(|| format!("failed to parse `{}`", journal_path.display()))?;
        for write in journal.writes.iter().rev() {
            recover_transaction_write(repository, write).with_context(|| {
                format!("failed to recover transaction write `{}`", write.label)
            })?;
        }
    }
    fs::remove_dir_all(txn_root)
        .with_context(|| format!("failed to remove `{}`", txn_root.display()))?;
    Ok(())
}

pub(crate) fn recover_transaction_write(
    repository: &Path,
    write: &TransactionJournalWrite,
) -> Result<()> {
    maybe_fail_during_restore()?;
    let target = repository.join(&write.target);
    let backup = repository.join(&write.backup);
    if backup.exists() {
        if target.exists() {
            fs::remove_file(&target)
                .with_context(|| format!("failed to remove `{}`", target.display()))?;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create `{}`", parent.display()))?;
        }
        fs::rename(&backup, &target).with_context(|| {
            format!(
                "failed to restore `{}` from `{}`",
                target.display(),
                backup.display()
            )
        })?;
        return Ok(());
    }
    if !write.had_original
        && write
            .staged
            .as_ref()
            .is_some_and(|staged| !repository.join(staged).exists())
        && target.exists()
    {
        fs::remove_file(&target)
            .with_context(|| format!("failed to remove partial `{}`", target.display()))?;
    }
    Ok(())
}

pub(crate) fn repository_relative(repository: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(repository)
        .with_context(|| format!("`{}` is outside repository", path.display()))?
        .to_str()
        .ok_or_else(|| product_error!("path `{}` is not UTF-8", path.display()))?
        .to_string())
}

pub(crate) fn sync_file(path: &Path) -> Result<()> {
    fs::File::open(path)
        .with_context(|| format!("failed to open `{}` for sync", path.display()))?
        .sync_all()
        .with_context(|| format!("failed to sync `{}`", path.display()))?;
    Ok(())
}

pub(crate) fn sync_directory(path: &Path) -> Result<()> {
    fs::File::open(path)
        .with_context(|| format!("failed to open directory `{}` for sync", path.display()))?
        .sync_all()
        .with_context(|| format!("failed to sync directory `{}`", path.display()))?;
    Ok(())
}

pub(crate) fn maybe_fail_after_transaction_write() -> Result<()> {
    TRANSACTION_FAIL_AFTER_WRITES.with(|fail_after| {
        let remaining = fail_after.get();
        if remaining == 0 {
            return;
        }
        fail_after.set(remaining - 1);
        if remaining == 1 {
            panic!("injected transaction interruption after committed write");
        }
    });
    Ok(())
}

pub(crate) fn maybe_fail_during_journal_replace() {
    TRANSACTION_FAIL_JOURNAL_REPLACE_AFTER.with(|fail_after| {
        let remaining = fail_after.get();
        if remaining == 0 {
            return;
        }
        fail_after.set(remaining - 1);
        if remaining == 1 {
            panic!("injected transaction interruption during journal replacement");
        }
    });
}

pub(crate) fn maybe_return_journal_error() -> Result<()> {
    TRANSACTION_RETURN_JOURNAL_ERROR_AFTER.with(|fail_after| {
        let remaining = fail_after.get();
        if remaining == 0 {
            return Ok(());
        }
        fail_after.set(remaining - 1);
        if remaining == 1 {
            bail!("injected transaction journal update error");
        }
        Ok(())
    })
}

pub(crate) fn maybe_return_staged_rename_error() -> Result<()> {
    TRANSACTION_RETURN_STAGED_RENAME_ERROR_AFTER.with(|fail_after| {
        let remaining = fail_after.get();
        if remaining == 0 {
            return Ok(());
        }
        fail_after.set(remaining - 1);
        if remaining == 1 {
            bail!("injected staged rename error after backup");
        }
        Ok(())
    })
}

pub(crate) fn maybe_fail_during_restore() -> Result<()> {
    TRANSACTION_FAIL_RESTORE.with(|fail| {
        if fail.replace(false) {
            bail!("injected transaction restore failure");
        }
        Ok(())
    })
}

pub(crate) fn canonicalize_existing_repository(repository: &Path) -> Result<PathBuf> {
    let canonical = repository
        .canonicalize()
        .with_context(|| format!("repository `{}` does not exist", repository.display()))?;
    if !canonical.is_dir() {
        bail!("repository `{}` is not a directory", canonical.display());
    }
    Ok(canonical)
}

pub(crate) fn resolve_repository_target(repository: &Path, artifact_path: &str) -> Result<PathBuf> {
    if artifact_path.trim().is_empty() {
        bail!("artifact path must not be blank");
    }
    let path = Path::new(artifact_path);
    if path.is_absolute() {
        bail!("artifact path `{artifact_path}` must be repository-relative");
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir => bail!("artifact path `{artifact_path}` must not traverse"),
            _ => bail!("artifact path `{artifact_path}` is unsupported"),
        }
    }
    let normalized_text = normalized
        .to_str()
        .ok_or_else(|| product_error!("artifact path `{artifact_path}` is not UTF-8"))?;
    if normalized_text.starts_with(".model-routing/") {
        bail!("artifact path `{artifact_path}` targets a reserved path");
    }
    if normalized_text == SETUP_CONFIG_PATH {
        return Ok(repository.join(normalized));
    }
    if !allowed_repository_target(normalized_text) {
        bail!("artifact path `{artifact_path}` is not an allowed host artifact path");
    }
    Ok(repository.join(normalized))
}

pub(crate) fn allowed_repository_target(path: &str) -> bool {
    if path == ".codex/config.toml" {
        return true;
    }
    [
        ".codex/agents/",
        ".claude/agents/",
        ".cursor/agents/",
        ".opencode/agents/",
        ".pi/workflows/",
        ".planr/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

pub(crate) fn ensure_parent_is_safe(repository: &Path, target: &Path) -> Result<()> {
    let mut current = repository.to_path_buf();
    let relative = target
        .strip_prefix(repository)
        .map_err(|_| product_error!("target escaped repository"))?;
    if let Some(parent) = relative.parent() {
        for component in parent.components() {
            let Component::Normal(part) = component else {
                bail!("artifact parent contains unsupported component");
            };
            current.push(part);
            if current.exists() {
                let metadata = fs::symlink_metadata(&current)
                    .with_context(|| format!("failed to inspect `{}`", current.display()))?;
                if metadata.file_type().is_symlink() {
                    bail!("artifact parent `{}` is a symlink", current.display());
                }
                if !metadata.is_dir() {
                    bail!("artifact parent `{}` is not a directory", current.display());
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn manifest_from_bundle(
    bundle: &RoutingBundleV1,
    bundle_sha256: String,
    previous: Option<ManagedSnapshot>,
) -> ManagedManifest {
    ManagedManifest {
        schema_version: 1,
        bundle_id: bundle.bundle_id.clone(),
        bundle_sha256,
        artifacts: bundle
            .artifacts
            .iter()
            .map(|artifact| ManagedArtifact {
                path: artifact.path.clone(),
                sha256: artifact.sha256.clone(),
                content: Some(artifact.content.clone()),
            })
            .collect(),
        previous,
    }
}

pub(crate) fn manifest_from_plan(
    bundle_id: &str,
    bundle_sha256: String,
    planned: &[PlannedArtifact],
    previous: Option<ManagedSnapshot>,
) -> ManagedManifest {
    ManagedManifest {
        schema_version: 1,
        bundle_id: bundle_id.to_string(),
        bundle_sha256,
        artifacts: planned
            .iter()
            .filter(|artifact| artifact.status != "removed")
            .map(|artifact| ManagedArtifact {
                path: artifact.path.clone(),
                sha256: artifact.sha256.clone(),
                content: artifact.managed_content.clone(),
            })
            .collect(),
        previous,
    }
}

pub(crate) fn snapshot_from_manifest(manifest: &ManagedManifest) -> ManagedSnapshot {
    ManagedSnapshot {
        bundle_id: manifest.bundle_id.clone(),
        bundle_sha256: manifest.bundle_sha256.clone(),
        artifacts: manifest.artifacts.clone(),
    }
}

pub(crate) fn write_manifest_file(repository: &Path, manifest: &ManagedManifest) -> Result<()> {
    let manifest_path = repository.join(MANIFEST_PATH);
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create `{}`", parent.display()))?;
    }
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)
        .with_context(|| format!("failed to write `{}`", manifest_path.display()))?;
    Ok(())
}

pub(crate) fn remove_manifest(repository: &Path) -> Result<()> {
    let manifest_path = repository.join(MANIFEST_PATH);
    if manifest_path.exists() {
        fs::remove_file(&manifest_path)
            .with_context(|| format!("failed to remove `{}`", manifest_path.display()))?;
    }
    Ok(())
}

pub(crate) fn read_manifest(repository: &Path) -> Result<Option<ManagedManifest>> {
    let manifest_path = repository.join(MANIFEST_PATH);
    if !manifest_path.exists() {
        return Ok(None);
    }
    let input = fs::read(&manifest_path)
        .with_context(|| format!("failed to read `{}`", manifest_path.display()))?;
    Ok(Some(serde_json::from_slice(&input).with_context(|| {
        format!("failed to parse `{}`", manifest_path.display())
    })?))
}

pub(crate) fn report_from_plan(
    action: &str,
    repository: &Path,
    bundle_id: Option<&str>,
    planned: &[PlannedArtifact],
) -> LifecycleReport {
    LifecycleReport {
        action: action.to_string(),
        bundle_id: bundle_id.map(ToOwned::to_owned),
        repository: repository.display().to_string(),
        artifacts: planned
            .iter()
            .map(|artifact| LifecycleArtifactReport {
                path: artifact.path.clone(),
                mode: artifact.mode.clone(),
                status: artifact.status.clone(),
                sha256: artifact.sha256.clone(),
                repair: repair_for_status(&artifact.status),
            })
            .collect(),
    }
}

pub(crate) fn repair_for_status(status: &str) -> Option<String> {
    match status {
        "modified" | "preserved-modified" => Some(
            "user-modified file preserved; run update or rollback after reconciling local edits"
                .to_string(),
        ),
        "missing" => Some(
            "managed file is missing; run update to recreate or uninstall to drop ownership"
                .to_string(),
        ),
        _ => None,
    }
}
