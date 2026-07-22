use anyhow::{Context, Result, bail, ensure};
use model_routing::{Integration, catalog_json, compile_json};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const FORBIDDEN_PUBLIC_PATHS: &[&str] = &[
    "xtask/",
    "scripts/",
    "reports/",
    "retained-evidence/",
    "evidence/",
    ".planr/",
    ".codex/",
    ".claude/",
    ".cursor/",
    "tmp/",
];
const FORBIDDEN_PUBLIC_WORDS: &[&str] = &["credential", "secret", "receipt"];

pub(crate) struct PrepareOptions {
    pub root: PathBuf,
    pub version: Option<String>,
    pub allow_dirty: bool,
}

pub(crate) struct VerifyOptions {
    pub root: PathBuf,
    pub inventory_only: bool,
    pub contract_only: bool,
    pub require_provenance: bool,
    pub expected_tag: Option<String>,
}

pub(crate) struct PackageOptions {
    pub root: PathBuf,
    pub target: Option<String>,
    pub cargo_target: Option<String>,
    pub stage_npm: bool,
    pub assemble_provenance: bool,
    pub aggregate_checksums_dir: Option<PathBuf>,
    pub provenance_dir: Option<PathBuf>,
    pub runner: String,
    pub git_sha: Option<String>,
    pub built_at: String,
    pub generated_by: String,
}

pub(crate) fn prepare(options: PrepareOptions) -> Result<()> {
    if !options.allow_dirty {
        ensure_clean_worktree(&options.root)?;
    }
    if let Some(version) = options.version.as_deref() {
        ensure!(
            valid_release_version(version),
            "invalid release version {version}"
        );
        replace_manifest_versions(&options.root, version)?;
        run(&options.root, "cargo", &["check", "--quiet"])?;
        run(
            &options.root,
            "cargo",
            &[
                "run",
                "--quiet",
                "-p",
                "xtask",
                "--",
                "release",
                "prepare",
                "--allow-dirty",
            ],
        )?;
        return Ok(());
    }
    regenerate_catalog(&options.root)?;
    for (host, output) in [
        (
            "codex-openai",
            "fixtures/routing-bundle-v1/valid-balanced-codex.json",
        ),
        (
            "mixed-host",
            "fixtures/routing-bundle-v1/valid-balanced-mixed.json",
        ),
    ] {
        fs::write(
            options.root.join(output),
            compile_json("balanced", host, Integration::Planr)?,
        )?;
    }
    verify_version_contract(&options.root)?;
    println!("release preparation passed");
    Ok(())
}

pub(crate) fn verify(options: VerifyOptions) -> Result<()> {
    let version = verify_version_contract(&options.root)?;
    if let Some(expected_tag) = options.expected_tag.as_deref() {
        ensure!(
            expected_tag == format!("v{version}"),
            "release tag {expected_tag} does not match manifest version v{version}"
        );
    }
    if options.contract_only {
        println!("release contract passed for v{version}");
        return Ok(());
    }
    verify_catalog(&options.root)?;
    verify_public_inventories(&options.root)?;
    if options.require_provenance {
        ensure!(
            options.root.join("npm/native/provenance.json").is_file(),
            "required native provenance is missing"
        );
        verify_native_provenance(&options.root)?;
    }
    if !options.inventory_only {
        run(&options.root, "cargo", &["fmt", "--all", "--", "--check"])?;
        run(
            &options.root,
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        )?;
        run(
            &options.root,
            "cargo",
            &["test", "--workspace", "--all-targets", "--all-features"],
        )?;
        verify_packaged_source_tests(&options.root, &version)?;
        run(
            &options.root,
            "sh",
            &["scripts/check-migration-manifest.sh"],
        )?;
        run(
            &options.root,
            "node",
            &["scripts/check-evidence-validator-parity.mjs"],
        )?;
        run(&options.root, "node", &["scripts/build-site.mjs"])?;
        run(&options.root, "betterleaks", &["dir", "."])?;
        run(
            &options.root,
            "trivy",
            &[
                "fs",
                "--skip-db-update",
                "--skip-java-db-update",
                "--scanners",
                "vuln,secret,misconfig",
                "--skip-dirs",
                "node_modules",
                "--skip-dirs",
                "target",
                "--skip-dirs",
                "dist",
                "--skip-dirs",
                ".pnpm-store",
                ".",
            ],
        )?;
        run(&options.root, "zizmor", &[".github/workflows"])?;
    }
    println!("release verification passed");
    Ok(())
}

