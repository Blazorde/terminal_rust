use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

/// An empty home for the binary under test. Without it the shell reads the
/// developer's own configuration (prompt, aliases, plugins), and these tests then
/// pass or fail depending on whose machine they run on.
fn isolated_home() -> &'static Path {
    static HOME: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    HOME.get_or_init(|| {
        let home = std::env::temp_dir().join("terminal-rust-tests-isolated-home");
        fs::create_dir_all(&home).expect("the isolated home should be created");
        home
    })
}

/// The binary under test, started with no access to the user's configuration.
/// A test that needs a configuration passes it explicitly afterwards.
fn shell_command() -> Command {
    let home = isolated_home();
    let mut command = Command::new(env!("CARGO_BIN_EXE_terminal_rust"));
    command
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .env("APPDATA", home.join("appdata"))
        .env("LOCALAPPDATA", home.join("localappdata"))
        .env_remove("TERMINAL_RUST_CONFIG");
    command
}

fn run_cli(current_dir: &Path, args: &[&str]) -> Output {
    shell_command()
        .args(args)
        .current_dir(current_dir)
        .output()
        .expect("the terminal_rust binary should start")
}

fn run_cli_with_config(current_dir: &Path, args: &[&str], config_path: &Path) -> Output {
    shell_command()
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
    let mut command = shell_command();
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
    let mut child = shell_command()
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
fn git_help_is_forwarded_to_native_git() {
    let directory = TestDirectory::new("git help");
    let output = run_cli(directory.path(), &["git", "help", "-a"]);
    let rendered = stdout(&output);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(
        rendered.contains("status"),
        "native Git help output: {rendered}"
    );
    assert!(
        rendered.contains("commit"),
        "native Git help output: {rendered}"
    );
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
fn echo_prints_the_given_text() {
    let directory = TestDirectory::new("echo");
    let output = run_cli(directory.path(), &["echo", "Hello, World!"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(output.stderr.is_empty());

    let binding = stdout(&output);
    let reported_text = binding.trim();
    assert_eq!(reported_text, "Hello, World!");
}

#[test]
fn native_git_commands_work_from_a_path_with_spaces_and_unicode() {
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

    let create_branch = run_cli(
        directory.path(),
        &["git", "branch", "native-pass-through-test"],
    );
    assert!(
        create_branch.status.success(),
        "stderr: {}",
        stderr(&create_branch)
    );

    let delete_branch = run_cli(
        directory.path(),
        &["git", "branch", "--delete", "native-pass-through-test"],
    );
    assert!(
        delete_branch.status.success(),
        "stderr: {}",
        stderr(&delete_branch)
    );
}

#[test]
fn a_name_that_is_not_built_in_runs_the_external_program() {
    let directory = TestDirectory::new("external program");

    let output = run_cli(directory.path(), &["rustc", "--version"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).starts_with("rustc "), "{}", stdout(&output));
}

#[test]
fn a_failing_external_program_fails_the_shell_with_its_status() {
    let directory = TestDirectory::new("external failure");

    let output = run_cli(
        directory.path(),
        &["rustc", "--terminal-rust-no-such-option"],
    );

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("rustc: exited with status"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_missing_program_still_reports_an_unknown_command() {
    let directory = TestDirectory::new("external missing");

    let output = run_cli(directory.path(), &["terminal-rust-no-such-program"]);

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("unknown command: terminal-rust-no-such-program"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn an_exported_variable_reaches_the_programs_the_shell_starts() {
    let directory = TestDirectory::new("exported variable");
    let shell = env!("CARGO_BIN_EXE_terminal_rust");
    let input = format!(
        "export TRS_IT_EXPORTED=visible\nTRS_IT_LOCAL=hidden\n\"{shell}\" printenv TRS_IT_EXPORTED\n\"{shell}\" printenv TRS_IT_LOCAL\nexit\n"
    );

    let output = run_interactive(directory.path(), &input);

    let printed = stdout(&output);
    assert!(printed.contains("visible"), "{printed}");
    assert!(!printed.contains("hidden"), "{printed}");
}

#[test]
fn variables_and_tilde_are_expanded_in_an_interactive_session() {
    let directory = TestDirectory::new("expansion");
    let input =
        "TRS_IT_WORD=expanded\necho $TRS_IT_WORD ${TRS_IT_WORD}s\necho '$TRS_IT_WORD'\nexit\n";

    let output = run_interactive(directory.path(), input);

    let printed = stdout(&output);
    assert!(printed.contains("expanded expandeds"), "{printed}");
    assert!(printed.contains("$TRS_IT_WORD"), "{printed}");
}

#[test]
fn output_stays_in_order_when_built_ins_and_a_program_are_chained() {
    let directory = TestDirectory::new("chain order");
    let input = "echo before-program ; rustc --version ; echo after-program\nexit\n";

    let output = run_interactive(directory.path(), input);

    let printed = stdout(&output);
    let before = printed.find("before-program").expect("first output");
    let program = printed.find("rustc 1.").expect("program output");
    let after = printed.find("after-program").expect("last output");
    assert!(before < program && program < after, "{printed}");
}

#[test]
fn a_failure_stops_an_and_chain_and_runs_an_or_chain() {
    let directory = TestDirectory::new("chain status");
    let input = "terminal-rust-no-such-program && echo should-not-print\nterminal-rust-no-such-program || echo recovered\nexit\n";

    let output = run_interactive(directory.path(), input);

    let printed = stdout(&output);
    assert!(!printed.contains("should-not-print"), "{printed}");
    assert!(printed.contains("recovered"), "{printed}");
}

#[test]
fn a_background_job_cannot_follow_an_and_or_list() {
    let directory = TestDirectory::new("background refused");

    let output = run_interactive(directory.path(), "echo hello && echo world &\nexit\n");

    assert!(
        stderr(&output).contains("a background job cannot follow && or ||"),
        "{}",
        stderr(&output)
    );
    assert!(!stdout(&output).contains("hello"), "{}", stdout(&output));
}

fn shell_lines(directory: &TestDirectory, lines: &[&str]) -> (String, String) {
    let mut input = lines.join("\n");
    input.push_str("\nexit\n");
    let output = run_interactive(directory.path(), &input);
    (stdout(&output), stderr(&output))
}

#[test]
fn redirections_write_append_and_read_files() {
    let directory = TestDirectory::new("redirect files");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo first > f.txt",
            "echo second >> f.txt",
            "cat f.txt",
            "echo replaced > f.txt",
            "cat < f.txt",
        ],
    );

    assert!(printed.contains("first\nsecond\n"), "{printed}\n{errors}");
    assert!(printed.contains("replaced\n"), "{printed}");
    assert_eq!(
        fs::read_to_string(directory.path().join("f.txt")).expect("the file should exist"),
        "replaced\n"
    );
}

#[test]
fn a_redirection_target_can_hold_spaces_and_a_variable() {
    let directory = TestDirectory::new("redirect target");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "NAME=report",
            "echo quoted > \"my file.txt\"",
            "echo variable > $NAME.txt",
            "cat \"my file.txt\"",
            "cat report.txt",
        ],
    );

    assert!(printed.contains("quoted\n"), "{printed}\n{errors}");
    assert!(printed.contains("variable\n"), "{printed}\n{errors}");
}

#[test]
fn a_built_in_can_feed_another_through_a_pipe() {
    let directory = TestDirectory::new("builtin pipe");

    let (printed, errors) = shell_lines(&directory, &["echo piped | cat | cat"]);

    assert!(printed.contains("piped\n"), "{printed}\n{errors}");
}

#[test]
fn standard_error_can_go_to_a_file_or_join_the_pipe() {
    let directory = TestDirectory::new("redirect stderr");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "ls missing-entry 2> err.txt",
            "cat err.txt",
            "ls missing-entry 2>&1 | cat",
        ],
    );

    let logged = fs::read_to_string(directory.path().join("err.txt")).expect("log should exist");
    assert!(logged.contains("missing-entry"), "{logged}");
    // The error is in the file and in the pipe, not on the terminal's stderr.
    assert!(!errors.contains("missing-entry"), "{errors}");
    assert_eq!(printed.matches("missing-entry").count(), 2, "{printed}");
}

#[test]
fn a_missing_input_file_fails_the_stage_without_running_it() {
    let directory = TestDirectory::new("missing input");

    let (printed, errors) = shell_lines(&directory, &["cat < nowhere.txt", "echo $?"]);

    assert!(errors.contains("cannot open 'nowhere.txt'"), "{errors}");
    assert!(printed.contains("1\n"), "{printed}");
}

#[test]
fn a_pipeline_with_a_syntax_error_runs_nothing() {
    let directory = TestDirectory::new("pipeline syntax");

    let (printed, errors) = shell_lines(&directory, &["echo before ; echo a |", "echo after > "]);

    assert!(!printed.contains("before"), "{printed}");
    assert!(errors.contains("expected a command after '|'"), "{errors}");
    assert!(
        errors.contains("expected a file name after '>'"),
        "{errors}"
    );
}

#[test]
fn quoted_operators_are_ordinary_text() {
    let directory = TestDirectory::new("quoted operators");

    let (printed, _) = shell_lines(&directory, &["echo \"a | b > c\""]);

    assert!(printed.contains("a | b > c\n"), "{printed}");
    assert!(!directory.path().join("c").exists());
}

#[cfg(unix)]
#[test]
fn programs_are_connected_with_real_pipes() {
    let directory = TestDirectory::new("program pipes");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo hello | tr a-z A-Z",
            "printf 'b\\na\\nc\\n' | sort | head -n 2",
            // Bounded on purpose: an endless producer would fill the memory of
            // the test if the pipe were not wired.
            "seq 1 1000 | head -n 2",
        ],
    );

    assert!(printed.contains("HELLO\n"), "{printed}\n{errors}");
    assert!(printed.contains("a\nb\n"), "{printed}\n{errors}");
    assert!(!printed.contains("c\n"), "{printed}");
    assert!(printed.contains("1\n2\n"), "{printed}");
    assert!(!printed.contains("3\n"), "{printed}");
}

