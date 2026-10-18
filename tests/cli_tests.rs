use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the system clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "terminal rust {label} {} {unique}",
            std::process::id()
        ));

        fs::create_dir(&path).expect("the temporary test directory should be created");

        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_cli(current_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_terminal_rust"))
        .args(args)
        .current_dir(current_dir)
        .output()
        .expect("the terminal_rust binary should start")
}

fn run_cli_with_config(current_dir: &Path, args: &[&str], config_path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_terminal_rust"))
        .args(args)
        .current_dir(current_dir)
        .env("TERMINAL_RUST_CONFIG", config_path)
        .output()
        .expect("the terminal_rust binary should start")
}

fn run_cli_with_plugin_data(
    current_dir: &Path,
    args: &[&str],
    config_path: &Path,
    data_root: &Path,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_terminal_rust"));
    command
        .args(args)
        .current_dir(current_dir)
        .env("TERMINAL_RUST_CONFIG", config_path);
    #[cfg(target_os = "windows")]
    command.env("LOCALAPPDATA", data_root);
    #[cfg(target_os = "macos")]
    command.env("HOME", data_root);
    #[cfg(all(unix, not(target_os = "macos")))]
    command.env("XDG_DATA_HOME", data_root);
    command
        .output()
        .expect("the terminal_rust binary should start")
}

fn plugin_root(data_root: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    return data_root.join("terminal-rust").join("plugins");
    #[cfg(target_os = "macos")]
    return data_root
        .join("Library")
        .join("Application Support")
        .join("terminal-rust")
        .join("plugins");
    #[cfg(all(unix, not(target_os = "macos")))]
    return data_root.join("terminal-rust").join("plugins");
}

fn run_interactive(current_dir: &Path, input: &str) -> Output {
    run_interactive_with_env(current_dir, input, None)
}