pub(crate) fn package(options: PackageOptions) -> Result<()> {
    let version = verify_version_contract(&options.root)?;
    if let Some(directory) = options.aggregate_checksums_dir.as_deref() {
        aggregate_release_checksums(&options.root, directory)?;
        println!("aggregate release checksums generated");
        return Ok(());
    }
    if options.assemble_provenance {
        assemble_native_provenance(&options, &version)?;
        println!("native provenance assembled for {version}");
        return Ok(());
    }
    let target = options.target.unwrap_or_else(detect_target);
    validate_package_target(&target, options.cargo_target.as_deref(), &detect_target())?;
    let mut cargo_args = vec!["build", "--release", "--locked", "--bin", "model-routing"];
    if let Some(cargo_target) = options.cargo_target.as_deref() {
        cargo_args.extend(["--target", cargo_target]);
    }
    run(&options.root, "cargo", &cargo_args)?;
    let binary = match options.cargo_target.as_deref() {
        Some(cargo_target) => options
            .root
            .join("target")
            .join(cargo_target)
            .join("release/model-routing"),
        None => options.root.join("target/release/model-routing"),
    };
    ensure!(
        binary.is_file(),
        "release binary missing at {}",
        binary.display()
    );
    let dist = options.root.join("dist");
    let stage = dist.join(format!("switchloom-{version}"));
    let archive = absolute(
        &std::env::current_dir()?,
        &dist.join(format!("switchloom-{target}.tar.gz")),
    );
    remove_owned_path(&options.root, &stage)?;
    if archive.exists() {
        fs::remove_file(&archive)?;
    }
    fs::create_dir_all(&stage)?;
    for (source, destination) in [
        (binary.as_path(), stage.join("model-routing")),
        (
            options.root.join("README.md").as_path(),
            stage.join("README.md"),
        ),
        (
            options.root.join("LICENSE").as_path(),
            stage.join("LICENSE"),
        ),
    ] {
        fs::copy(source, destination)?;
    }
    let sums = ["model-routing", "README.md", "LICENSE"]
        .into_iter()
        .map(|name| Ok(format!("{}  {name}\n", sha256_file(&stage.join(name))?)))
        .collect::<Result<String>>()?;
    fs::write(stage.join("SHA256SUMS"), sums)?;
    run(
        &stage,
        "tar",
        &[
            "-czf",
            archive.to_str().context("archive path is not UTF-8")?,
            "model-routing",
            "README.md",
            "LICENSE",
            "SHA256SUMS",
        ],
    )?;
    if options.stage_npm {
        let npm_binary = options
            .root
            .join("npm/native")
            .join(&target)
            .join("model-routing");
        if let Some(parent) = npm_binary.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&binary, &npm_binary)?;
        if let Some(provenance_dir) = options.provenance_dir {
            let git_sha = options
                .git_sha
                .map(Ok)
                .unwrap_or_else(|| git_stdout(&options.root, &["rev-parse", "HEAD"]))?;
            ensure!(
                is_git_sha(&git_sha),
                "git SHA must be 40 lowercase hex characters"
            );
            let rust_target = options.cargo_target.unwrap_or_else(|| target.clone());
            let receipt = serde_json::json!({
                "target": target,
                "rust_target": rust_target,
                "runner": options.runner,
                "path": format!("npm/native/{target}/model-routing"),
                "version": format!("model-routing {version}"),
                "sha256": sha256_file(&npm_binary)?,
                "git_sha": git_sha,
                "built_at": options.built_at,
            });
            let target_dir = absolute(&options.root, &provenance_dir).join(&target);
            fs::create_dir_all(&target_dir)?;
            fs::write(
                target_dir.join("provenance.json"),
                format!("{}\n", serde_json::to_string_pretty(&receipt)?),
            )?;
            fs::write(
                target_dir.join("SHA256SUMS"),
                format!(
                    "{}  npm/native/{target}/model-routing\n",
                    sha256_file(&npm_binary)?
                ),
            )?;
        }
    }
    println!("release artifact: {}", archive.display());
    Ok(())
}