#[cfg(unix)]
#[test]
fn a_program_can_read_a_file_and_write_two_streams() {
    let directory = TestDirectory::new("program redirects");
    fs::write(directory.path().join("names.txt"), "b\na\n").expect("input should be written");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "sort < names.txt > sorted.txt",
            "cat sorted.txt",
            "sh -c 'echo out; echo err >&2' > both.txt 2>&1",
            "cat both.txt",
            "sh -c 'echo only-err >&2' 2> e.txt",
            "cat e.txt",
        ],
    );

    assert!(printed.contains("a\nb\n"), "{printed}\n{errors}");
    assert!(printed.contains("out\nerr\n"), "{printed}\n{errors}");
    assert!(printed.contains("only-err\n"), "{printed}\n{errors}");
    assert!(!errors.contains("only-err"), "{errors}");
}

#[cfg(unix)]
#[test]
fn the_status_of_a_pipeline_is_the_status_of_its_last_stage() {
    let directory = TestDirectory::new("pipeline status");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "sh -c 'exit 3' | cat",
            "echo first=$?",
            "echo x | sh -c 'exit 4'",
            "echo second=$?",
            // Two programs: only the last one decides.
            "sh -c 'exit 3' | sh -c 'cat > /dev/null'",
            "echo third=$?",
        ],
    );

    assert!(printed.contains("first=0\n"), "{printed}\n{errors}");
    assert!(printed.contains("second=4\n"), "{printed}\n{errors}");
    assert!(printed.contains("third=0\n"), "{printed}\n{errors}");
    assert!(errors.contains("exited with status 4"), "{errors}");
}

#[cfg(unix)]
#[test]
fn a_missing_program_in_a_pipeline_fails_only_its_own_stage() {
    let directory = TestDirectory::new("missing program");

    let (printed, errors) = shell_lines(
        &directory,
        &["terminal-rust-no-such-program | cat", "echo still-here"],
    );

    assert!(
        errors.contains("unknown command: terminal-rust-no-such-program"),
        "{errors}"
    );
    assert!(printed.contains("still-here\n"), "{printed}");
}

#[cfg(unix)]
#[test]
fn output_stays_in_order_around_a_pipeline() {
    let directory = TestDirectory::new("pipeline order");

    let (printed, errors) = shell_lines(
        &directory,
        &["echo one", "echo two | tr a-z A-Z", "echo three"],
    );

    let one = printed.find("one").expect("first line");
    let two = printed.find("TWO").expect("second line");
    let three = printed.find("three").expect("third line");
    assert!(one < two && two < three, "{printed}\n{errors}");
}