fn run_interactive_with_env(current_dir: &Path, input: &str, config_path: Option<&Path>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_terminal_rust"))
        .current_dir(current_dir)
        .env("USER", "terminal-rust-test")
        .env("USERNAME", "terminal-rust-test")
        .envs(config_path.map(|path| ("TERMINAL_RUST_CONFIG", path)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the terminal_rust binary should start");

    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(input.as_bytes())
        .expect("interactive input should be written");

    child
        .wait_with_output()
        .expect("the terminal_rust binary should finish")
}

#[test]
fn interactive_shell_loads_icon_mode_from_toml() {
    let directory = TestDirectory::new("configured prompt");
    let config = directory.path().join("config.toml");
    fs::write(&config, "version = 1\n[prompt]\nicons = \"always\"\n")
        .expect("configuration should be written");

    let output = run_interactive_with_env(directory.path(), "exit\n", Some(&config));

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let rendered = stdout(&output);
    assert!(
        rendered.contains('\u{f007}'),
        "user icon missing: {rendered}"
    );
    assert!(
        rendered.contains('\u{f07b}'),
        "path icon missing: {rendered}"
    );
}

#[test]
fn interactive_shell_follows_configured_segment_order() {
    let directory = TestDirectory::new("ordered prompt");
    let config = directory.path().join("config.toml");
    fs::write(
        &config,
        "version = 1\n[prompt]\nsegments = [\"symbol\", \"path\"]\nseparator = \" | \"\n",
    )
    .expect("configuration should be written");

    let output = run_interactive_with_env(directory.path(), "exit\n", Some(&config));
    let rendered = stdout(&output);
    let rendered_path = rendered
        .strip_prefix("> | ")
        .and_then(|value| value.strip_suffix(' '))
        .expect("configured prompt should follow symbol then path order");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        fs::canonicalize(rendered_path).expect("rendered path should exist"),
        fs::canonicalize(directory.path()).expect("fixture path should exist")
    );
}

#[test]
fn powerline_theme_uses_ascii_separator_without_terminal_font_capability() {
    let directory = TestDirectory::new("powerline fallback");
    let config = directory.path().join("config.toml");
    fs::write(&config, "version = 1\ntheme = \"powerline\"\n")
        .expect("configuration should be written");

    let output = run_interactive_with_env(directory.path(), "exit\n", Some(&config));
    let rendered = stdout(&output);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(
        rendered.ends_with(" | > "),
        "powerline should use its ASCII fallback: {rendered}"
    );
    assert!(
        rendered.contains(&directory.path().display().to_string()),
        "prompt should contain the current path: {rendered}"
    );
}

#[test]
fn invalid_toml_reports_its_path_and_uses_defaults() {
    let directory = TestDirectory::new("invalid config");
    let config = directory.path().join("broken config.toml");
    fs::write(&config, "version = [").expect("invalid configuration should be written");

    let output = run_interactive_with_env(directory.path(), "exit\n", Some(&config));
    let error = stderr(&output);

    assert!(output.status.success());
    assert!(error.contains("broken config.toml"), "stderr: {error}");
    assert!(error.contains("invalid TOML"), "stderr: {error}");
    assert!(error.contains("using defaults"), "stderr: {error}");
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be valid UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be valid UTF-8")
}

#[test]
fn pwd_prints_the_process_working_directory() {
    let directory = TestDirectory::new("pwd");
    let output = run_cli(directory.path(), &["pwd"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(output.stderr.is_empty());

    let reported_path = PathBuf::from(stdout(&output).trim());
    assert_eq!(
        fs::canonicalize(reported_path).expect("the reported directory should exist"),
        fs::canonicalize(directory.path()).expect("the test directory should exist")
    );
}

#[test]
fn ls_lists_files_and_directories_from_the_working_directory() {
    let directory = TestDirectory::new("ls");
    fs::write(directory.path().join("alpha.txt"), "alpha")
        .expect("the fixture file should be created");
    fs::create_dir(directory.path().join("nested directory"))
        .expect("the fixture directory should be created");

    let output = run_cli(directory.path(), &["ls"]);
    let lines: Vec<String> = stdout(&output).lines().map(ToOwned::to_owned).collect();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(output.stderr.is_empty());
    assert!(lines.contains(&"alpha.txt".to_string()));
    assert!(lines.contains(&"nested directory".to_string()));
}

#[test]
fn unknown_command_returns_a_failure_and_writes_to_stderr() {
    let directory = TestDirectory::new("unknown command");
    let output = run_cli(directory.path(), &["does-not-exist"]);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(stderr(&output).trim(), "unknown command: does-not-exist");
}

#[test]
fn missing_required_argument_returns_a_failure_and_writes_to_stderr() {
    let directory = TestDirectory::new("missing argument");
    let output = run_cli(directory.path(), &["cat"]);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(stderr(&output).trim(), "cat: missing file name");
}

#[test]
fn help_lists_every_public_command() {
    let directory = TestDirectory::new("help");
    let output = run_cli(directory.path(), &["help"]);
    let rendered = stdout(&output);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(output.stderr.is_empty());

    for command in [
        "pwd",
        "cd",
        "ls",
        "mkdir",
        "touch",
        "cat",
        "rm",
        "clear",
        "dashboard",
        "git",
        "theme",
        "plugins",
        "plugin",
        "commands",
        "help",
        "exit",
        "quit",
    ] {
        assert!(
            rendered
                .lines()
                .any(|line| line.trim_start().starts_with(command)),
            "help should list {command}:\n{rendered}"
        );
    }
}

#[test]
fn git_help_explains_supported_commands_and_read_only_limits() {
    let directory = TestDirectory::new("git help");
    let output = run_cli(directory.path(), &["git", "help"]);
    let rendered = stdout(&output);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(output.stderr.is_empty());
    assert!(rendered.contains("git status [options]"));
    assert!(rendered.contains("git branch [options]"));
    assert!(rendered.contains("git log [options]"));
    assert!(rendered.contains("git log -5 --oneline"));
    assert!(rendered.contains("push, reset and clean"));
}

#[test]
fn plugin_alias_and_help_explain_the_available_plugin_actions() {
    let directory = TestDirectory::new("plugin help");

    let plural = run_cli(directory.path(), &["plugins", "help"]);
    let singular = run_cli(directory.path(), &["plugin", "help"]);

    assert!(plural.status.success(), "stderr: {}", stderr(&plural));
    assert!(singular.status.success(), "stderr: {}", stderr(&singular));
    assert_eq!(stdout(&plural), stdout(&singular));
    assert!(stdout(&plural).contains("plugins list"));
    assert!(stdout(&plural).contains("requested permissions"));
    assert!(stdout(&plural).contains("declarative"));
}

#[test]
fn commands_help_explains_custom_commands_and_listing() {
    let directory = TestDirectory::new("commands help");
    let output = run_cli(directory.path(), &["commands", "help"]);
    let rendered = stdout(&output);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(rendered.contains("shortcuts configured"));
    assert!(rendered.contains("commands list"));
    assert!(rendered.contains("docs/custom-commands.md"));
}

#[test]
fn configured_builtin_command_executes_and_is_listed() {
    let directory = TestDirectory::new("custom command");
    let config = directory.path().join("config.toml");
    fs::write(
        &config,
        r#"version = 1
[commands.assistance]
kind = "builtin"
target = "help"
append_args = false
description = "Show shell help"
"#,
    )
    .expect("configuration should be written");

    let executed = run_cli_with_config(directory.path(), &["assistance"], &config);
    assert!(executed.status.success(), "stderr: {}", stderr(&executed));
    assert!(stdout(&executed).contains("Available commands:"));

    let listed = run_cli_with_config(directory.path(), &["commands", "list"], &config);
    assert!(listed.status.success(), "stderr: {}", stderr(&listed));
    assert!(stdout(&listed).contains("assistance"));
    assert!(stdout(&listed).contains("[builtin] help"));
}

#[test]
fn configured_process_command_executes_without_a_system_shell() {
    let directory = TestDirectory::new("custom process command");
    let config = directory.path().join("config.toml");
    fs::write(
        &config,
        format!(
            "version = 1\n[commands.self-help]\nkind = \"process\"\ntarget = {:?}\nargs = [\"help\"]\nappend_args = false\n",
            env!("CARGO_BIN_EXE_terminal_rust")
        ),
    )
    .expect("configuration should be written");

    let output = run_cli_with_config(directory.path(), &["self-help"], &config);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("Available commands:"));
}

#[test]
fn official_declarative_plugin_executes_and_can_be_disabled() {
    let directory = TestDirectory::new("official plugin");
    let data_root = directory.path().join("plugin data");
    let plugin = plugin_root(&data_root).join("example.hello").join("0.1.0");
    fs::create_dir_all(&plugin).expect("plugin directory should be created");
    fs::write(
        plugin.join("terminal-rust-plugin.toml"),
        r#"schema_version = 1
id = "example.hello"
name = "Hello plugin"
version = "0.1.0"
description = "Official declarative plugin"
host_api = "^1.0"
terminal_version = ">=0.3, <2.0"
permissions = ["command.register", "output.write"]

[[commands]]
name = "hello"
summary = "Print the official plugin greeting"
response = "Hello from the official Terminal Rust plugin!\n"
"#,
    )
    .expect("plugin manifest should be written");
    let config = directory.path().join("config.toml");
    fs::write(
        &config,
        r#"version = 1
[plugins]
enabled = ["example.hello"]
[plugins.grants]
"example.hello" = ["command.register", "output.write"]
"#,
    )
    .expect("plugin configuration should be written");

    let executed = run_cli_with_plugin_data(directory.path(), &["hello"], &config, &data_root);

    assert!(executed.status.success(), "stderr: {}", stderr(&executed));
    assert_eq!(
        stdout(&executed),
        "Hello from the official Terminal Rust plugin!\n"
    );

    fs::write(
        &config,
        r#"version = 1
[plugins]
enabled = []
[plugins.grants]
"example.hello" = ["command.register", "output.write"]
"#,
    )
    .expect("disabled plugin configuration should be written");
    let disabled = run_cli_with_plugin_data(directory.path(), &["hello"], &config, &data_root);
    let native = run_cli_with_plugin_data(directory.path(), &["pwd"], &config, &data_root);

    assert!(!disabled.status.success());
    assert_eq!(stderr(&disabled).trim(), "unknown command: hello");
    assert!(native.status.success(), "stderr: {}", stderr(&native));
}

#[test]
fn theme_list_and_preview_are_available_without_starting_the_shell() {
    let directory = TestDirectory::new("theme commands");

    let list = run_cli(directory.path(), &["theme", "list"]);
    let listed = stdout(&list);
    assert!(list.status.success(), "stderr: {}", stderr(&list));
    for name in ["default", "minimal", "git-focus", "powerline"] {
        assert!(listed.contains(name), "theme list should include {name}");
    }

    let preview = run_cli(directory.path(), &["theme", "preview", "minimal"]);
    assert!(preview.status.success(), "stderr: {}", stderr(&preview));
    let preview = stdout(&preview);
    assert!(preview.starts_with("Theme: minimal\n\n"));
    assert!(preview.contains("terminal-rust  git:main*  > ls\n"));
    assert!(preview.contains("git status --short"));
    assert!(preview.contains("cargo test --lib"));
    assert!(preview.contains("cat: missing.txt: No such file or directory"));
    assert!(!preview.contains("\u{1b}["));
}

#[test]
fn theme_preview_reports_an_unknown_preset() {
    let directory = TestDirectory::new("unknown theme");
    let output = run_cli(directory.path(), &["theme", "preview", "missing"]);
    let error = stderr(&output);

    assert!(!output.status.success());
    assert!(error.contains("unknown theme 'missing'"), "stderr: {error}");
    assert!(error.contains("default, minimal, git-focus, powerline"));
}

#[test]
fn file_commands_complete_a_cross_platform_workflow() {
    let directory = TestDirectory::new("file workflow");

    let mkdir = run_cli(directory.path(), &["mkdir", "nested directory"]);
    assert!(mkdir.status.success(), "stderr: {}", stderr(&mkdir));

    let file = directory.path().join("nested directory").join("notes.txt");
    let file_arg = file.to_string_lossy().into_owned();
    let touch = run_cli(directory.path(), &["touch", &file_arg]);
    assert!(touch.status.success(), "stderr: {}", stderr(&touch));
    assert!(file.is_file());

    fs::write(&file, "portable content\n").expect("the fixture file should be writable");
    let cat = run_cli(directory.path(), &["cat", &file_arg]);
    assert!(cat.status.success(), "stderr: {}", stderr(&cat));
    assert_eq!(stdout(&cat), "portable content\n");

    let remove_file = run_cli(directory.path(), &["rm", &file_arg]);
    assert!(
        remove_file.status.success(),
        "stderr: {}",
        stderr(&remove_file)
    );
    assert!(!file.exists());

    let directory_arg = directory.path().join("nested directory");
    let directory_arg = directory_arg.to_string_lossy().into_owned();
    let remove_directory = run_cli(directory.path(), &["rm", "-d", &directory_arg]);
    assert!(
        remove_directory.status.success(),
        "stderr: {}",
        stderr(&remove_directory)
    );
    assert!(!Path::new(&directory_arg).exists());
}

#[test]
fn interactive_rm_force_returns_to_the_prompt_without_consuming_the_next_command() {
    let directory = TestDirectory::new("rm force prompt");
    let target = directory.path().join("tree to remove");
    fs::create_dir(&target).expect("target directory should be created");
    fs::write(target.join("file.txt"), "content").expect("target file should be created");

    let input = format!("rm -rf \"{}\"\npwd\nexit\n", target.display());
    let output = run_interactive(directory.path(), &input);
    let rendered = stdout(&output);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(rendered.contains(&directory.path().display().to_string()));
    assert!(
        !target.exists(),
        "rm -rf should remove the target without prompting"
    );
}

#[test]
fn dashboard_renders_important_sections_without_a_terminal() {
    let directory = TestDirectory::new("dashboard");
    let output = run_cli(directory.path(), &["dashboard"]);
    let rendered = stdout(&output);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(output.stderr.is_empty());
    for section in ["SYSTEM", "PERFORMANCE", "STORAGE & NETWORK", "CONTEXT"] {
        assert!(
            rendered.contains(section),
            "dashboard should include {section}"
        );
    }
    assert!(
        !rendered.contains('\u{1b}'),
        "piped output should not use ANSI escapes"
    );
}

#[test]
fn interactive_prompt_stays_simple_outside_git() {
    let directory = TestDirectory::new("prompt outside git");
    let output = run_interactive(directory.path(), "exit\n");
    let rendered = stdout(&output);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(
        rendered.ends_with("  > "),
        "prompt should end with its symbol"
    );
    assert!(
        rendered.contains(&directory.path().display().to_string()),
        "prompt should contain the current path: {rendered}"
    );
}

#[test]
fn interactive_prompt_shows_a_dirty_git_branch() {
    let directory = TestDirectory::new("git prompt");
    let git_available = Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());

    if !git_available {
        return;
    }

    let initialized = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(directory.path())
        .status()
        .expect("git should start");
    assert!(initialized.success());

    let named_branch = Command::new("git")
        .args(["symbolic-ref", "HEAD", "refs/heads/main"])
        .current_dir(directory.path())
        .status()
        .expect("git should name the initial branch");
    assert!(named_branch.success());

    fs::write(directory.path().join("dirty file.txt"), "dirty")
        .expect("the dirty fixture should be created");

    let output = run_interactive(directory.path(), "exit\n");
    let rendered = stdout(&output);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(
        rendered.ends_with("  git:main*  > "),
        "the Git prompt should contain its branch and dirty marker: {rendered}"
    );
    assert!(
        rendered.contains(&directory.path().display().to_string()),
        "prompt should contain the current path: {rendered}"
    );
}

#[test]
fn git_read_only_helpers_work_from_a_path_with_spaces_and_unicode() {
    let directory = TestDirectory::new("git repo with spaces été");
    let git_available = Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());

    if !git_available {
        return;
    }

    let initialized = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(directory.path())
        .status()
        .expect("git should start");
    assert!(initialized.success());

    let named_branch = Command::new("git")
        .args(["symbolic-ref", "HEAD", "refs/heads/main"])
        .current_dir(directory.path())
        .status()
        .expect("git should name the initial branch");
    assert!(named_branch.success());

    for (key, value) in [
        ("user.name", "Terminal Rust test"),
        ("user.email", "terminal-rust-test@example.invalid"),
    ] {
        let configured = Command::new("git")
            .args(["config", key, value])
            .current_dir(directory.path())
            .status()
            .expect("git config should start");
        assert!(configured.success());
    }

    fs::write(directory.path().join("tracked file.txt"), "fixture")
        .expect("the tracked fixture should be written");
    let added = Command::new("git")
        .args(["add", "--", "tracked file.txt"])
        .current_dir(directory.path())
        .status()
        .expect("git add should start");
    assert!(added.success());
    let committed = Command::new("git")
        .args(["commit", "--quiet", "-m", "cross-platform fixture"])
        .current_dir(directory.path())
        .status()
        .expect("git commit should start");
    assert!(committed.success());

    let status = run_cli(directory.path(), &["git", "status", "--short"]);
    assert!(status.status.success(), "stderr: {}", stderr(&status));
    assert!(stdout(&status).is_empty(), "working tree should be clean");

    let branch = run_cli(directory.path(), &["git", "branch", "--show-current"]);
    assert!(branch.status.success(), "stderr: {}", stderr(&branch));
    assert_eq!(stdout(&branch).trim(), "main");

    let log = run_cli(directory.path(), &["git", "log", "-1", "--oneline"]);
    assert!(log.status.success(), "stderr: {}", stderr(&log));
    assert!(stdout(&log).contains("cross-platform fixture"));
}