fn assemble_native_provenance(options: &PackageOptions, version: &str) -> Result<()> {
    let git_sha = options
        .git_sha
        .clone()
        .map(Ok)
        .unwrap_or_else(|| git_stdout(&options.root, &["rev-parse", "HEAD"]))?;
    ensure!(
        is_git_sha(&git_sha),
        "git SHA must be 40 lowercase hex characters"
    );
    let targets = [
        "darwin-arm64",
        "darwin-x86_64",
        "linux-arm64",
        "linux-x86_64",
    ]
    .into_iter()
    .map(|target| {
        let (rust_target, runner) = target_metadata(target)?;
        let relative = format!("npm/native/{target}/model-routing");
        let binary = options.root.join(&relative);
        ensure!(
            binary.is_file(),
            "native binary missing at {}",
            binary.display()
        );
        Ok(serde_json::json!({
            "target": target,
            "rust_target": rust_target,
            "runner": runner,
            "path": relative,
            "version": format!("model-routing {version}"),
            "sha256": sha256_file(&binary)?,
            "git_sha": git_sha,
            "built_at": options.built_at,
        }))
    })
    .collect::<Result<Vec<_>>>()?;
    let provenance = serde_json::json!({
        "schema_version": "switchloom.native_provenance.v1",
        "package_version": version,
        "git_sha": git_sha,
        "generated_by": options.generated_by,
        "targets": targets,
    });
    let output = options.root.join("npm/native/provenance.json");
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        output,
        format!("{}\n", serde_json::to_string_pretty(&provenance)?),
    )?;
    verify_native_provenance(&options.root)
}

fn aggregate_release_checksums(root: &Path, directory: &Path) -> Result<()> {
    let directory = absolute(root, directory);
    ensure!(directory.is_dir(), "release asset directory missing");
    let expected = BTreeSet::from([
        "switchloom-darwin-arm64.tar.gz".to_owned(),
        "switchloom-darwin-x86_64.tar.gz".to_owned(),
        "switchloom-linux-arm64.tar.gz".to_owned(),
        "switchloom-linux-x86_64.tar.gz".to_owned(),
    ]);
    let actual = fs::read_dir(&directory)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("switchloom-") && name.ends_with(".tar.gz"))
        .collect::<BTreeSet<_>>();
    ensure!(
        actual == expected,
        "release archive set mismatch: expected {expected:?}, found {actual:?}"
    );
    let checksums = actual
        .iter()
        .map(|name| Ok(format!("{}  {name}\n", sha256_file(&directory.join(name))?)))
        .collect::<Result<String>>()?;
    fs::write(directory.join("SHA256SUMS"), checksums)?;
    Ok(())
}

fn regenerate_catalog(root: &Path) -> Result<()> {
    let catalog = root.join("website/data/catalog.json");
    let generated = catalog_json()?;
    fs::write(&catalog, &generated)?;
    let data: Value = serde_json::from_str(&generated)?;
    let compositions = data["compositions"]
        .as_array()
        .context("catalog compositions must be an array")?;
    let bundles = root.join("website/data/bundles");
    remove_owned_path(root, &bundles)?;
    fs::create_dir_all(&bundles)?;
    for entry in compositions {
        let id = entry["entryId"]
            .as_str()
            .context("catalog entryId missing")?;
        let policy = entry["policy"]["id"]
            .as_str()
            .context("catalog policy missing")?;
        let host = entry["binding"]["id"]
            .as_str()
            .context("catalog binding missing")?;
        fs::write(
            bundles.join(format!("{id}.json")),
            compile_json(policy, host, Integration::Standalone)?,
        )?;
    }
    println!("regenerated {} catalog compositions", compositions.len());
    Ok(())
}

