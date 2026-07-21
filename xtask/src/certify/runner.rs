use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

static NEXT_RUN: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub(crate) struct OwnedReportRepo {
    pub report_dir: PathBuf,
    pub workdir: PathBuf,
}

impl OwnedReportRepo {
    pub fn create(report_root: &Path, host: &str) -> Result<Self> {
        let report_root = if report_root.is_absolute() {
            report_root.to_owned()
        } else {
            std::env::current_dir()?.join(report_root)
        };
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let suffix = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
        let report_dir = report_root
            .join(host)
            .join(format!("{epoch}-{}-{suffix}", std::process::id()));
        let workdir = report_dir.join("workdir");
        fs::create_dir_all(&workdir).context("failed to create owned certification repository")?;
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&workdir)
            .status()
            .context("failed to initialize owned certification repository")?;
        if !status.success() {
            bail!("git init failed for owned certification repository");
        }
        Ok(Self {
            report_dir,
            workdir,
        })
    }
}

#[derive(Debug)]
pub(crate) struct FileSnapshot {
    path: PathBuf,
    original: Option<Vec<u8>>,
    restored: bool,
    outcome: RestorationTracker,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "status", content = "error", rename_all = "snake_case")]
pub(crate) enum RestorationOutcome {
    NotRequired,
    Pending,
    RestoredExplicitly,
    RestoredByDropFallback,
    Failed(String),
}

pub(crate) type RestorationTracker = Rc<RefCell<RestorationOutcome>>;

impl FileSnapshot {
    pub fn capture(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let original = if path.exists() {
            Some(
                fs::read(&path)
                    .with_context(|| format!("failed to snapshot {}", path.display()))?,
            )
        } else {
            None
        };
        Ok(Self {
            path,
            original,
            restored: false,
            outcome: Rc::new(RefCell::new(RestorationOutcome::Pending)),
        })
    }

    pub fn restore(&mut self) -> Result<()> {
        match restore_file(&self.path, self.original.as_deref()) {
            Ok(()) => {
                self.restored = true;
                *self.outcome.borrow_mut() = RestorationOutcome::RestoredExplicitly;
                Ok(())
            }
            Err(error) => {
                *self.outcome.borrow_mut() = RestorationOutcome::Failed(error.to_string());
                Err(error)
            }
        }
    }

    pub fn outcome_tracker(&self) -> RestorationTracker {
        Rc::clone(&self.outcome)
    }
}

impl Drop for FileSnapshot {
    fn drop(&mut self) {
        if !self.restored {
            *self.outcome.borrow_mut() = match restore_file(&self.path, self.original.as_deref()) {
                Ok(()) => RestorationOutcome::RestoredByDropFallback,
                Err(error) => RestorationOutcome::Failed(error.to_string()),
            };
        }
    }
}

fn restore_file(path: &Path, original: Option<&[u8]>) -> Result<()> {
    match original {
        Some(bytes) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, bytes)
                .with_context(|| format!("failed to restore {}", path.display()))?;
        }
        None if path.exists() => fs::remove_file(path).with_context(|| {
            format!(
                "failed to remove temporary external state {}",
                path.display()
            )
        })?,
        None => {}
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct ProcessSpec {
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: PathBuf,
    pub timeout: Duration,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProcessReceipt {
    pub label: String,
    pub argv: Vec<String>,
    pub env_keys: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub status: Option<i32>,
    pub timed_out: bool,
    pub elapsed_ms: u128,
}

impl ProcessReceipt {
    pub fn success(&self) -> bool {
        self.status == Some(0) && !self.timed_out
    }
}

pub(crate) fn run_process(spec: &ProcessSpec, capture_dir: &Path) -> Result<ProcessReceipt> {
    run_process_with_environment(spec, capture_dir, std::env::vars())
}

fn run_process_with_environment(
    spec: &ProcessSpec,
    capture_dir: &Path,
    ambient_env: impl IntoIterator<Item = (String, String)>,
) -> Result<ProcessReceipt> {
    fs::create_dir_all(capture_dir)?;
    let stdout_path = capture_dir.join(format!("{}.stdout", spec.label));
    let stderr_path = capture_dir.join(format!("{}.stderr", spec.label));
    let raw = RawCapture::create(&spec.label)?;
    let stdout_file = File::create(&raw.stdout)?;
    let stderr_file = File::create(&raw.stderr)?;
    let mut effective_env = ambient_env.into_iter().collect::<BTreeMap<_, _>>();
    effective_env.extend(spec.env.clone());
    let secrets = effective_env
        .iter()
        .filter(|(key, _)| secret_key(key))
        .map(|(_, value)| value.clone())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let started = Instant::now();
    let mut child = Command::new(&spec.program)
        .args(&spec.args)
        .env_clear()
        .envs(&effective_env)
        .current_dir(&spec.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .with_context(|| format!("failed to spawn {}", spec.label))?;
    let (status, timed_out) = wait_bounded(&mut child, spec.timeout)?;
    let secret_refs = secrets.iter().map(String::as_str).collect::<Vec<_>>();
    let stdout = redact(
        &String::from_utf8_lossy(&fs::read(&raw.stdout).unwrap_or_default()),
        &secret_refs,
    );
    let stderr = redact(
        &String::from_utf8_lossy(&fs::read(&raw.stderr).unwrap_or_default()),
        &secret_refs,
    );
    fs::write(&stdout_path, &stdout)?;
    fs::write(&stderr_path, &stderr)?;
    let argv = std::iter::once(spec.program.as_str())
        .chain(spec.args.iter().map(String::as_str))
        .map(|value| redact(value, &secret_refs))
        .collect();
    let receipt = ProcessReceipt {
        label: spec.label.clone(),
        argv,
        env_keys: spec
            .env
            .keys()
            .filter(|key| !secret_key(key))
            .cloned()
            .collect(),
        stdout,
        stderr,
        status: status.code(),
        timed_out,
        elapsed_ms: started.elapsed().as_millis(),
    };
    write_receipt(capture_dir, &receipt)?;
    Ok(receipt)
}

struct RawCapture {
    stdout: PathBuf,
    stderr: PathBuf,
}

impl RawCapture {
    fn create(label: &str) -> Result<Self> {
        let id = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "switchloom-capture-{}-{}-{id}",
            std::process::id(),
            label.replace(|character: char| !character.is_ascii_alphanumeric(), "-")
        ));
        fs::create_dir_all(&root)?;
        Ok(Self {
            stdout: root.join("stdout"),
            stderr: root.join("stderr"),
        })
    }
}

