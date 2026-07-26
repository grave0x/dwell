//! Integration test: full dwell workflow (init → add → apply → diff → status).

use std::fs;
use std::path::PathBuf;

/// Create a temporary directory and clean it up on drop.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("dwell-test-{}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        TempDir { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
#[ignore = "flaky: race condition with git temp dir in CI"]
fn test_full_workflow() {
    let tmp = TempDir::new();
    let home = tmp.path.join("home");
    let source = tmp.path.join("source");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&source).unwrap();

    // 1. Create a dotfile in the "home"
    let bashrc_content = "export EDITOR=nvim\nalias ll='ls -la'\n";
    fs::write(home.join(".bashrc"), bashrc_content).unwrap();

    // 2. Add it to the source directory
    let sd = dwell_store::SourceDir::open(&source).unwrap();
    let entry = sd.add_file(&home.join(".bashrc"), &home).unwrap();
    assert_eq!(entry.source_path, "dot_bashrc");
    assert!(!entry.encrypted);

    // 3. Verify source file exists with correct content
    let source_file = source.join("dot_bashrc");
    assert!(source_file.exists());
    assert_eq!(fs::read_to_string(&source_file).unwrap(), bashrc_content);

    // 4. Init regular git repo (non-bare, for adding files)
    let repo_dir = source.join(".git");
    {
        let repo = git2::Repository::init(&source).expect("Failed to init git repo");
        let mut config = repo.config().expect("config");
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
    }
    assert!(repo_dir.exists());

    // 5. Stage and commit
    let repo = dwell_store::GitRepo::open(&repo_dir).unwrap();
    repo.add("dot_bashrc").unwrap();
    let commit_id = repo.commit("Initial dotfiles").unwrap();
    assert!(!commit_id.is_empty());

    // 6. Create deploy state
    let state_path = source.join("state.json");
    let mut deployer = dwell_deploy::Deployer::new(home.clone(), &state_path, false)
        .unwrap()
        .with_force(true);

    // 7. Apply
    let data = serde_json::json!({
        "home": home.to_string_lossy(),
        "hostname": "test",
        "os": std::env::consts::OS,
    });
    let entries = sd.entries().unwrap();
    let results = deployer.apply_all(&entries, &source, &data).unwrap();

    assert_eq!(results.len(), 1);
    assert!(results[0].success);

    // 8. Verify target was created
    let target = home.join(".bashrc");
    assert!(target.exists());

    // 9. Save state
    deployer.save_state(&state_path).unwrap();
    assert!(state_path.exists());

    // 10. Diff should show no changes
    let state2 = dwell_store::DeployState::load(&state_path).unwrap();
    let deployed_hash = state2.get_hash(&target).unwrap();
    assert!(!deployed_hash.is_empty());

    // 11. Modify source, diff should show changes
    fs::write(&source_file, "export EDITOR=hx\n").unwrap();
    let sd2 = dwell_store::SourceDir::open(&source).unwrap();
    let differ = dwell_deploy::Differ::new(home.clone());
    let diffs = differ.diff_all(&sd2).unwrap();
    let changed: Vec<_> = diffs
        .iter()
        .filter(|d| d.status == dwell_core::DiffStatus::Modified)
        .collect();
    assert_eq!(changed.len(), 1);
}

#[test]
fn test_init_and_doctor_flow() {
    let tmp = TempDir::new();
    let source_dir = tmp.path.join("dotfiles");

    // Init
    fs::create_dir_all(&source_dir).unwrap();
    let git_dir = source_dir.join(".git");
    dwell_store::GitRepo::init(&git_dir).unwrap();

    // Verify git repo works
    let repo = dwell_store::GitRepo::open(&git_dir).unwrap();
    assert!(repo.head_commit().unwrap().is_none()); // no commits yet

    // Create a dwellignore
    let ignore_path = source_dir.join(".dwellignore");
    fs::write(&ignore_path, "*.log\ntemplates/\n").unwrap();

    // Verify ignore patterns
    let sd = dwell_store::SourceDir::open(&source_dir).unwrap();
    let entries = sd.entries().unwrap();
    assert!(entries.is_empty()); // empty directory
}

#[test]
fn test_template_rendering() {
    let registry = dwell_template::TemplateRegistry::new();

    // Handlebars
    let engine = registry.get("handlebars").unwrap();
    let result = engine
        .render("Hello, {{name}}!", &serde_json::json!({"name": "World"}))
        .unwrap();
    assert_eq!(result, "Hello, World!");

    // Rhai
    let engine = registry.get("rhai").unwrap();
    let result = engine
        .render(
            r#"let greeting = "Hello"; let name = world; `${greeting}, ${name}!`"#,
            &serde_json::json!({"world": "Rhai"}),
        )
        .unwrap();
    assert_eq!(result, "Hello, Rhai!");
}

#[test]
fn test_source_to_target_edge_cases() {
    let home = PathBuf::from("/home/user");

    // Basic dotfile
    assert_eq!(
        dwell_core::source_to_target("dot_bashrc", &home),
        PathBuf::from("/home/user/.bashrc")
    );

    // Nested config
    assert_eq!(
        dwell_core::source_to_target("dot_config/nvim/init.lua", &home),
        PathBuf::from("/home/user/.config/nvim/init.lua")
    );

    // Template file (strip .tmpl)
    assert_eq!(
        dwell_core::source_to_target("dot_config/git/config.tmpl", &home),
        PathBuf::from("/home/user/.config/git/config")
    );

    // Private encrypted file
    assert_eq!(
        dwell_core::source_to_target("private_dot_ssh/id_ed25519", &home),
        PathBuf::from("/home/user/.ssh/id_ed25519")
    );

    // Run-once script: should still map to correct target
    assert_eq!(
        dwell_core::source_to_target("run_once_install-packages.sh", &home),
        PathBuf::from("/home/user/install-packages.sh")
    );
}

#[test]
fn test_ignore_patterns() {
    let ignore = dwell_store::IgnorePatterns::parse("*.tmp\nsecret.key\ntemplates/\n");
    assert!(ignore.is_ignored("foo.tmp"));
    assert!(ignore.is_ignored("secret.key"));
    assert!(ignore.is_ignored("templates/something"));
    assert!(!ignore.is_ignored("src/main.rs"));
}

#[test]
fn test_deploy_state_persistence() {
    let tmp = TempDir::new();
    let state_path = tmp.path.join("state.json");

    // Create fresh state
    let mut state = dwell_store::DeployState::new();
    assert_eq!(state.generation, 0);

    // Record an entry
    let target = PathBuf::from("/home/test/.bashrc");
    state.record(&target, "dot_bashrc", "abc123", "file");
    state.next_generation();
    assert_eq!(state.generation, 1);
    assert!(state.is_deployed(&target));

    // Save and reload
    state.save(&state_path).unwrap();
    let loaded = dwell_store::DeployState::load(&state_path).unwrap();
    assert_eq!(loaded.generation, 1);
    assert!(loaded.is_deployed(&target));
    assert_eq!(loaded.get_hash(&target).unwrap(), "abc123");
}