fn verify_catalog(root: &Path) -> Result<()> {
    let current = fs::read_to_string(root.join("website/data/catalog.json"))?;
    ensure!(
        current == catalog_json()?,
        "catalog does not match package-owned generated sources"
    );
    println!("catalog verified");
    Ok(())
}

fn verify_public_inventories(root: &Path) -> Result<()> {
    verify_publication_boundary(root)?;
    let cargo = output(
        root,
        "cargo",
        &[
            "package",
            "--package",
            "model-routing",
            "--list",
            "--allow-dirty",
            "--no-verify",
            "--offline",
        ],
    )?;
    let cargo_files = lines(&cargo.stdout);
    verify_inventory("Cargo", &cargo_files)?;
    let npm = output(root, "npm", &["pack", "--dry-run", "--json"])?;
    let npm_value: Value = serde_json::from_slice(&npm.stdout)?;
    let npm_files = npm_value
        .pointer("/0/files")
        .and_then(Value::as_array)
        .context("npm pack JSON has no file inventory")?
        .iter()
        .map(|entry| entry["path"].as_str().unwrap_or_default().to_owned())
        .collect::<Vec<_>>();
    verify_inventory("npm", &npm_files)?;
    ensure!(
        cargo_files.iter().any(|path| path == "src/lib.rs"),
        "Cargo package omitted src/lib.rs"
    );
    ensure!(
        npm_files
            .iter()
            .any(|path| path == "npm/bin/model-routing.js"),
        "npm package omitted launcher"
    );
    println!(
        "public inventories passed: {} Cargo files, {} npm files",
        cargo_files.len(),
        npm_files.len()
    );
    Ok(())
}

fn verify_packaged_source_tests(root: &Path, version: &str) -> Result<()> {
    run(
        root,
        "cargo",
        &[
            "package",
            "--package",
            "model-routing",
            "--allow-dirty",
            "--no-verify",
            "--offline",
        ],
    )?;
    let archive = fs::canonicalize(
        root.join("target/package")
            .join(format!("model-routing-{version}.crate")),
    )?;
    ensure!(
        archive.is_file(),
        "Cargo package archive missing at {}",
        archive.display()
    );

    let stage = std::env::temp_dir().join(format!("switchloom-package-source-test-{version}"));
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    fs::create_dir_all(&stage)?;
    let archive = archive
        .to_str()
        .context("Cargo package archive path is not UTF-8")?;
    run(&stage, "tar", &["-xzf", archive])?;

    let unpacked = stage.join(format!("model-routing-{version}"));
    ensure!(
        unpacked.is_dir(),
        "unpacked Cargo package missing at {}",
        unpacked.display()
    );
    run(
        &unpacked,
        "cargo",
        &[
            "test",
            "--offline",
            "codex_runtime_evidence_rejects_retained_source_without_claimed_raw_output",
            "--",
            "--nocapture",
        ],
    )?;
    println!("packaged source provenance test passed");
    Ok(())
}

fn verify_publication_boundary(root: &Path) -> Result<()> {
    let metadata = output(
        root,
        "cargo",
        &["metadata", "--format-version", "1", "--no-deps"],
    )?;
    let metadata: Value = serde_json::from_slice(&metadata.stdout)?;
    let packages = metadata["packages"]
        .as_array()
        .context("Cargo metadata packages missing")?;
    let publishable = packages
        .iter()
        .filter(|package| {
            package["publish"]
                .as_array()
                .is_none_or(|registries| !registries.is_empty())
        })
        .filter_map(|package| package["name"].as_str())
        .collect::<Vec<_>>();
    ensure!(
        publishable == ["model-routing"],
        "only model-routing may be published; found {publishable:?}"
    );
    Ok(())
}

