use crate::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "lifecycle_transactions.rs"]
mod transactions;

fn temp_repo(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("model-routing-{name}-{unique}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn apply_bundle_file_with_bundle(
    repository: &Path,
    bundle: &RoutingBundleV1,
) -> Result<LifecycleReport> {
    let bundle_file = write_bundle_file(repository, "bundle.json", bundle);
    apply_bundle_file(repository, &bundle_file)
}

fn write_bundle_file(repository: &Path, name: &str, bundle: &RoutingBundleV1) -> PathBuf {
    let bundle_file = repository.join(name);
    fs::write(&bundle_file, serde_json::to_vec_pretty(bundle).unwrap()).unwrap();
    bundle_file
}

fn has_transaction_directory(repository: &Path) -> bool {
    fs::read_dir(repository.join(".model-routing"))
        .unwrap()
        .any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("txn-"))
        })
}

fn has_transaction_backup(repository: &Path) -> bool {
    fs::read_dir(repository.join(".model-routing"))
        .unwrap()
        .filter_map(std::result::Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("txn-"))
                && entry.path().join("backup").exists()
        })
}