/// Run the interactive shell on a piped stdin and give up after `limit`, so a
/// shell that blocks fails the test instead of hanging it.
fn run_interactive_with_timeout(
    current_dir: &Path,
    input: &str,
    limit: std::time::Duration,
) -> Option<Output> {
    let mut child = shell_command()
        .current_dir(current_dir)
        .env("USER", "terminal-rust-test")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the terminal_rust binary should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input.as_bytes())
        .expect("interactive input should be written");

    let deadline = std::time::Instant::now() + limit;
    loop {
        if child
            .try_wait()
            .expect("the shell status should be readable")
            .is_some()
        {
            return Some(
                child
                    .wait_with_output()
                    .expect("the shell output should be readable"),
            );
        }
        if std::time::Instant::now() >= deadline {
            child.kill().expect("a blocked shell should be stopped");
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn rm_reads_its_confirmation_from_a_piped_stdin_without_blocking() {
    let directory = TestDirectory::new("rm confirmation");
    fs::create_dir(directory.path().join("doomed")).expect("fixture directory should be created");

    let output = run_interactive_with_timeout(
        directory.path(),
        "rm -r doomed\ny\nexit\n",
        std::time::Duration::from_secs(15),
    );

    assert!(
        output.is_some(),
        "the shell blocked while rm waited for its confirmation"
    );
    assert!(!directory.path().join("doomed").exists());
}

#[test]
fn rm_refuses_dot_and_dot_dot_and_removes_nothing() {
    let directory = TestDirectory::new("rm protected");
    let inner = directory.path().join("inner");
    fs::create_dir(&inner).expect("fixture directory should be created");
    fs::write(inner.join("precious.txt"), "keep").expect("fixture file should be written");

    let output = run_interactive_with_timeout(
        &inner,
        "rm -rf .\nrm -rf ..\nrm -rf ./\nexit\n",
        std::time::Duration::from_secs(15),
    )
    .expect("the shell should answer");

    assert!(inner.join("precious.txt").exists());
    let errors = stderr(&output);
    assert!(errors.contains("refusing to remove '.'"), "{errors}");
    assert!(errors.contains("refusing to remove '..'"), "{errors}");
}

#[cfg(unix)]
#[test]
fn a_failing_process_command_names_its_exit_status() {
    let directory = TestDirectory::new("process status");
    let config = directory.path().join("config.toml");
    fs::write(
        &config,
        "version = 1\n[commands.fail]\nkind = \"process\"\ntarget = \"sh\"\nargs = [\"-c\", \"exit 3\"]\nappend_args = false\n",
    )
    .expect("configuration should be written");

    let output = run_cli_with_config(directory.path(), &["fail"], &config);

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("fail: exited with status 3"),
        "{}",
        stderr(&output)
    );
}

// A directory name can hold an escape sequence. Shown as it is, this one would set
// the window title; every display of a name must neutralize it.
#[cfg(unix)]
const HOSTILE_NAME: &str = "evil\u{1b}]0;PWNED\u{7}name";

#[cfg(unix)]
#[test]
fn ls_neutralizes_control_characters_in_file_names() {
    let directory = TestDirectory::new("ls hostile");
    fs::create_dir(directory.path().join(HOSTILE_NAME)).expect("hostile directory should exist");
    fs::write(directory.path().join("plain.txt"), "x").expect("plain file should exist");

    let output = run_cli(directory.path(), &["ls"]);
    let text = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(text.contains("evil?]0;PWNED?name"), "{text:?}");
    assert!(text.contains("plain.txt"), "{text:?}");
    assert!(
        !text.contains('\u{1b}') && !text.contains('\u{7}'),
        "{text:?}"
    );
}

#[cfg(unix)]
#[test]
fn pwd_neutralizes_control_characters_in_the_current_directory() {
    let directory = TestDirectory::new("pwd hostile");
    let hostile = directory.path().join(HOSTILE_NAME);
    fs::create_dir(&hostile).expect("hostile directory should exist");

    let output = run_cli(&hostile, &["pwd"]);
    let text = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(text.contains("evil?]0;PWNED?name"), "{text:?}");
    assert!(
        !text.contains('\u{1b}') && !text.contains('\u{7}'),
        "{text:?}"
    );
}

// Another terminal can delete the directory the shell is in. The prompt used to
// fail and the shell exited, so the user could not even `cd` out of it.
#[cfg(unix)]
#[test]
fn the_shell_survives_the_deletion_of_its_current_directory() {
    let directory = TestDirectory::new("deleted cwd");
    let gone = directory.path().join("gone");
    let safe = directory.path().join("safe");
    fs::create_dir(&gone).expect("directory to delete should exist");
    fs::create_dir(&safe).expect("destination should exist");
    let input = format!(
        "rmdir \"{}\"\ncd \"{}\"\npwd\necho still-alive\nexit\n",
        gone.display(),
        safe.display()
    );

    let output = run_interactive(&gone, &input);
    let text = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(text.contains("still-alive"), "{text:?}");
    assert!(
        text.contains(&format!("{}\n", safe.display())),
        "pwd should print the new directory: {text:?}"
    );
    assert!(!stderr(&output).contains("prompt:"), "{}", stderr(&output));
}

fn glob_fixture(directory: &TestDirectory) {
    for file in ["a.txt", "b.txt", "c.md", ".hidden.txt", "[draft] one.md"] {
        fs::write(directory.path().join(file), "").expect("fixture file");
    }
    fs::create_dir(directory.path().join("sub")).expect("fixture directory");
    fs::write(directory.path().join("sub").join("d.txt"), "").expect("fixture file");
}

#[test]
fn a_pattern_expands_to_the_sorted_matching_names() {
    let directory = TestDirectory::new("glob expand");
    glob_fixture(&directory);

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo *.txt",
            "echo ?.md",
            "echo [ab].txt",
            "echo */d.txt",
            "echo .*.txt",
        ],
    );

    assert!(printed.contains("a.txt b.txt\n"), "{printed}\n{errors}");
    assert!(printed.contains("c.md\n"), "{printed}");
    assert!(printed.contains("sub/d.txt\n"), "{printed}");
    assert!(printed.contains(".hidden.txt\n"), "{printed}");
    // `*.txt` does not list the hidden file.
    assert!(!printed.contains("a.txt b.txt .hidden"), "{printed}");
}

#[test]
fn quoting_or_escaping_keeps_a_pattern_literal() {
    let directory = TestDirectory::new("glob literal");
    glob_fixture(&directory);

    let (printed, errors) = shell_lines(
        &directory,
        &["echo \"*.txt\"", "echo '*.txt'", "echo \\*.txt"],
    );

    assert_eq!(printed.matches("*.txt\n").count(), 3, "{printed}\n{errors}");
    assert!(!printed.contains("a.txt"), "{printed}");
}

#[test]
fn a_pattern_without_a_match_stays_as_written() {
    let directory = TestDirectory::new("glob nomatch");
    glob_fixture(&directory);

    let (printed, _) = shell_lines(&directory, &["echo *.zip missing/*.rs"]);

    assert!(printed.contains("*.zip missing/*.rs\n"), "{printed}");
}

#[test]
fn a_quoted_bracket_in_a_pattern_word_is_not_a_wildcard() {
    let directory = TestDirectory::new("glob quoted bracket");
    glob_fixture(&directory);

    let (printed, errors) = shell_lines(&directory, &["echo \"[draft]\"*.md"]);

    assert!(printed.contains("[draft] one.md\n"), "{printed}\n{errors}");
    assert!(!printed.contains("c.md"), "{printed}");
}

#[test]
fn a_pattern_works_with_commands_and_pipes() {
    let directory = TestDirectory::new("glob commands");
    glob_fixture(&directory);

    let (printed, errors) = shell_lines(
        &directory,
        &["rm *.txt", "echo done", "ls | cat", "echo *.md | cat"],
    );

    assert!(!directory.path().join("a.txt").exists(), "{errors}");
    assert!(!directory.path().join("b.txt").exists());
    assert!(directory.path().join(".hidden.txt").exists());
    assert!(printed.contains("done\n"), "{printed}");
    assert!(printed.contains("[draft] one.md c.md\n"), "{printed}");
}

#[cfg(unix)]
#[test]
fn the_status_is_the_exit_code_of_the_program() {
    let directory = TestDirectory::new("exit codes");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "sh -c 'exit 3'",
            "echo code=$?",
            "sh -c 'exit 0'",
            "echo code=$?",
            "terminal-rust-no-such-program",
            "echo code=$?",
            "cd /terminal-rust-missing-directory",
            "echo code=$?",
            "sh -c 'kill -9 $$'",
            "echo code=$?",
        ],
    );

    let codes: Vec<&str> = printed
        .lines()
        .filter_map(|line| line.rsplit("> ").next())
        .filter(|line| line.starts_with("code="))
        .collect();
    assert_eq!(
        codes,
        ["code=3", "code=0", "code=127", "code=1", "code=137"],
        "{printed}\n{errors}"
    );
}

#[cfg(unix)]
#[test]
fn the_code_of_a_pipeline_is_the_code_of_its_last_program() {
    let directory = TestDirectory::new("pipeline codes");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "sh -c 'exit 3' | sh -c 'cat > /dev/null; exit 4'",
            "echo code=$?",
            "sh -c 'exit 5' | cat",
            "echo code=$?",
            "echo x | sh -c 'exit 6'",
            "echo code=$?",
        ],
    );

    let codes: Vec<&str> = printed
        .lines()
        .filter_map(|line| line.rsplit("> ").next())
        .filter(|line| line.starts_with("code="))
        .collect();
    assert_eq!(codes, ["code=4", "code=0", "code=6"], "{printed}\n{errors}");
}

#[cfg(unix)]
#[test]
fn and_or_chains_still_follow_success_and_failure_with_exact_codes() {
    let directory = TestDirectory::new("chain codes");

    let (printed, _) = shell_lines(
        &directory,
        &[
            "sh -c 'exit 9' && echo wrong || echo recovered",
            "sh -c 'exit 0' && echo ok",
        ],
    );

    assert!(printed.contains("recovered"), "{printed}");
    assert!(!printed.contains("wrong"), "{printed}");
    assert!(printed.contains("ok"), "{printed}");
}