fn verify_inventory(kind: &str, files: &[String]) -> Result<()> {
    for file in files {
        let normalized = file.trim_start_matches("./").to_ascii_lowercase();
        if FORBIDDEN_PUBLIC_PATHS.iter().any(|prefix| {
            normalized == prefix.trim_end_matches('/') || normalized.starts_with(prefix)
        }) || FORBIDDEN_PUBLIC_WORDS
            .iter()
            .any(|word| normalized.contains(word))
        {
            bail!("{kind} public artifact contains forbidden path {file}");
        }
    }
    Ok(())
}

fn verify_version_contract(root: &Path) -> Result<String> {
    let version = workspace_version(root)?;
    ensure!(
        valid_release_version(&version),
        "invalid workspace version {version}"
    );
    let package: Value = serde_json::from_slice(&fs::read(root.join("package.json"))?)?;
    ensure!(
        package["version"].as_str() == Some(&version),
        "Cargo/npm version mismatch"
    );
    let xtask = fs::read_to_string(root.join("xtask/Cargo.toml"))?;
    ensure!(
        xtask.contains(&format!(
            "model-routing = {{ path = \"..\", version = \"{version}\" }}"
        )),
        "xtask path dependency version does not match workspace version"
    );
    let changelog = fs::read_to_string(root.join("CHANGELOG.md"))?;
    ensure!(
        changelog.contains(&format!("## [{version}]")),
        "CHANGELOG has no {version} section"
    );
    Ok(version)
}

fn workspace_version(root: &Path) -> Result<String> {
    let manifest = fs::read_to_string(root.join("Cargo.toml"))?;
    let mut in_workspace_package = false;
    for line in manifest.lines() {
        if line.starts_with('[') {
            in_workspace_package = line.trim() == "[workspace.package]";
        } else if in_workspace_package {
            if let Some(version) = line
                .strip_prefix("version = \"")
                .and_then(|line| line.strip_suffix('"'))
            {
                return Ok(version.to_owned());
            }
        }
    }
    bail!("workspace package version missing")
}

fn replace_manifest_versions(root: &Path, version: &str) -> Result<()> {
    replace_once(
        &root.join("Cargo.toml"),
        &format!("version = \"{}\"", workspace_version(root)?),
        &format!("version = \"{version}\""),
    )?;
    let package_path = root.join("package.json");
    let old_npm = serde_json::from_slice::<Value>(&fs::read(&package_path)?)?["version"]
        .as_str()
        .context("package version missing")?
        .to_owned();
    replace_once(
        &package_path,
        &format!("\"version\": \"{old_npm}\""),
        &format!("\"version\": \"{version}\""),
    )?;
    let xtask_path = root.join("xtask/Cargo.toml");
    let xtask = fs::read_to_string(&xtask_path)?;
    let start = "model-routing = { path = \"..\", version = \"";
    let old = xtask
        .lines()
        .find(|line| line.starts_with(start))
        .context("xtask model-routing dependency missing")?;
    replace_once(
        &xtask_path,
        old,
        &format!("model-routing = {{ path = \"..\", version = \"{version}\" }}"),
    )?;
    Ok(())
}

fn replace_once(path: &Path, old: &str, new: &str) -> Result<()> {
    let text = fs::read_to_string(path)?;
    ensure!(
        text.matches(old).count() == 1,
        "expected one version field in {}",
        path.display()
    );
    fs::write(path, text.replacen(old, new, 1))?;
    Ok(())
}

fn ensure_clean_worktree(root: &Path) -> Result<()> {
    let status = git_stdout(root, &["status", "--porcelain=v1", "--untracked-files=all"])?;
    ensure!(
        status.is_empty(),
        "worktree is dirty; commit or stash before preparing a release"
    );
    Ok(())
}

