use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;

const ARCHIVE_NAME: &str = "git-runtime.tar.gz";
const VERSION_NAME: &str = "git-runtime-version.txt";
const READY_NAME: &str = ".ready-version";
const RUNTIME_DIRECTORY: &str = "git-runtime";

#[derive(Clone, Debug)]
pub struct GitRuntime {
    pub executable: PathBuf,
    pub exec_path: PathBuf,
    pub template_dir: Option<PathBuf>,
    pub ca_bundle: Option<PathBuf>,
    pub path_entries: Vec<PathBuf>,
}

impl GitRuntime {
    /// Installs the bundled runtime into the app's writable data directory.
    /// Returns `None` in development builds, where the release archive is absent.
    pub fn prepare(resources: &Path, app_data: &Path) -> Result<Option<Self>, String> {
        let archive_path = resources.join(ARCHIVE_NAME);
        let version_path = resources.join(VERSION_NAME);
        if !archive_path.exists() && !version_path.exists() {
            return Ok(None);
        }
        if !archive_path.is_file() || !version_path.is_file() {
            return Err("bundled Git resources are incomplete".to_string());
        }

        let version = fs::read_to_string(&version_path)
            .map_err(|error| format!("cannot read bundled Git version: {error}"))?;
        let version = version.trim();
        if version.is_empty() {
            return Err("bundled Git version file is empty".to_string());
        }

        let runtime_dir = app_data.join(RUNTIME_DIRECTORY);
        let ready_path = runtime_dir.join(READY_NAME);
        let archive_digest = digest_file(&archive_path)?;
        let marker_contents = format!("{version}\n{archive_digest}");
        let ready = fs::read_to_string(&ready_path)
            .is_ok_and(|installed_marker| installed_marker.trim() == marker_contents)
            && Self::discover(&runtime_dir).is_ok();
        if !ready {
            if runtime_dir.exists() {
                fs::remove_dir_all(&runtime_dir)
                    .map_err(|error| format!("cannot remove old bundled Git runtime: {error}"))?;
            }
            fs::create_dir_all(&runtime_dir)
                .map_err(|error| format!("cannot create bundled Git directory: {error}"))?;
            extract_runtime(&archive_path, &runtime_dir)?;
            run_relocation_fixup(&runtime_dir)?;
            Self::discover(&runtime_dir)?;
            fs::write(&ready_path, marker_contents)
                .map_err(|error| format!("cannot mark bundled Git runtime as ready: {error}"))?;
        }

        Self::discover(&runtime_dir).map(Some)
    }

    fn discover(root: &Path) -> Result<Self, String> {
        let executable = first_existing(
            root,
            if cfg!(windows) {
                &["Library/bin/git.exe", "cmd/git.exe", "bin/git.exe"]
            } else {
                &["bin/git"]
            },
        )
        .ok_or_else(|| "bundled Git archive does not contain a Git executable".to_string())?;

        let exec_path = first_directory(
            root,
            &[
                "libexec/git-core",
                "Library/libexec/git-core",
                "lib/git-core",
                "Library/lib/git-core",
                "mingw64/libexec/git-core",
            ],
        )
        .ok_or_else(|| "bundled Git archive does not contain Git's helper programs".to_string())?;

        let template_dir = first_directory(
            root,
            &[
                "share/git-core/templates",
                "Library/share/git-core/templates",
                "mingw64/share/git-core/templates",
            ],
        );
        let ca_bundle = first_existing(
            root,
            &[
                "ssl/cacert.pem",
                "Library/ssl/cacert.pem",
                "etc/ssl/certs/ca-certificates.crt",
                "Library/etc/ssl/certs/ca-bundle.crt",
                "Library/etc/ssl/cert.pem",
            ],
        );
        let path_entries = [
            "bin",
            "Library/bin",
            "Library/usr/bin",
            "Scripts",
            "cmd",
            "Library/mingw-w64/bin",
            "mingw64/bin",
        ]
        .into_iter()
        .map(|relative| root.join(relative))
        .filter(|path| path.is_dir())
        .collect();

        Ok(Self {
            executable,
            exec_path,
            template_dir,
            ca_bundle,
            path_entries,
        })
    }
}

fn digest_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("cannot open bundled Git archive for verification: {error}"))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot verify bundled Git archive: {error}"))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn extract_runtime(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let archive_file = File::open(archive_path)
        .map_err(|error| format!("cannot open bundled Git archive: {error}"))?;
    let decoder = GzDecoder::new(archive_file);
    let mut archive = Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|error| format!("cannot read bundled Git archive: {error}"))?;
    for entry in entries {
        let mut entry =
            entry.map_err(|error| format!("cannot read bundled Git archive entry: {error}"))?;
        let extracted = entry
            .unpack_in(destination)
            .map_err(|error| format!("cannot extract bundled Git archive entry: {error}"))?;
        if !extracted {
            return Err("bundled Git archive contains a path outside its destination".to_string());
        }
    }
    Ok(())
}

fn run_relocation_fixup(root: &Path) -> Result<(), String> {
    #[cfg(windows)]
    let python = first_existing(root, &["python.exe", "Library/bin/python.exe"]);
    #[cfg(not(windows))]
    let python = first_existing(root, &["bin/python"]);
    let python = python.ok_or_else(|| {
        "bundled Git archive is missing Python required for relocation".to_string()
    })?;

    #[cfg(windows)]
    let fixup = first_existing(root, &["Scripts/conda-unpack-script.py"]);
    #[cfg(not(windows))]
    let fixup = first_existing(root, &["bin/conda-unpack"]);
    let fixup = fixup.ok_or_else(|| {
        "bundled Git archive is missing conda-unpack required for relocation".to_string()
    })?;

    let mut command = Command::new(python);
    command.arg(&fixup).current_dir(root);
    #[cfg(windows)]
    command.env(
        "PATH",
        std::env::join_paths(
            [
                root.join("Library/mingw-w64/bin"),
                root.join("Library/usr/bin"),
                root.join("Library/bin"),
                root.join("Scripts"),
                root.to_path_buf(),
            ]
            .into_iter()
            .chain(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            )),
        )
        .map_err(|error| format!("cannot configure Git relocation environment: {error}"))?,
    );
    let status = command
        .status()
        .map_err(|error| format!("cannot relocate bundled Git runtime: {error}"))?;
    if !status.success() {
        return Err(format!(
            "bundled Git relocation failed with status {}",
            status
        ));
    }
    Ok(())
}

