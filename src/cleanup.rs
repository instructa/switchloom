//! Cleanup of persisted Switchloom installations. No role installation or generation.
//!
//! The manifest and transaction journal are existing on-disk contracts. Keep their
//! reader here so removal preserves user edits and can recover interrupted writes.
use crate::digest::sha256;
use crate::error::{Result, ResultContext};
use crate::{bail, product_error};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

const CODEX_CONFIG_PATH: &str = ".codex/config.toml";
const SETUP_CONFIG_PATH: &str = ".switchloom/config.toml";
const MANIFEST_PATH: &str = ".model-routing/manifest.json";
const TRANSACTION_JOURNAL: &str = "journal.json";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ManagedManifest {
    schema_version: u32,
    bundle_id: String,
    bundle_sha256: String,
    artifacts: Vec<ManagedArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    previous: Option<ManagedSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ManagedArtifact {
    path: String,
    sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ownership_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ManagedSnapshot {
    bundle_id: String,
    bundle_sha256: String,
    artifacts: Vec<ManagedArtifact>,
}

#[derive(Default)]
struct CodexConfigEntries {
    agents: BTreeMap<String, toml::Value>,
    multi_agent_v2: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransactionJournal {
    schema_version: u32,
    writes: Vec<TransactionJournalWrite>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransactionJournalWrite {
    label: String,
    target: String,
    staged: Option<String>,
    backup: String,
    committed: bool,
    #[serde(default)]
    backup_created: bool,
    had_original: bool,
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
            ownership_content: artifact.ownership_content.clone(),
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

fn status_for_managed_artifact(target: &Path, artifact: &ManagedArtifact) -> Result<&'static str> {
    if !regular_file_exists(target)? {
        return Ok("missing");
    }
    if artifact.path == CODEX_CONFIG_PATH {
        let current = fs::read_to_string(target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        return if codex_config_contains_owned_entries(&current, artifact)? {
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

fn uninstall_managed_artifact(target: &Path, artifact: &ManagedArtifact) -> Result<&'static str> {
    if !regular_file_exists(target)? {
        return Ok("removed");
    }
    if artifact.path == CODEX_CONFIG_PATH {
        let current = fs::read_to_string(target)
            .with_context(|| format!("failed to read `{}`", target.display()))?;
        if !codex_config_contains_owned_entries(&current, artifact)? {
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

fn codex_config_contains_owned_entries(
    current_content: &str,
    managed: &ManagedArtifact,
) -> Result<bool> {
    let managed_content = managed_artifact_ownership_content(managed)?;
    codex_config_contains_desired_entries(current_content, managed_content)
}

fn managed_artifact_ownership_content(managed: &ManagedArtifact) -> Result<&str> {
    managed
        .ownership_content
        .as_deref()
        .or(managed.content.as_deref())
        .ok_or_else(|| product_error!("managed artifact `{}` has no stored content", managed.path))
}

fn codex_config_contains_desired_entries(
    current_content: &str,
    desired_content: &str,
) -> Result<bool> {
    let current = codex_config_entries(current_content)?;
    let desired = codex_config_entries(desired_content)?;
    Ok(desired
        .agents
        .iter()
        .all(|(name, desired_entry)| current.agents.get(name) == Some(desired_entry))
        && desired
            .multi_agent_v2
            .iter()
            .all(|(key, desired_entry)| current.multi_agent_v2.get(key) == Some(desired_entry)))
}

fn remove_managed_codex_config_entries(
    target: &Path,
    managed: &ManagedArtifact,
) -> Result<Option<String>> {
    let current = fs::read_to_string(target)
        .with_context(|| format!("failed to read `{}`", target.display()))?;
    let managed_content = managed_artifact_ownership_content(managed)?;
    let mut root = parse_toml_root(&current)?;
    let entries = codex_config_entries(managed_content)?;
    remove_codex_agent_entries(&mut root, &entries.agents)?;
    remove_codex_multi_agent_v2_entries(&mut root, &entries.multi_agent_v2)?;
    render_toml_root(root)
}

fn parse_toml_root(content: &str) -> Result<toml::value::Table> {
    match toml::from_str::<toml::Value>(content)? {
        toml::Value::Table(table) => Ok(table),
        _ => bail!("Codex config must be a TOML table"),
    }
}

fn codex_config_entries(content: &str) -> Result<CodexConfigEntries> {
    let root = parse_toml_root(content)?;
    let agents = match root.get("agents") {
        Some(agents) => agents
            .as_table()
            .ok_or_else(|| product_error!("Codex config `agents` must be a table"))?
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
        None => BTreeMap::new(),
    };
    let multi_agent_v2 = match root
        .get("features")
        .and_then(toml::Value::as_table)
        .and_then(|features| features.get("multi_agent_v2"))
    {
        Some(value) => value
            .as_table()
            .ok_or_else(|| {
                product_error!("Codex config `features.multi_agent_v2` must be a table")
            })?
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
        None => BTreeMap::new(),
    };
    Ok(CodexConfigEntries {
        agents,
        multi_agent_v2,
    })
}

fn remove_codex_agent_entries(
    root: &mut toml::value::Table,
    names: &BTreeMap<String, toml::Value>,
) -> Result<()> {
    let Some(agents_value) = root.get_mut("agents") else {
        return Ok(());
    };
    let agents = agents_value
        .as_table_mut()
        .ok_or_else(|| product_error!("Codex config `agents` must be a table"))?;
    for name in names.keys() {
        agents.remove(name);
    }
    if agents.is_empty() {
        root.remove("agents");
    }
    Ok(())
}

fn remove_codex_multi_agent_v2_entries(
    root: &mut toml::value::Table,
    keys: &BTreeMap<String, toml::Value>,
) -> Result<()> {
    let Some(features_value) = root.get_mut("features") else {
        return Ok(());
    };
    let features = features_value
        .as_table_mut()
        .ok_or_else(|| product_error!("Codex config `features` must be a table"))?;
    let Some(multi_agent_value) = features.get_mut("multi_agent_v2") else {
        return Ok(());
    };
    let multi_agent = multi_agent_value
        .as_table_mut()
        .ok_or_else(|| product_error!("Codex config `features.multi_agent_v2` must be a table"))?;
    for key in keys.keys() {
        multi_agent.remove(key);
    }
    if multi_agent.is_empty() {
        features.remove("multi_agent_v2");
    }
    if features.is_empty() {
        root.remove("features");
    }
    Ok(())
}

fn render_toml_root(root: toml::value::Table) -> Result<Option<String>> {
    if root.is_empty() {
        return Ok(None);
    }
    let mut content = toml::to_string_pretty(&toml::Value::Table(root))?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    Ok(Some(content))
}

fn recover_pending_transactions(repository: &Path) -> Result<()> {
    ensure_parent_is_safe(repository, &repository.join(MANIFEST_PATH))?;
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

fn recover_transaction(repository: &Path, txn_root: &Path) -> Result<()> {
    let journal_path = txn_root.join(TRANSACTION_JOURNAL);
    if regular_file_exists(&journal_path)? {
        let input = fs::read(&journal_path)
            .with_context(|| format!("failed to read `{}`", journal_path.display()))?;
        let journal: TransactionJournal = serde_json::from_slice(&input)
            .with_context(|| format!("failed to parse `{}`", journal_path.display()))?;
        for write in journal.writes.iter().rev() {
            recover_transaction_write(repository, txn_root, write).with_context(|| {
                format!("failed to recover transaction write `{}`", write.label)
            })?;
        }
    }
    fs::remove_dir_all(txn_root)
        .with_context(|| format!("failed to remove `{}`", txn_root.display()))?;
    Ok(())
}

fn recover_transaction_write(
    repository: &Path,
    txn_root: &Path,
    write: &TransactionJournalWrite,
) -> Result<()> {
    let target = if write.target == MANIFEST_PATH {
        repository.join(MANIFEST_PATH)
    } else {
        resolve_repository_target(repository, &write.target)?
    };
    ensure_parent_is_safe(repository, &target)?;
    let target_exists = regular_file_exists(&target)?;
    let backup = transaction_file(repository, &txn_root.join("backup"), &write.backup)?;
    let staged = write
        .staged
        .as_ref()
        .map(|path| transaction_file(repository, &txn_root.join("stage"), path))
        .transpose()?;
    let staged_exists = staged.as_deref().map(regular_file_exists).transpose()?;
    if regular_file_exists(&backup)? {
        if target_exists {
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
    if !write.had_original && staged_exists == Some(false) && target_exists {
        fs::remove_file(&target)
            .with_context(|| format!("failed to remove partial `{}`", target.display()))?;
    }
    Ok(())
}

fn canonicalize_existing_repository(repository: &Path) -> Result<PathBuf> {
    let canonical = repository
        .canonicalize()
        .with_context(|| format!("repository `{}` does not exist", repository.display()))?;
    if !canonical.is_dir() {
        bail!("repository `{}` is not a directory", canonical.display());
    }
    Ok(canonical)
}

fn resolve_repository_target(repository: &Path, artifact_path: &str) -> Result<PathBuf> {
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
    if normalized_text != SETUP_CONFIG_PATH && !allowed_repository_target(normalized_text) {
        bail!("artifact path `{artifact_path}` is not an allowed host artifact path");
    }
    let target = repository.join(normalized);
    ensure_parent_is_safe(repository, &target)?;
    Ok(target)
}

fn allowed_repository_target(path: &str) -> bool {
    if path == ".codex/config.toml" {
        return true;
    }
    if path == ".pi/settings.json" {
        return true;
    }
    [
        ".codex/agents/",
        ".claude/agents/",
        ".cursor/agents/",
        ".opencode/agents/",
        ".pi/agents/",
        ".pi/chains/",
        ".planr/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

fn ensure_parent_is_safe(repository: &Path, target: &Path) -> Result<()> {
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
            let metadata = match fs::symlink_metadata(&current) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            {
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

fn write_manifest_file(repository: &Path, manifest: &ManagedManifest) -> Result<()> {
    let manifest_path = repository.join(MANIFEST_PATH);
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create `{}`", parent.display()))?;
    }
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)
        .with_context(|| format!("failed to write `{}`", manifest_path.display()))?;
    Ok(())
}

fn remove_manifest(repository: &Path) -> Result<()> {
    let manifest_path = repository.join(MANIFEST_PATH);
    if manifest_path.exists() {
        fs::remove_file(&manifest_path)
            .with_context(|| format!("failed to remove `{}`", manifest_path.display()))?;
    }
    Ok(())
}

fn read_manifest(repository: &Path) -> Result<Option<ManagedManifest>> {
    let manifest_path = repository.join(MANIFEST_PATH);
    if !regular_file_exists(&manifest_path)? {
        return Ok(None);
    }
    let input = fs::read(&manifest_path)
        .with_context(|| format!("failed to read `{}`", manifest_path.display()))?;
    Ok(Some(serde_json::from_slice(&input).with_context(|| {
        format!("failed to parse `{}`", manifest_path.display())
    })?))
}

fn repair_for_status(status: &str) -> Option<String> {
    match status {
        "modified" | "preserved-modified" => Some(
            "user-modified file preserved; reconcile local edits before uninstalling again"
                .to_string(),
        ),
        "missing" => Some("managed file is missing; uninstall to drop ownership".to_string()),
        _ => None,
    }
}

fn regular_file_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => bail!(
            "path `{}` must be a regular file, not a symlink or directory",
            path.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn transaction_file(repository: &Path, directory: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    if !path
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        bail!("transaction path must be repository-relative without traversal");
    }
    let path = repository.join(path);
    if !path.starts_with(directory) || path == directory {
        bail!("transaction path is outside its recorded transaction directory");
    }
    ensure_parent_is_safe(repository, &path)?;
    Ok(path)
}