#[derive(Deserialize)]
struct NativeProvenance {
    schema_version: String,
    package_version: String,
    git_sha: String,
    generated_by: String,
    targets: Vec<NativeTarget>,
}

#[derive(Deserialize)]
struct NativeTarget {
    target: String,
    rust_target: String,
    runner: String,
    path: String,
    version: String,
    sha256: String,
    git_sha: String,
    built_at: String,
}

fn verify_native_provenance(root: &Path) -> Result<()> {
    let provenance: NativeProvenance =
        serde_json::from_slice(&fs::read(root.join("npm/native/provenance.json"))?)?;
    ensure!(
        provenance.schema_version == "switchloom.native_provenance.v1",
        "unsupported native provenance schema"
    );
    ensure!(
        !provenance.generated_by.trim().is_empty(),
        "provenance generator missing"
    );
    ensure!(
        provenance.package_version == workspace_version(root)?,
        "provenance version mismatch"
    );
    ensure!(
        is_git_sha(&provenance.git_sha),
        "invalid provenance git SHA"
    );
    let expected = BTreeSet::from([
        "darwin-arm64".to_owned(),
        "darwin-x86_64".to_owned(),
        "linux-arm64".to_owned(),
        "linux-x86_64".to_owned(),
    ]);
    let actual = provenance
        .targets
        .iter()
        .map(|target| target.target.clone())
        .collect::<BTreeSet<_>>();
    ensure!(actual == expected, "native provenance target set mismatch");
    for target in provenance.targets {
        let (expected_rust_target, _) = target_metadata(&target.target)?;
        ensure!(
            target.git_sha == provenance.git_sha,
            "target git SHA mismatch"
        );
        ensure!(
            target.version == format!("model-routing {}", provenance.package_version),
            "target version mismatch"
        );
        ensure!(
            target.rust_target == expected_rust_target,
            "target Rust triple mismatch"
        );
        ensure!(!target.runner.trim().is_empty(), "target runner missing");
        ensure!(
            !target.built_at.trim().is_empty(),
            "target build identity missing"
        );
        ensure!(
            target.sha256.len() == 64
                && target.sha256.chars().all(|character| {
                    character.is_ascii_hexdigit() && !character.is_ascii_uppercase()
                }),
            "invalid target digest"
        );
        ensure!(
            target.path == format!("npm/native/{}/model-routing", target.target),
            "native path does not match target"
        );
        let path = root.join(&target.path);
        ensure!(
            path.starts_with(root.join("npm/native")),
            "native path escapes npm/native"
        );
        ensure!(
            path.is_file(),
            "native binary missing at {}",
            path.display()
        );
        ensure!(
            sha256_file(&path)? == target.sha256,
            "native digest mismatch for {}",
            target.target
        );
    }
    let current_target = detect_target();
    if valid_target(&current_target) {
        let binary = root
            .join("npm/native")
            .join(current_target)
            .join("model-routing");
        if binary.is_file() {
            let output = Command::new(&binary).arg("--version").output()?;
            ensure!(
                output.status.success(),
                "current native binary version check failed"
            );
            ensure!(
                String::from_utf8(output.stdout)?.trim()
                    == format!("model-routing {}", provenance.package_version),
                "current native binary version mismatch"
            );
        }
    }
    println!(
        "native provenance validated for {}",
        provenance.package_version
    );
    Ok(())
}

fn target_metadata(target: &str) -> Result<(&'static str, &'static str)> {
    match target {
        "darwin-arm64" => Ok(("aarch64-apple-darwin", "macos-14")),
        "darwin-x86_64" => Ok(("x86_64-apple-darwin", "macos-14")),
        "linux-arm64" => Ok(("aarch64-unknown-linux-gnu", "ubuntu-24.04-arm")),
        "linux-x86_64" => Ok(("x86_64-unknown-linux-gnu", "ubuntu-24.04")),
        _ => bail!("unsupported native target {target}"),
    }
}