impl Drop for RawCapture {
    fn drop(&mut self) {
        if let Some(root) = self.stdout.parent() {
            let _ = fs::remove_dir_all(root);
        }
    }
}

fn write_receipt(capture_dir: &Path, receipt: &ProcessReceipt) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    fs::write(
        capture_dir.join(format!("{}.receipt.json", receipt.label)),
        bytes,
    )?;
    Ok(())
}

fn wait_bounded(child: &mut std::process::Child, timeout: Duration) -> Result<(ExitStatus, bool)> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok((status, false));
        }
        if Instant::now() >= deadline {
            child
                .kill()
                .context("failed to terminate timed-out host process")?;
            return Ok((child.wait()?, true));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn secret_key(key: &str) -> bool {
    let key = key.to_ascii_uppercase();
    [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "CREDENTIAL",
        "AUTH",
        "API_KEY",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn redact(value: &str, secrets: &[&str]) -> String {
    let mut redacted = value.to_owned();
    for secret in secrets {
        redacted = redacted.replace(secret, "[REDACTED]");
    }
    redacted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "switchloom-runner-{name}-{}-{}",
            std::process::id(),
            NEXT_RUN.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn exit_seven_is_preserved_after_explicit_restoration() {
        let root = temp("exit-7");
        let state = root.join("config.toml");
        fs::write(&state, "original").unwrap();
        let mut snapshot = FileSnapshot::capture(&state).unwrap();
        fs::write(&state, "temporary mutation").unwrap();
        let receipt = run_process(
            &ProcessSpec {
                label: "exit-seven".into(),
                program: "sh".into(),
                args: vec!["-c".into(), "exit 7".into()],
                env: BTreeMap::new(),
                cwd: root.clone(),
                timeout: Duration::from_secs(2),
            },
            &root,
        )
        .unwrap();
        snapshot.restore().unwrap();
        assert_eq!(receipt.status, Some(7));
        let retained: ProcessReceipt =
            serde_json::from_slice(&fs::read(root.join("exit-seven.receipt.json")).unwrap())
                .unwrap();
        assert_eq!(retained.status, Some(7));
        assert_eq!(fs::read_to_string(state).unwrap(), "original");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn drop_fallback_restores_temporary_external_state() {
        let root = temp("drop");
        let state = root.join("config.toml");
        fs::write(&state, "original").unwrap();
        {
            let _snapshot = FileSnapshot::capture(&state).unwrap();
            fs::write(&state, "temporary mutation").unwrap();
        }
        assert_eq!(fs::read_to_string(state).unwrap(), "original");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runner_times_out_and_redacts_secret_values() {
        let root = temp("timeout");
        let secret = "never-print-this-token";
        let receipt = run_process_with_environment(
            &ProcessSpec {
                label: "timeout".into(),
                program: "sh".into(),
                args: vec![
                    "-c".into(),
                    "printf '%s' \"$AMBIENT_AUTH_TOKEN\"; sleep 2".into(),
                ],
                env: BTreeMap::new(),
                cwd: root.clone(),
                timeout: Duration::from_millis(50),
            },
            &root,
            [("AMBIENT_AUTH_TOKEN".into(), secret.into())],
        )
        .unwrap();
        assert!(receipt.timed_out);
        assert_eq!(receipt.status, None);
        assert!(!receipt.stdout.contains(secret));
        assert!(
            !fs::read_to_string(root.join("timeout.stdout"))
                .unwrap()
                .contains(secret)
        );
        assert!(
            !fs::read_to_string(root.join("timeout.stderr"))
                .unwrap()
                .contains(secret)
        );
        let retained: ProcessReceipt =
            serde_json::from_slice(&fs::read(root.join("timeout.receipt.json")).unwrap()).unwrap();
        assert!(retained.timed_out);
        assert_eq!(retained.status, None);
        assert!(!retained.stdout.contains(secret));
        fs::remove_dir_all(root).unwrap();
    }
}