#[test]
fn exit_leaves_with_the_given_code_or_the_last_status() {
    let directory = TestDirectory::new("exit code");

    for (input, expected) in [
        ("exit 3\n", 3),
        ("exit 0\n", 0),
        ("exit 300\n", 44),
        ("exit 200\n", 200),
        ("exit -1\n", 255),
        ("echo fine\nexit\n", 0),
        ("cat\nexit\n", 1),
        ("cat\n", 1),
        ("echo fine\n", 0),
    ] {
        let output = run_interactive(directory.path(), input);

        assert_eq!(
            output.status.code(),
            Some(expected),
            "input {input:?}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn exit_rejects_a_code_that_is_not_a_number_and_keeps_the_shell() {
    let directory = TestDirectory::new("exit invalid");

    let (printed, errors) = shell_lines(&directory, &["exit soon", "echo still-here"]);

    assert!(
        errors.contains("exit: numeric argument required: 'soon'"),
        "{errors}"
    );
    assert!(printed.contains("still-here\n"), "{printed}");
}

#[test]
fn a_variable_prefix_applies_to_one_command_only() {
    let directory = TestDirectory::new("env prefix");
    let shell = env!("CARGO_BIN_EXE_terminal_rust");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            &format!("TRS_PREFIX=temporary \"{shell}\" printenv TRS_PREFIX"),
            "echo after=$TRS_PREFIX",
            &format!("TRS_PREFIX=temporary \"{shell}\" printenv TRS_PREFIX | cat"),
            "export TRS_KEPT=original",
            &format!("TRS_KEPT=changed \"{shell}\" printenv TRS_KEPT"),
            &format!("\"{shell}\" printenv TRS_KEPT"),
            "TRS_PLAIN=assigned",
            "echo plain=$TRS_PLAIN",
        ],
    );

    assert_eq!(
        printed.matches("temporary\n").count(),
        2,
        "{printed}\n{errors}"
    );
    assert!(printed.contains("after=\n"), "{printed}");
    assert!(printed.contains("changed\n"), "{printed}");
    assert!(printed.contains("original\n"), "{printed}");
    // Without a command, the words still assign a shell variable.
    assert!(printed.contains("plain=assigned\n"), "{printed}");
}

#[test]
fn several_prefixed_variables_reach_a_built_in_and_a_program() {
    let directory = TestDirectory::new("env prefixes");
    let shell = env!("CARGO_BIN_EXE_terminal_rust");

    let (printed, errors) = shell_lines(
        &directory,
        &[&format!("TRS_ONE=1 TRS_TWO=2 \"{shell}\" printenv TRS_TWO")],
    );

    assert!(printed.contains("2\n"), "{printed}\n{errors}");
}

#[cfg(unix)]
#[test]
fn a_background_job_gives_the_prompt_back_at_once() {
    let directory = TestDirectory::new("background prompt");
    let started = std::time::Instant::now();

    let output = run_interactive(
        directory.path(),
        "sleep 20 &\necho prompt-is-back\njobs\nkill %1\nexit\n",
    );

    let printed = stdout(&output);
    let errors = stderr(&output);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "waited for the job"
    );
    assert!(errors.contains("[1] "), "{errors}");
    assert!(printed.contains("prompt-is-back\n"), "{printed}");
    assert!(printed.contains("[1]+  Running  sleep 20"), "{printed}");
}

#[cfg(unix)]
#[test]
fn a_finished_job_is_reported_once_before_the_next_prompt() {
    let directory = TestDirectory::new("background notice");

    let (printed, errors) = shell_lines(
        &directory,
        &["sh -c 'exit 3' &", "sleep 0.5", "echo next", "echo again"],
    );

    assert_eq!(printed.matches("Exit 3").count(), 1, "{printed}\n{errors}");
    let notice = printed.find("[1]+  Exit 3").expect("the notice");
    let next = printed.find("next\n").expect("the next command");
    assert!(notice < next, "{printed}");
}

#[cfg(unix)]
#[test]
fn fg_and_wait_return_the_exit_code_of_the_job() {
    let directory = TestDirectory::new("background fg");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "sh -c 'sleep 0.2; exit 4' &",
            "fg",
            "echo fg=$?",
            "sh -c 'sleep 0.2; exit 5' &",
            "wait",
            "echo wait=$?",
            "jobs",
            "echo done-listing",
        ],
    );

    assert!(printed.contains("fg=4\n"), "{printed}\n{errors}");
    assert!(printed.contains("wait=5\n"), "{printed}\n{errors}");
    assert!(!printed.contains("Running"), "{printed}");
    assert!(errors.contains("fg: exited with status 4"), "{errors}");
}

#[cfg(unix)]
#[test]
fn a_background_program_reads_an_empty_input() {
    let directory = TestDirectory::new("background stdin");

    let (_, errors) = shell_lines(
        &directory,
        &[
            "sh -c 'cat > seen.txt; echo finished > finished.txt' &",
            "wait",
        ],
    );

    assert!(errors.contains("[1] "), "{errors}");
    assert_eq!(
        fs::read_to_string(directory.path().join("seen.txt")).expect("cat output"),
        ""
    );
    assert!(directory.path().join("finished.txt").exists());
}

#[cfg(unix)]
#[test]
fn a_background_pipeline_runs_with_its_redirections() {
    let directory = TestDirectory::new("background pipeline");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "printf 'b\\na\\n' | sort > sorted.txt &",
            "wait",
            "cat sorted.txt",
        ],
    );

    assert!(printed.contains("a\nb\n"), "{printed}\n{errors}");
}

#[cfg(unix)]
#[test]
fn kill_stops_a_job_and_an_unknown_job_is_an_error() {
    let directory = TestDirectory::new("background kill");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "fg",
            "sleep 20 &",
            "kill %1",
            "sleep 0.5",
            "echo checked",
            "fg %9",
        ],
    );

    assert!(errors.contains("fg: no current job"), "{errors}");
    assert!(errors.contains("fg: %9: no such job"), "{errors}");
    assert!(
        printed.contains("[1]+  Exit 137  sleep 20"),
        "{printed}\n{errors}"
    );
    assert!(printed.contains("checked\n"), "{printed}");
}

// The shell quotes what you typed and the names it could not find in its error
// messages. A name holding an escape sequence must not reach the terminal there.
#[cfg(unix)]
#[test]
fn error_messages_neutralize_control_characters_in_quoted_names() {
    let directory = TestDirectory::new("hostile error");
    let hostile = "no\u{1b}]0;PWNED\u{7}where";

    let (printed, errors) = shell_lines(
        &directory,
        &[
            &format!("cd \"{hostile}\""),
            &format!("cat \"{hostile}\" | cat"),
            &format!("\"{hostile}\""),
            "echo still-alive",
        ],
    );

    assert!(errors.contains("no?]0;PWNED?where"), "{errors:?}");
    assert!(
        !errors.contains('\u{1b}') && !errors.contains('\u{7}'),
        "{errors:?}"
    );
    assert!(printed.contains("still-alive\n"), "{printed}");
}

fn git_is_installed() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

#[test]
fn the_output_of_git_goes_through_pipes_and_redirections() {
    if !git_is_installed() {
        return;
    }
    let directory = TestDirectory::new("git pipeline");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "git --version | cat",
            "git --version > version.txt",
            "cat version.txt",
            "echo x | git hash-object --stdin",
        ],
    );

    // `git --version` is printed twice: through the pipe and from the file.
    assert_eq!(
        printed.matches("git version").count(),
        2,
        "{printed}\n{errors}"
    );
    let written =
        fs::read_to_string(directory.path().join("version.txt")).expect("redirected output");
    assert!(written.starts_with("git version"), "{written}");
    // The blob id of "x\n": the built-in `echo` fed Git's standard input.
    assert!(
        printed.contains("587be6b4c3f93f93c489c0111bba5596147a26cb"),
        "{printed}\n{errors}"
    );
}