fn valid_release_version(value: &str) -> bool {
    let (core, suffix) = value
        .split_once('-')
        .map_or((value, None), |(core, suffix)| (core, Some(suffix)));
    let core_valid = core.split('.').count() == 3
        && core
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    let suffix_valid = suffix.is_none_or(|suffix| {
        ["alpha", "beta", "rc"].into_iter().any(|kind| {
            suffix
                .strip_prefix(kind)
                .and_then(|rest| rest.strip_prefix('.'))
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
    });
    core_valid && suffix_valid
}

fn valid_target(target: &str) -> bool {
    matches!(
        target,
        "darwin-arm64" | "darwin-x86_64" | "linux-arm64" | "linux-x86_64"
    )
}

fn validate_package_target(
    target: &str,
    cargo_target: Option<&str>,
    detected_host: &str,
) -> Result<()> {
    ensure!(valid_target(target), "unsupported release target {target}");
    match cargo_target {
        Some(cargo_target) => {
            let (expected, _) = target_metadata(target)?;
            ensure!(
                cargo_target == expected,
                "release target {target} requires Cargo target {expected}, got {cargo_target}"
            );
        }
        None => ensure!(
            target == detected_host,
            "release target {target} does not match detected host {detected_host}; pass its matching --cargo-target"
        ),
    }
    Ok(())
}

fn detect_target() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        other => other,
    };
    format!("{os}-{arch}")
}

fn is_git_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

fn remove_owned_path(root: &Path, path: &Path) -> Result<()> {
    ensure!(
        path.starts_with(root),
        "refusing to remove path outside repository"
    );
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn absolute(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        root.join(path)
    }
}

fn lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::to_owned)
        .collect()
}

fn git_stdout(root: &Path, args: &[&str]) -> Result<String> {
    let output = output(root, "git", args)?;
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn run(root: &Path, program: &str, args: &[&str]) -> Result<()> {
    run_path(root, Path::new(program), args)
}

fn run_path(root: &Path, program: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .with_context(|| format!("failed to run {}", program.display()))?;
    ensure!(
        status.success(),
        "{} exited with status {status}",
        program.display()
    );
    Ok(())
}

fn output(root: &Path, program: &str, args: &[&str]) -> Result<Output> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("failed to run {program}"))?;
    if !output.status.success() {
        bail!(
            "{program} exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_versions_are_bounded() {
        for valid in ["1.2.3", "1.2.3-alpha.1", "1.2.3-beta.9", "1.2.3-rc.0"] {
            assert!(valid_release_version(valid));
        }
        for invalid in ["v1.2.3", "1.2", "1.2.3-dev.1", "1.2.3-rc", "1.2.3.4"] {
            assert!(!valid_release_version(invalid));
        }
    }

    #[test]
    fn public_inventory_rejects_internal_and_sensitive_paths() {
        for path in [
            "xtask/src/main.rs",
            "scripts/release.sh",
            "reports/live.json",
            "retained-evidence/report.json",
            "tmp/state",
            "docs/credential-receipt.json",
        ] {
            assert!(
                verify_inventory("test", &[path.into()]).is_err(),
                "accepted {path}"
            );
        }
        verify_inventory("test", &["src/lib.rs".into(), "README.md".into()]).unwrap();
    }

    #[test]
    fn archive_label_must_match_host_or_explicit_cargo_triple() {
        assert!(validate_package_target("darwin-arm64", None, "darwin-arm64").is_ok());
        assert!(validate_package_target("linux-x86_64", None, "darwin-arm64").is_err());
        assert!(
            validate_package_target(
                "linux-x86_64",
                Some("x86_64-unknown-linux-gnu"),
                "darwin-arm64"
            )
            .is_ok()
        );
        assert!(
            validate_package_target("linux-x86_64", Some("aarch64-apple-darwin"), "darwin-arm64")
                .is_err()
        );
    }
}