fn first_existing(root: &Path, candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(|relative| root.join(relative))
        .find(|path| path.is_file())
}

fn first_directory(root: &Path, candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(|relative| root.join(relative))
        .find(|path| path.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    fn test_directory() -> PathBuf {
        std::env::temp_dir().join(format!(
            "terminal-rust-git-runtime-{}-{}",
            std::process::id(),
            NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn development_without_bundled_resources_uses_system_git() {
        let root = test_directory();
        fs::create_dir_all(&root).expect("test directory should be created");

        let runtime = GitRuntime::prepare(&root, &root.join("data"))
            .expect("missing optional archive should not be an error");

        assert!(runtime.is_none());
        fs::remove_dir_all(root).expect("test directory should be removed");
    }

    #[test]
    fn archive_extraction_rejects_paths_outside_the_runtime_directory() {
        let root = test_directory();
        fs::create_dir_all(&root).expect("test directory should be created");
        let archive_path = root.join("malicious.tar.gz");

        let archive_file = File::create(&archive_path).expect("archive should be created");
        let mut encoder =
            flate2::write::GzEncoder::new(archive_file, flate2::Compression::default());
        let contents = b"outside";
        let mut header = tar::Header::new_gnu();
        header.as_mut_bytes()[..14].copy_from_slice(b"../outside.txt");
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        encoder
            .write_all(header.as_bytes())
            .expect("archive header should be written");
        encoder
            .write_all(contents)
            .expect("archive contents should be written");
        encoder
            .write_all(&vec![0; 512 - contents.len()])
            .expect("archive block should be padded");
        encoder
            .write_all(&[0; 1024])
            .expect("archive terminator should be written");
        encoder
            .finish()
            .expect("compressed archive should be finalized");

        let destination = root.join("runtime");
        fs::create_dir_all(&destination).expect("runtime directory should be created");
        let result = extract_runtime(&archive_path, &destination);

        assert!(result.is_err());
        assert!(!root.join("outside.txt").exists());
        fs::remove_dir_all(root).expect("test directory should be removed");
    }

    #[test]
    fn git_runtime_discovery_requires_helper_programs() {
        let root = test_directory();
        let git_path = if cfg!(windows) {
            root.join("Library/bin/git.exe")
        } else {
            root.join("bin/git")
        };
        fs::create_dir_all(git_path.parent().expect("Git path has parent"))
            .expect("Git directories should be created");
        File::create(&git_path).expect("Git executable fixture should be created");

        assert!(GitRuntime::discover(&root).is_err());
        fs::remove_dir_all(root).expect("test directory should be removed");
    }

    #[test]
    fn bundled_runtime_relocates_and_runs_git_from_a_path_with_spaces() {
        let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources");
        if !resources.join(ARCHIVE_NAME).is_file() {
            return;
        }

        let root = test_directory();
        let app_data = root.join("app data with spaces");
        let runtime = GitRuntime::prepare(&resources, &app_data)
            .expect("bundled Git should prepare")
            .expect("CI should provide the bundled runtime archive");

        let version = runtime_command(&runtime, &["--version"])
            .output()
            .expect("bundled Git should start");
        assert!(version.status.success());
        assert!(String::from_utf8_lossy(&version.stdout).contains("git version 2.56.0"));

        let helper_path = runtime_command(&runtime, &["--exec-path"])
            .output()
            .expect("Git should report its helper path");
        assert!(helper_path.status.success());
        assert!(
            !String::from_utf8_lossy(&helper_path.stdout)
                .trim()
                .is_empty()
        );

        let repository = app_data.join("repository with spaces");
        fs::create_dir_all(&repository).expect("repository fixture should be created");
        let repository_arg = repository.to_string_lossy();
        let initialized = runtime_command(&runtime, &["init", "--quiet", repository_arg.as_ref()])
            .status()
            .expect("Git should initialize a repository with spaces in its path");
        assert!(initialized.success());

        let help = runtime_command(&runtime, &["--no-pager", "help", "-a"])
            .output()
            .expect("Git's built-in help should start");
        assert!(help.status.success());
        assert!(String::from_utf8_lossy(&help.stdout).contains("add"));

        fs::remove_dir_all(root).expect("test directory should be removed");
    }

    fn runtime_command(runtime: &GitRuntime, args: &[&str]) -> Command {
        let mut command = Command::new(&runtime.executable);
        command.args(args).env("GIT_EXEC_PATH", &runtime.exec_path);
        if let Some(template_dir) = runtime.template_dir.as_ref() {
            command.env("GIT_TEMPLATE_DIR", template_dir);
        }
        if let Some(ca_bundle) = runtime.ca_bundle.as_ref() {
            command.env("GIT_SSL_CAINFO", ca_bundle);
        }
        let current_path = std::env::var_os("PATH").unwrap_or_default();
        let path = runtime
            .path_entries
            .iter()
            .cloned()
            .chain(std::env::split_paths(&current_path));
        command.env(
            "PATH",
            std::env::join_paths(path).expect("runtime paths should form a valid PATH"),
        );
        command
    }
}