#[test]
fn a_failing_git_in_a_pipeline_sets_its_exit_code() {
    if !git_is_installed() {
        return;
    }
    let directory = TestDirectory::new("git failure");

    let (printed, _) = shell_lines(
        &directory,
        &[
            "git definitely-not-a-git-command 2> err.txt",
            "echo code=$?",
        ],
    );

    assert!(!printed.contains("code=0"), "{printed}");
    let logged = fs::read_to_string(directory.path().join("err.txt")).expect("stderr file");
    assert!(logged.contains("not a git command"), "{logged}");
}

#[test]
fn a_substitution_is_replaced_by_the_output_without_trailing_newlines() {
    let directory = TestDirectory::new("subst basic");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo today is $(echo fine)",
            "echo [$(echo a ; echo)]",
            "echo [$(true)]",
        ],
    );

    assert!(printed.contains("today is fine\n"), "{printed}\n{errors}");
    assert!(printed.contains("[a]\n"), "{printed}");
    assert!(printed.contains("[]\n"), "{printed}");
}

#[test]
fn an_unquoted_result_is_split_and_a_quoted_one_is_not() {
    let directory = TestDirectory::new("subst split");
    fs::write(directory.path().join("a.txt"), "").expect("fixture");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo [$(echo \"a   b\")]",
            "echo \"[$(echo \"a   b\")]\"",
            "NAME=$(echo \"x   y\")",
            "echo \"<$NAME>\"",
            // The result is never matched against file names.
            "echo $(echo '*.txt')",
        ],
    );

    assert!(printed.contains("[a b]\n"), "{printed}\n{errors}");
    assert!(printed.contains("[a   b]\n"), "{printed}");
    assert!(printed.contains("<x   y>\n"), "{printed}");
    assert!(printed.contains("*.txt\n"), "{printed}");
    assert!(!printed.contains("a.txt"), "{printed}");
}

#[test]
fn quoting_and_escaping_keep_a_substitution_literal() {
    let directory = TestDirectory::new("subst literal");

    let (printed, _) = shell_lines(
        &directory,
        &[
            "echo '$(echo no)'",
            "echo \\$(echo no)",
            "echo \"\\$(echo no)\"",
        ],
    );

    assert_eq!(printed.matches("$(echo no)").count(), 3, "{printed}");
}

#[test]
fn a_result_cannot_inject_an_operator() {
    let directory = TestDirectory::new("subst injection");

    let (printed, _) = shell_lines(
        &directory,
        &[
            "echo $(echo 'x; echo pwned > injected.txt')",
            "echo $(echo 'a | b > c')",
        ],
    );

    assert!(
        printed.contains("x; echo pwned > injected.txt\n"),
        "{printed}"
    );
    assert!(printed.contains("a | b > c\n"), "{printed}");
    assert!(!directory.path().join("injected.txt").exists());
    assert!(!directory.path().join("c").exists());
}

#[test]
fn substitutions_nest_and_hold_whole_command_lines() {
    let directory = TestDirectory::new("subst nesting");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo nested $(echo $(echo deep))",
            "echo chain $(echo a ; echo b && echo c)",
            "echo pipe $(echo hello | cat)",
            "echo \"quoted $(echo \"inner ) paren\")\"",
        ],
    );

    assert!(printed.contains("nested deep\n"), "{printed}\n{errors}");
    assert!(printed.contains("chain a b c\n"), "{printed}");
    assert!(printed.contains("pipe hello\n"), "{printed}");
    assert!(printed.contains("quoted inner ) paren\n"), "{printed}");
}

#[test]
fn a_substitution_works_in_a_redirection_target_and_for_cd() {
    let directory = TestDirectory::new("subst targets");
    fs::create_dir(directory.path().join("sub")).expect("fixture");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo written > $(echo result.txt)",
            "cat result.txt",
            "cd $(echo sub)",
            "pwd",
        ],
    );

    assert!(printed.contains("written\n"), "{printed}\n{errors}");
    assert!(printed.contains("/sub\n"), "{printed}");
}

#[test]
fn an_unclosed_substitution_rejects_the_whole_line() {
    let directory = TestDirectory::new("subst unclosed");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo first ; echo $(echo x",
            "echo $((1 + 2))",
            "echo still-here",
        ],
    );

    assert!(!printed.contains("first"), "{printed}");
    assert!(errors.contains("missing closing parenthesis"), "{errors}");
    // The line after the rejected one still runs, arithmetic included.
    assert!(printed.contains("3\n"), "{printed}");
    assert!(printed.contains("still-here\n"), "{printed}");
}

#[test]
fn the_standard_error_of_a_substitution_is_not_part_of_its_result() {
    let directory = TestDirectory::new("subst stderr");

    let (printed, errors) = shell_lines(&directory, &["echo [$(cat missing-input)]"]);

    assert!(printed.contains("[]\n"), "{printed}");
    assert!(errors.contains("missing-input"), "{errors}");
}

#[test]
fn substitutions_nested_too_deeply_are_refused() {
    let directory = TestDirectory::new("subst depth");
    let line = format!("echo {}x{}", "$(echo ".repeat(12), ")".repeat(12));

    let (_, errors) = shell_lines(&directory, &[&line]);

    assert!(errors.contains("nested too deeply"), "{errors}");
}

#[cfg(unix)]
#[test]
fn the_output_of_programs_is_collected_in_order() {
    let directory = TestDirectory::new("subst programs");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo [$(echo first ; printf second)]",
            "echo $(printf 'b\\na\\n' | sort)",
            "export TRS_SUBST=$(printf 'kept value')",
            "printenv TRS_SUBST",
        ],
    );

    assert!(printed.contains("[first second]\n"), "{printed}\n{errors}");
    assert!(printed.contains("a b\n"), "{printed}");
    assert!(printed.contains("kept value\n"), "{printed}");
}

#[test]
fn the_output_of_git_is_collected() {
    if !git_is_installed() {
        return;
    }
    let directory = TestDirectory::new("subst git");

    let (printed, errors) = shell_lines(&directory, &["echo version: $(git --version)"]);

    assert!(
        printed.contains("version: git version"),
        "{printed}\n{errors}"
    );
}

#[test]
fn braces_give_one_word_per_alternative_and_sequence_value() {
    let directory = TestDirectory::new("braces words");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo {a,b,c}",
            "echo file{.txt,.md}",
            "echo {1..5} {a..c} {01..03}",
            "echo {a,b}{1,2}",
            "echo '{kept,literal}' \"{also,literal}\"",
        ],
    );

    assert!(printed.contains("a b c\n"), "{printed}\n{errors}");
    assert!(printed.contains("file.txt file.md\n"), "{printed}");
    assert!(printed.contains("1 2 3 4 5 a b c 01 02 03\n"), "{printed}");
    assert!(printed.contains("a1 a2 b1 b2\n"), "{printed}");
    assert!(
        printed.contains("{kept,literal} {also,literal}\n"),
        "{printed}"
    );
}

#[test]
fn braces_build_paths_for_everyday_commands() {
    let directory = TestDirectory::new("braces paths");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "mkdir -p project/{src,tests,docs/{a,b}}",
            "touch project/{src,tests}/mod.txt",
            "cp project/src/mod.txt{,.bak}",
            "ls project/src",
        ],
    );

    let root = directory.path().join("project");
    assert!(
        root.join("docs/a").is_dir() && root.join("docs/b").is_dir(),
        "{errors}"
    );
    assert!(root.join("tests/mod.txt").exists(), "{errors}");
    assert!(root.join("src/mod.txt.bak").exists(), "{errors}");
    assert!(printed.contains("mod.txt.bak"), "{printed}\n{errors}");
}

