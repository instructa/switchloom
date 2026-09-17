use model_routing::{status_repository, uninstall_repository};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const ROLE: &str = ".codex/agents/worker.toml";
const CONFIG: &str = ".codex/config.toml";
const MANIFEST: &str = ".model-routing/manifest.json";
const OWNED_CONFIG: &str = "[agents.worker]\nconfig_file = 'agents/worker.toml'\n";

struct Repository(PathBuf);

impl Repository {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "switchloom-cleanup-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn manifest(&self, artifacts: Vec<Value>) {
        self.write(
            MANIFEST,
            &json!({
                "schema_version": 1,
                "bundle_id": "installed-codex-roles",
                "bundle_sha256": "stored-bundle-digest",
                "artifacts": artifacts,
            })
            .to_string(),
        );
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn artifact(path: &str, content: &str) -> Value {
    json!({ "path": path, "sha256": format!("{:x}", Sha256::digest(content)), "content": content })
}

fn cli(command: &str, repository: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_switchloom"))
        .args([command, "--repository"])
        .arg(repository)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
fn cli_uninstall_removes_owned_files_and_preserves_unrelated_codex_settings() {
    let repo = Repository::new();
    repo.write(ROLE, "model = 'gpt-5.6-sol'\n");
    let own_features = "[features.multi_agent_v2]\nenabled = true\n";
    let owned = format!("{OWNED_CONFIG}{own_features}");
    repo.write(CONFIG, &format!(
        "model = 'personal-model'\n{owned}[agents.personal]\nconfig_file = 'personal.toml'\n[features.other]\nenabled = true\n"
    ));
    repo.manifest(vec![
        artifact(ROLE, "model = 'gpt-5.6-sol'\n"),
        artifact(CONFIG, &owned),
    ]);

    let before = cli("status", &repo.0);
    assert!(
        before["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["status"] == "managed")
    );
    let result = cli("uninstall", &repo.0);
    assert!(
        result["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["status"] == "removed")
    );
    assert!(!repo.0.join(ROLE).exists());
    assert!(!repo.0.join(MANIFEST).exists());
    let config: toml::Value =
        toml::from_str(&fs::read_to_string(repo.0.join(CONFIG)).unwrap()).unwrap();
    assert_eq!(config["model"].as_str(), Some("personal-model"));
    assert_eq!(
        config["agents"]["personal"]["config_file"].as_str(),
        Some("personal.toml")
    );
    assert!(config["agents"].get("worker").is_none());
    assert!(config["features"].get("multi_agent_v2").is_none());
    assert_eq!(config["features"]["other"]["enabled"].as_bool(), Some(true));
    assert!(
        cli("status", &repo.0)["artifacts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn uninstall_retains_modified_files_and_drops_already_missing_files() {
    let repo = Repository::new();
    repo.write(ROLE, "user edits");
    repo.write(CONFIG, "[agents.worker]\nconfig_file = 'my-worker.toml'\n");
    repo.manifest(vec![
        artifact(ROLE, "installed content"),
        artifact(CONFIG, OWNED_CONFIG),
        artifact(".codex/agents/missing.toml", "missing content"),
    ]);
    let result = uninstall_repository(&repo.0).unwrap();
    assert_eq!(result.artifacts[0].status, "preserved-modified");
    assert_eq!(result.artifacts[1].status, "preserved-modified");
    assert_eq!(result.artifacts[2].status, "removed");
    assert_eq!(fs::read_to_string(repo.0.join(ROLE)).unwrap(), "user edits");
    assert_eq!(status_repository(&repo.0).unwrap().artifacts.len(), 2);

    fs::remove_file(repo.0.join(ROLE)).unwrap();
    fs::remove_file(repo.0.join(CONFIG)).unwrap();
    uninstall_repository(&repo.0).unwrap();
    assert!(!repo.0.join(MANIFEST).exists());
}

#[test]
fn preexisting_feature_flags_are_not_owned_by_agent_registration() {
    let repo = Repository::new();
    let flags = "[features.multi_agent_v2]\nenabled = true\nhide_spawn_agent_metadata = true\n";
    repo.write(CONFIG, &format!("{OWNED_CONFIG}{flags}"));
    let mut config = artifact(CONFIG, &format!("{OWNED_CONFIG}{flags}"));
    config["ownership_content"] = Value::from(OWNED_CONFIG);
    repo.manifest(vec![config]);
    uninstall_repository(&repo.0).unwrap();
    let config: toml::Value =
        toml::from_str(&fs::read_to_string(repo.0.join(CONFIG)).unwrap()).unwrap();
    assert!(config.get("agents").is_none());
    assert_eq!(
        config["features"]["multi_agent_v2"]["enabled"].as_bool(),
        Some(true)
    );
}

#[test]
fn cleanup_refuses_unmanaged_or_escaping_manifest_paths() {
    let outside = Repository::new();
    outside.write("important.txt", "keep me");
    let absolute = outside.0.join("important.txt").display().to_string();
    for path in [
        absolute.as_str(),
        "../important.txt",
        "src/main.rs",
        ".codex/../important.txt",
        MANIFEST,
    ] {
        let repo = Repository::new();
        repo.manifest(vec![artifact(path, "keep me")]);
        assert!(status_repository(&repo.0).is_err());
        assert!(uninstall_repository(&repo.0).is_err());
        assert!(repo.0.join(MANIFEST).exists());
        assert_eq!(
            fs::read_to_string(outside.0.join("important.txt")).unwrap(),
            "keep me"
        );
    }
}

#[cfg(unix)]
#[test]
fn cleanup_never_follows_artifact_or_manifest_symlinks() {
    use std::os::unix::fs::symlink;
    let outside = Repository::new();
    outside.write("agents/worker.toml", "keep me");
    for path in [".codex", ROLE, ".model-routing", MANIFEST] {
        let repo = Repository::new();
        repo.manifest(vec![artifact(ROLE, "keep me")]);
        let link = repo.0.join(path);
        if link.exists() {
            if link.is_dir() {
                fs::remove_dir_all(&link).unwrap();
            } else {
                fs::remove_file(&link).unwrap();
            }
        }
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let target = if path == ROLE || path == MANIFEST {
            outside.0.join("agents/worker.toml")
        } else {
            outside.0.clone()
        };
        symlink(&target, &link).unwrap();
        assert!(uninstall_repository(&repo.0).is_err(), "followed {path}");
        assert_eq!(
            fs::read_to_string(outside.0.join("agents/worker.toml")).unwrap(),
            "keep me"
        );
    }
}

fn journal_write(target: &str, index: usize, original: bool) -> Value {
    json!({
        "label": target, "target": target,
        "staged": format!(".model-routing/txn-pending/stage/artifact-{index}"),
        "backup": format!(".model-routing/txn-pending/backup/artifact-{index}"),
        "committed": true, "backup_created": original, "had_original": original,
    })
}

#[test]
fn cleanup_recovers_interrupted_writes_before_reading_managed_files() {
    let repo = Repository::new();
    let extra = ".codex/agents/partial.toml";
    repo.write(ROLE, "interrupted update");
    repo.write(extra, "partially installed role");
    repo.write(
        ".model-routing/txn-pending/backup/artifact-0",
        "original content",
    );
    repo.manifest(vec![artifact(ROLE, "original content")]);
    repo.write(".model-routing/txn-pending/journal.json", &json!({
        "schema_version": 1, "writes": [journal_write(ROLE, 0, true), journal_write(extra, 1, false)]
    }).to_string());

    let result = status_repository(&repo.0).unwrap();
    assert_eq!(result.artifacts[0].status, "managed");
    assert_eq!(
        fs::read_to_string(repo.0.join(ROLE)).unwrap(),
        "original content"
    );
    assert!(!repo.0.join(extra).exists());
    assert!(!repo.0.join(".model-routing/txn-pending").exists());
    uninstall_repository(&repo.0).unwrap();
    assert!(!repo.0.join(ROLE).exists());
}

#[test]
fn recovery_rejects_paths_outside_the_transaction_and_keeps_backups() {
    let outside = Repository::new();
    outside.write("important.txt", "keep me");
    for field in ["target", "backup", "staged"] {
        let repo = Repository::new();
        repo.write(ROLE, "pending content");
        repo.write(
            ".model-routing/txn-pending/backup/artifact-0",
            "original content",
        );
        let mut write = journal_write(ROLE, 0, true);
        write[field] = Value::from(outside.0.join("important.txt").display().to_string());
        repo.write(
            ".model-routing/txn-pending/journal.json",
            &json!({
                "schema_version": 1, "writes": [write],
            })
            .to_string(),
        );
        assert!(uninstall_repository(&repo.0).is_err());
        assert!(
            repo.0
                .join(".model-routing/txn-pending/backup/artifact-0")
                .exists()
        );
        assert_eq!(
            fs::read_to_string(repo.0.join(ROLE)).unwrap(),
            "pending content"
        );
        assert_eq!(
            fs::read_to_string(outside.0.join("important.txt")).unwrap(),
            "keep me"
        );
    }
}