#[test]
fn braces_combine_with_variables_substitutions_and_patterns() {
    let directory = TestDirectory::new("braces combine");
    fs::write(directory.path().join("a.txt"), "").expect("fixture");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "NAME=report",
            "echo ${NAME}/{x,y}",
            "echo {$(echo one),two}",
            "echo {a,b}.txt *.txt",
            "echo {a,b}>out.txt",
            "cat out.txt",
        ],
    );

    assert!(
        printed.contains("report/x report/y\n"),
        "{printed}\n{errors}"
    );
    assert!(printed.contains("one two\n"), "{printed}");
    assert!(printed.contains("a.txt b.txt a.txt\n"), "{printed}");
}

#[test]
fn a_huge_brace_product_is_refused_without_freezing() {
    let directory = TestDirectory::new("braces huge");

    let (printed, errors) = shell_lines(
        &directory,
        &["echo {1..100}{1..100}{1..2}", "echo still-here"],
    );

    assert!(errors.contains("too many words"), "{errors}");
    assert!(printed.contains("still-here\n"), "{printed}");
}

#[test]
fn mkdir_and_touch_take_several_targets_and_report_each_failure() {
    let directory = TestDirectory::new("several targets");
    fs::create_dir(directory.path().join("exists")).expect("fixture");

    let (_, errors) = shell_lines(
        &directory,
        &[
            "mkdir one exists two",
            "touch f1 f2 f3",
            "mkdir -p deep/er/still",
        ],
    );

    assert!(directory.path().join("one").is_dir() && directory.path().join("two").is_dir());
    assert!(errors.contains("'exists' already exists"), "{errors}");
    for file in ["f1", "f2", "f3"] {
        assert!(directory.path().join(file).is_file(), "{file}");
    }
    assert!(directory.path().join("deep/er/still").is_dir());
}

#[test]
fn cat_prints_every_file_of_a_pattern_and_keeps_going_after_a_missing_one() {
    let directory = TestDirectory::new("cat several");
    fs::write(directory.path().join("a.txt"), "alpha\n").expect("fixture");
    fs::write(directory.path().join("b.txt"), "beta\n").expect("fixture");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "cat *.txt",
            "cat a.txt nowhere.txt b.txt",
            "echo from-pipe | cat a.txt - b.txt",
        ],
    );

    assert!(printed.contains("alpha\nbeta\n"), "{printed}\n{errors}");
    assert!(errors.contains("nowhere.txt"), "{errors}");
    assert!(
        printed.contains("alpha\nfrom-pipe\nbeta\n"),
        "{printed}\n{errors}"
    );
}

#[test]
fn a_for_loop_runs_once_per_word_pattern_and_substitution() {
    let directory = TestDirectory::new("flow for");
    for file in ["a.txt", "b.txt"] {
        fs::write(directory.path().join(file), "").expect("fixture");
    }

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "for f in x y z; do echo word $f; done",
            "for f in *.txt; do echo file $f; done",
            "for n in {1..3}; do echo n=$n; done",
            "for w in $(echo p q); do echo sub $w; done",
            "for f in nothing-here-*; do echo literal $f; done",
            "for f in; do echo never; done",
            "echo last=$f",
        ],
    );

    assert!(
        printed.contains("word x\nword y\nword z\n"),
        "{printed}\n{errors}"
    );
    assert!(printed.contains("file a.txt\nfile b.txt\n"), "{printed}");
    assert!(printed.contains("n=1\nn=2\nn=3\n"), "{printed}");
    assert!(printed.contains("sub p\nsub q\n"), "{printed}");
    assert!(printed.contains("literal nothing-here-*\n"), "{printed}");
    assert!(!printed.contains("never"), "{printed}");
    // The variable keeps the last value after the loop.
    assert!(printed.contains("last=nothing-here-*\n"), "{printed}");
}

#[test]
fn loops_nest_and_hold_chains_and_pipes() {
    let directory = TestDirectory::new("flow nest");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "for a in 1 2; do for b in x y; do echo $a$b; done; done",
            "for w in hi yo; do echo $w | cat && echo ok-$w; done",
        ],
    );

    assert!(printed.contains("1x\n1y\n2x\n2y\n"), "{printed}\n{errors}");
    assert!(printed.contains("hi\nok-hi\nyo\nok-yo\n"), "{printed}");
}

#[test]
fn if_picks_the_first_branch_whose_condition_succeeds() {
    let directory = TestDirectory::new("flow if");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "if true; then echo A; fi",
            "if false; then echo no; else echo B; fi",
            "if false; then echo no; elif false; then echo no; elif true; then echo C; else echo no; fi",
            "if false; then echo no; fi",
            "echo status=$?",
            "if true && false; then echo no; else echo D; fi",
            "if echo cond; then echo E; fi",
        ],
    );

    for expected in ["A\n", "B\n", "C\n", "D\n", "cond\nE\n", "status=0\n"] {
        assert!(
            printed.contains(expected),
            "{expected:?} in {printed}\n{errors}"
        );
    }
    assert!(!printed.contains("no\n"), "{printed}");
}

#[test]
fn while_and_until_loop_on_the_status_of_their_condition() {
    let directory = TestDirectory::new("flow while");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "while test ! -f stop.txt; do echo turn; touch stop.txt; done",
            "until test -f made.txt; do touch made.txt; echo made; done",
            "while false; do echo never; done",
            "echo after=$?",
        ],
    );

    assert_eq!(printed.matches("turn\n").count(), 1, "{printed}\n{errors}");
    assert_eq!(printed.matches("made\n").count(), 1, "{printed}");
    assert!(!printed.contains("never"), "{printed}");
    assert!(printed.contains("after=0\n"), "{printed}");
}

#[test]
fn break_and_continue_control_the_innermost_loop() {
    let directory = TestDirectory::new("flow break");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "for n in 1 2 3 4; do if test $n = 2; then continue; fi; if test $n = 4; then break; fi; echo n=$n; done",
            "for a in 1 2; do for b in x y z; do if test $b = y; then break; fi; echo $a$b; done; done",
            "break",
            "continue",
            "echo still-here",
        ],
    );

    assert!(printed.contains("n=1\nn=3\n"), "{printed}\n{errors}");
    assert!(
        !printed.contains("n=2") && !printed.contains("n=4"),
        "{printed}"
    );
    assert!(printed.contains("1x\n2x\n"), "{printed}");
    assert!(
        !printed.contains("1y") && !printed.contains("1z"),
        "{printed}"
    );
    assert!(
        errors.contains("break: only meaningful in a loop"),
        "{errors}"
    );
    assert!(
        errors.contains("continue: only meaningful in a loop"),
        "{errors}"
    );
    assert!(printed.contains("still-here\n"), "{printed}");
}

#[test]
fn a_failing_condition_is_quiet_but_a_missing_command_is_reported() {
    let directory = TestDirectory::new("flow quiet");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "if cat missing-file.txt; then echo no; else echo handled; fi",
            "if terminal-rust-no-such-program; then echo no; else echo fallback; fi",
        ],
    );

    assert!(
        printed.contains("handled\n") && printed.contains("fallback\n"),
        "{printed}"
    );
    assert!(!errors.contains("missing-file.txt"), "{errors}");
    assert!(
        errors.contains("unknown command: terminal-rust-no-such-program"),
        "{errors}"
    );
}

#[test]
fn the_status_of_a_loop_is_the_one_of_its_last_command() {
    let directory = TestDirectory::new("flow status");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "for x in 1 2; do echo $x; done && echo chained",
            "for x in 1 2; do cat nowhere.txt; done || echo recovered",
            "echo code=$?",
        ],
    );

    assert!(printed.contains("1\n2\nchained\n"), "{printed}\n{errors}");
    assert!(printed.contains("recovered\n"), "{printed}");
    assert!(errors.matches("nowhere.txt").count() == 2, "{errors}");
}

#[test]
fn a_block_typed_over_several_lines_runs_when_it_is_closed() {
    let directory = TestDirectory::new("flow lines");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "for n in 1 2",
            "do",
            "  if test $n = 1",
            "  then",
            "    echo one",
            "  else",
            "    echo other",
            "  fi",
            "done",
            "echo after",
        ],
    );

    assert!(printed.contains("one\nother\n"), "{printed}\n{errors}");
    assert!(printed.contains("> after\n"), "{printed}");
}

#[test]
fn an_unfinished_or_misplaced_block_rejects_the_input_before_running_anything() {
    let directory = TestDirectory::new("flow syntax");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo first ; for x in a; do echo inside",
            "done",
            "echo second",
        ],
    );

    // The first two lines are one unfinished block, closed by the stray `done`
    // on the next line: they run as a whole. A real stray keyword is an error.
    assert!(
        printed.contains("first\n") || errors.contains("syntax error"),
        "{printed}\n{errors}"
    );

    let (printed, errors) = shell_lines(&directory, &["echo before ; done", "echo ok"]);
    assert!(!printed.contains("before"), "{printed}");
    assert!(errors.contains("unexpected token 'done'"), "{errors}");
    assert!(printed.contains("ok\n"), "{printed}");
}

#[cfg(unix)]
#[test]
fn exit_inside_a_loop_leaves_the_shell_with_its_code() {
    let directory = TestDirectory::new("flow exit");

    let output = run_interactive(
        directory.path(),
        "for x in 1 2 3; do echo turn $x; exit 3; done\necho unreachable\n",
    );

    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(stdout(&output).contains("turn 1\n"));
    assert!(!stdout(&output).contains("turn 2") && !stdout(&output).contains("unreachable"));
}

#[cfg(unix)]
#[test]
fn ctrl_c_stops_a_loop_and_the_shell_carries_on() {
    use std::io::Read;
    let directory = TestDirectory::new("flow interrupt");
    let mut child = shell_command()
        .current_dir(directory.path())
        .env("USER", "terminal-rust-test")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the terminal_rust binary should start");
    let mut stdin = child.stdin.take().expect("stdin should be piped");
    stdin
        .write_all(b"while true; do echo spinning; sleep 0.1; done\n")
        .expect("the loop should be sent");
    stdin.flush().expect("flush");
    std::thread::sleep(Duration::from_millis(700));

    // The signal goes to the shell only; the loop must notice it by itself.
    Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("kill should run");
    std::thread::sleep(Duration::from_millis(500));
    stdin
        .write_all(b"echo carried-on\nexit\n")
        .expect("the next lines should be sent");
    drop(stdin);

    // Read in the background so a loop that never stops cannot freeze the test.
    let mut stdout_pipe = child.stdout.take().expect("stdout");
    let reader = std::thread::spawn(move || {
        let mut output = String::new();
        let _ = stdout_pipe.read_to_string(&mut output);
        output
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .expect("the shell status should be readable")
        {
            break Some(status);
        }
        if std::time::Instant::now() > deadline {
            child.kill().expect("the stuck shell should be stopped");
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let output = reader.join().expect("the output reader should end");

    assert!(
        status.is_some(),
        "the loop never stopped after Ctrl+C: {output}"
    );
    assert!(output.contains("carried-on"), "{output}");
}

#[cfg(unix)]
#[test]
fn a_script_can_hold_a_loop_over_several_lines() {
    let directory = TestDirectory::new("flow script");
    fs::write(
        directory.path().join("loop.trs"),
        "# a loop in a script\nfor n in 1 2 3\ndo\n  echo script-$n\ndone\necho script-end\n",
    )
    .expect("script fixture");

    let (printed, errors) = shell_lines(&directory, &["script run loop.trs"]);

    assert!(
        printed.contains("script-1\nscript-2\nscript-3\nscript-end\n"),
        "{printed}\n{errors}"
    );
}

#[test]
fn test_and_brackets_decide_if_and_while_without_any_system_program() {
    let directory = TestDirectory::new("test builtin");
    fs::write(directory.path().join("data.txt"), "x").expect("fixture");
    fs::create_dir(directory.path().join("sub")).expect("fixture");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "if [ -f data.txt ]; then echo file; fi",
            "if [ -d sub ] && [ ! -e missing.txt ]; then echo dir-and-absent; fi",
            "if test -s data.txt -a -d sub; then echo both; fi",
            "if [ \"a b\" = \"a b\" ]; then echo equal; fi",
            "if [ 10 -gt 9 ]; then echo numeric; fi",
            "if [ -z \"\" ]; then echo empty; fi",
            "if [ -f nothing ]; then echo no; else echo not-a-file; fi",
            "for n in 1 2 3 4; do if [ $n -eq 2 ]; then continue; fi; echo n=$n; done",
            "while [ ! -f stop.txt ]; do touch stop.txt; echo once; done",
        ],
    );

    for expected in [
        "file\n",
        "dir-and-absent\n",
        "both\n",
        "equal\n",
        "numeric\n",
        "empty\n",
        "not-a-file\n",
        "n=1\nn=3\nn=4\n",
        "once\n",
    ] {
        assert!(
            printed.contains(expected),
            "{expected:?} in {printed}\n{errors}"
        );
    }
    assert!(!printed.contains("no\n"), "{printed}");
    assert!(
        errors.is_empty(),
        "a false condition must be silent: {errors}"
    );
}

#[test]
fn test_reports_its_status_and_its_mistakes() {
    let directory = TestDirectory::new("test status");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "[ 1 -eq 1 ]",
            "echo true=$?",
            "[ 1 -eq 2 ]",
            "echo false=$?",
            "[ x -eq 1 ]",
            "echo wrong=$?",
            "[ a = a",
            "echo unclosed=$?",
            "test",
            "echo none=$?",
        ],
    );

    assert!(printed.contains("true=0\n"), "{printed}\n{errors}");
    assert!(printed.contains("false=1\n"), "{printed}");
    assert!(printed.contains("wrong=2\n"), "{printed}");
    assert!(printed.contains("unclosed=2\n"), "{printed}");
    assert!(printed.contains("none=1\n"), "{printed}");
    assert!(
        errors.contains("integer expression expected: 'x'"),
        "{errors}"
    );
    assert!(errors.contains("[: missing ']'"), "{errors}");
    // A false condition (`[ 1 -eq 2 ]`, `test`) says nothing: only the two mistakes.
    assert_eq!(errors.lines().count(), 2, "{errors}");
}

#[test]
fn type_says_what_each_name_runs() {
    let directory = TestDirectory::new("type names");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "alias where pwd",
            "type for echo where which",
            "type terminal-rust-no-such-program",
            "echo status=$?",
            "type echo terminal-rust-no-such-program",
            "which cat",
        ],
    );

    assert!(
        printed.contains("for is a shell keyword\n"),
        "{printed}\n{errors}"
    );
    assert!(printed.contains("echo is a shell builtin\n"), "{printed}");
    assert!(
        printed.contains("where is an alias for 'pwd'\n"),
        "{printed}"
    );
    assert!(printed.contains("which is a shell builtin\n"), "{printed}");
    assert!(printed.contains("status=1\n"), "{printed}");
    assert!(
        errors
            .matches("type: terminal-rust-no-such-program: not found")
            .count()
            == 2,
        "{errors}"
    );
    // `echo` is found even when another name in the list is not.
    assert!(
        printed.matches("echo is a shell builtin").count() >= 2,
        "{printed}"
    );
}

#[cfg(unix)]
#[test]
fn type_gives_the_path_of_a_program() {
    let directory = TestDirectory::new("type path");

    let (printed, errors) = shell_lines(&directory, &["type sh", "type /bin/sh"]);

    assert!(
        printed.contains("sh is /") && printed.contains("/sh\n"),
        "{printed}\n{errors}"
    );
    assert!(printed.contains("/bin/sh is /bin/sh\n"), "{printed}");
}

#[cfg(unix)]
#[test]
fn type_only_finds_executable_files_that_exist() {
    use std::os::unix::fs::PermissionsExt;
    let directory = TestDirectory::new("type executable");
    let bin = directory.path().join("bin");
    fs::create_dir(&bin).expect("fixture");
    fs::write(bin.join("runme"), "#!/bin/sh\n").expect("fixture");
    fs::write(bin.join("plainfile"), "data").expect("fixture");
    fs::set_permissions(bin.join("runme"), fs::Permissions::from_mode(0o755)).expect("chmod");
    fs::set_permissions(bin.join("plainfile"), fs::Permissions::from_mode(0o644)).expect("chmod");

    let export = format!("export PATH=\"{}:$PATH\"", bin.display());
    let (printed, errors) = shell_lines(
        &directory,
        &[
            &export,
            "type runme",
            "type plainfile",
            "type /terminal-rust/no/such/program",
        ],
    );

    assert!(
        printed.contains(&format!("runme is {}/runme\n", bin.display())),
        "{printed}\n{errors}"
    );
    assert!(errors.contains("type: plainfile: not found"), "{errors}");
    assert!(
        errors.contains("type: /terminal-rust/no/such/program: not found"),
        "{errors}"
    );
    assert!(!printed.contains("plainfile is"), "{printed}");
}

#[test]
fn parameter_expansion_gives_defaults_alternatives_and_lengths() {
    let directory = TestDirectory::new("parameter expansion");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo default=${TR_UNSET:-fallback}",
            "TR_SET=value",
            "echo kept=${TR_SET:-fallback} alt=${TR_SET:+yes} none=[${TR_UNSET:+yes}]",
            "TR_EMPTY=",
            "echo empty=${TR_EMPTY:-fallback} bare=${TR_EMPTY-fallback}end",
            "echo length=${#TR_SET}",
            "echo nested=${TR_UNSET:-${TR_SET}}",
            "echo quoted=\"${TR_UNSET:-two words}\"",
            "echo before ${TR_UNSET:?missing value} after",
            "echo status=$?",
            "echo ${TR_UNSET:=x}",
        ],
    );

    for expected in [
        "default=fallback\n",
        "kept=value alt=yes none=[]\n",
        "empty=fallback bare=end\n",
        "length=5\n",
        "nested=value\n",
        "quoted=two words\n",
        "status=1\n",
    ] {
        assert!(
            printed.contains(expected),
            "{expected:?} in {printed}\n{errors}"
        );
    }
    assert!(
        !printed.contains("before"),
        "a failed expansion must not run the command: {printed}"
    );
    assert!(errors.contains("TR_UNSET: missing value"), "{errors}");
    assert!(
        errors.contains("assigning in an expansion is not supported"),
        "{errors}"
    );
}

#[test]
fn arithmetic_expansion_computes_in_lines_loops_and_assignments() {
    let directory = TestDirectory::new("arithmetic");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo sum=$((1+2)) product=$(( 6 * 7 )) group=$(( (1+2)*3 ))",
            "n=5",
            "echo double=$((n*2)) dollar=$(($n+1)) glued=$((2+2))abc",
            "total=$((10-3))",
            "echo total=$total",
            "echo \"quoted=$((7%4))\" single='$((1+1))'",
            "echo cmp=$((3>2)) eq=$((2==3)) logic=$((1&&0))",
            "i=0",
            "while [ $i -lt 3 ]; do echo i=$i; i=$((i+1)); done",
            "echo status=$((1+1))$?",
            "echo before $((1/0)) after",
            "echo failed=$?",
            "echo $((9223372036854775807+1))",
            "echo $((1+",
        ],
    );

    for expected in [
        "sum=3 product=42 group=9\n",
        "double=10 dollar=6 glued=4abc\n",
        "total=7\n",
        "quoted=3 single=$((1+1))\n",
        "cmp=1 eq=0 logic=0\n",
        "i=0\ni=1\ni=2\n",
        "failed=1\n",
    ] {
        assert!(
            printed.contains(expected),
            "{expected:?} in {printed}\n{errors}"
        );
    }
    assert!(
        !printed.contains("before"),
        "a failed expansion must not run the command: {printed}"
    );
    assert!(errors.contains("arithmetic: division by 0"), "{errors}");
    assert!(
        errors.contains("arithmetic: result out of range"),
        "{errors}"
    );
}

#[test]
fn source_runs_a_file_in_the_current_shell_and_reports_its_errors() {
    let directory = TestDirectory::new("source file");
    fs::write(
        directory.path().join("lib.sh"),
        "# a helper file\nTR_GREETING=hello\necho from-file $TR_GREETING\nfor n in 1 2; do\n  echo n=$n\ndone\n",
    )
    .expect("fixture");
    fs::write(
        directory.path().join("broken.sh"),
        "echo before-error\nterminal-rust-no-such-command\necho after-error\n",
    )
    .expect("fixture");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "source lib.sh",
            "echo kept=$TR_GREETING",
            ". ./lib.sh",
            "source broken.sh",
            "source missing.sh",
            "source lib.sh extra",
            "source",
        ],
    );

    assert!(
        printed.matches("from-file hello\n").count() == 2,
        "{printed}\n{errors}"
    );
    assert!(printed.contains("n=1\nn=2\n"), "{printed}");
    assert!(
        printed.contains("kept=hello\n"),
        "variables stay in the shell: {printed}"
    );
    assert!(printed.contains("before-error\n"), "{printed}");
    assert!(
        !printed.contains("after-error"),
        "an error stops the file: {printed}"
    );
    assert!(errors.contains("source: broken.sh:2:"), "{errors}");
    assert!(errors.contains("source: missing.sh:"), "{errors}");
    assert!(
        errors.contains("arguments for the file are not supported"),
        "{errors}"
    );
    assert!(errors.contains("source: expected a file"), "{errors}");
}

#[test]
fn read_stores_a_line_from_a_pipe_a_redirection_or_the_next_input_line() {
    let directory = TestDirectory::new("read line");
    fs::write(directory.path().join("empty.txt"), "").expect("fixture");
    fs::write(directory.path().join("one.txt"), "alpha beta gamma\n").expect("fixture");

    let (printed, errors) = shell_lines(
        &directory,
        &[
            "echo hello big world | read first rest",
            "echo first=$first rest=$rest",
            "read a b < one.txt",
            "echo a=$a b=$b",
            "read single < one.txt",
            "echo single=$single",
            "read < one.txt",
            "echo reply=$REPLY",
            "gone=old",
            "read gone < empty.txt",
            "echo eof-status=$? gone=[$gone]",
            "echo piped | read -p 'HIDDEN-PROMPT ' quiet",
            "echo quiet=$quiet",
            "read -p 'Name: ' who",
            "Alice",
            "echo typed=$who",
            "read bad-name < one.txt",
            "read -s x",
        ],
    );

    for expected in [
        "first=hello rest=big world\n",
        "a=alpha b=beta gamma\n",
        "single=alpha beta gamma\n",
        "reply=alpha beta gamma\n",
        "eof-status=1 gone=[]\n",
        "typed=Alice\n",
        "quiet=piped\n",
    ] {
        assert!(
            printed.contains(expected),
            "{expected:?} in {printed}\n{errors}"
        );
    }
    assert!(
        errors.contains("Name: "),
        "the prompt goes to the error stream: {errors}"
    );
    assert!(
        !errors.contains("HIDDEN-PROMPT"),
        "a pipe does not see the prompt: {errors}"
    );
    assert!(
        errors.contains("read: not a valid name: bad-name"),
        "{errors}"
    );
    assert!(errors.contains("read: unsupported option '-s'"), "{errors}");
}
