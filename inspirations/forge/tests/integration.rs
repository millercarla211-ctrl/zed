use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use forge::cli;
use forge::{
    build_remote_health_report, build_sync_overview, execute_sync, plan_sync_with_registry,
    remote_definition, retry_job, retry_job_at, upsert_remote, AuthStore, BranchMapping, MetadataDb,
    RemoteKind, RemoteRegistry, Repository, SyncDirection, TokenBundle,
};
use tempfile::tempdir;

fn cwd_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct CurrentDirGuard {
    original: PathBuf,
}

impl CurrentDirGuard {
    fn change_to(path: &Path) -> Self {
        let original = std::env::current_dir().expect("read current dir");
        std::env::set_current_dir(path).expect("set current dir");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

fn with_repo_dir<T>(path: &Path, run: impl FnOnce() -> T) -> T {
    let _guard = cwd_lock().lock().expect("lock cwd guard");
    let _cwd = CurrentDirGuard::change_to(path);
    run()
}

#[test]
fn init_creates_expected_layout() {
    let dir = tempdir().expect("tempdir");
    let repo = Repository::init(dir.path()).expect("init repo");

    assert!(repo.forge_dir.exists());
    for rel in [
        "objects/chunks",
        "objects/packs",
        "refs/heads",
        "refs/remotes",
        "manifests",
        "dictionaries",
        "mirrors",
    ] {
        assert!(repo.forge_dir.join(rel).exists(), "missing {}", rel);
    }

    let head = fs::read_to_string(repo.forge_dir.join("HEAD")).expect("read HEAD");
    assert_eq!(head, "ref: refs/heads/main\n");

    let db = MetadataDb::open(&repo.forge_dir.join("metadata.redb")).expect("open metadata db");
    assert!(db.get_all_tracked_files().expect("tracked files").is_empty());
}

#[test]
fn add_commit_and_checkout_roundtrip() {
    let dir = tempdir().expect("tempdir");
    Repository::init(dir.path()).expect("init repo");

    let file_path = dir.path().join("notes.txt");
    let original = b"forge roundtrip v1\n".to_vec();
    let updated = b"forge roundtrip v2\n".to_vec();
    fs::write(&file_path, &original).expect("write original file");

    let (first_commit, second_commit) = with_repo_dir(dir.path(), || {
        cli::add::run(&["notes.txt".to_string()], false).expect("stage original");
        cli::commit::run("initial snapshot").expect("commit original");

        let repo = Repository::discover(Path::new(".")).expect("discover repo");
        let first_commit = hex::encode(repo.read_head().expect("read head").expect("first head"));

        fs::write("notes.txt", &updated).expect("write updated file");
        cli::add::run(&["notes.txt".to_string()], false).expect("stage updated");
        cli::commit::run("updated snapshot").expect("commit updated");

        let second_commit =
            hex::encode(repo.read_head().expect("read head").expect("second head"));

        cli::checkout::run(&first_commit).expect("checkout first commit");
        assert_eq!(fs::read("notes.txt").expect("read after first checkout"), original);

        cli::checkout::run(&second_commit).expect("checkout second commit");
        assert_eq!(fs::read("notes.txt").expect("read after second checkout"), updated);

        (first_commit, second_commit)
    });

    assert_ne!(first_commit, second_commit);
}

#[test]
fn sync_overview_reports_primary_remote_and_auth_state() {
    let dir = tempdir().expect("tempdir");
    let repo = Repository::init(dir.path()).expect("init repo");

    let mut config = repo.read_config().expect("read config");
    config.remote_url = Some("https://github.com/acme/forge-demo.git".to_string());
    fs::write(
        repo.config_path(),
        toml::to_string_pretty(&config).expect("serialize config"),
    )
    .expect("write config");

    let auth = AuthStore::open(&repo.forge_dir).expect("open auth store");
    auth.save(
        "github",
        &TokenBundle {
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: None,
            extra: serde_json::Value::Null,
        },
    )
    .expect("save github token");

    let db = MetadataDb::open(&repo.metadata_db_path()).expect("open db");
    let overview = build_sync_overview(&repo, &db, &auth).expect("build sync overview");
    let primary = overview.primary_remote.expect("primary remote");

    assert_eq!(primary.name, "origin");
    assert!(primary.authenticated);
    assert_eq!(primary.locator.as_deref(), Some("https://github.com/acme/forge-demo.git"));
    assert_eq!(overview.authenticated_backends.len(), 1);
    assert_eq!(overview.authenticated_backends[0].name, "github");
}

#[test]
fn sync_plan_reports_missing_auth_once_per_remote() {
    let dir = tempdir().expect("tempdir");
    let repo = Repository::init(dir.path()).expect("init repo");
    let auth = AuthStore::open(&repo.forge_dir).expect("open auth");
    let registry = RemoteRegistry {
        version: 1,
        primary: Some("origin".to_string()),
        remotes: vec![remote_definition(
            "origin",
            RemoteKind::GitHub,
            "https://github.com/acme/forge-demo.git",
            Some("github".to_string()),
            vec![BranchMapping {
                local: "main".to_string(),
                remote: "main".to_string(),
                direction: SyncDirection::Bidirectional,
                enabled: true,
            }],
            true,
        )],
    };

    let plan =
        plan_sync_with_registry(&repo, &auth, &registry, None).expect("plan sync with auth gap");
    let auth_conflicts = plan
        .conflicts
        .iter()
        .filter(|conflict| {
            conflict.remote.as_deref() == Some("origin")
                && conflict.summary.contains("authentication")
        })
        .count();
    assert_eq!(auth_conflicts, 1);
}

#[test]
fn sync_plan_reports_missing_remote_ref_for_pull() {
    let dir = tempdir().expect("tempdir");
    let repo = Repository::init(dir.path()).expect("init repo");
    let auth = AuthStore::open(&repo.forge_dir).expect("open auth");
    let registry = RemoteRegistry {
        version: 1,
        primary: Some("origin".to_string()),
        remotes: vec![remote_definition(
            "origin",
            RemoteKind::GitHub,
            "https://github.com/acme/forge-demo.git",
            None,
            vec![BranchMapping {
                local: "main".to_string(),
                remote: "main".to_string(),
                direction: SyncDirection::Pull,
                enabled: true,
            }],
            true,
        )],
    };

    let plan = plan_sync_with_registry(&repo, &auth, &registry, None).expect("plan pull sync");
    assert!(plan.conflicts.iter().any(|conflict| {
        conflict.summary.contains("has no tracked branch ref")
            && conflict.remote.as_deref() == Some("origin")
    }));
}

#[test]
fn execute_sync_cancels_when_requested_remote_is_missing() {
    let dir = tempdir().expect("tempdir");
    let repo = Repository::init(dir.path()).expect("init repo");

    let report =
        execute_sync(&repo, Some("missing"), false, false).expect("execute sync with missing remote");
    assert!(report.results.is_empty());
    assert!(report
        .warnings
        .iter()
        .any(|warning| warning.contains("blocked by 1 unresolved conflict")));
    assert!(report
        .plan
        .conflicts
        .iter()
        .any(|conflict| conflict.summary.contains("requested remote 'missing'")));

    let db = MetadataDb::open(&repo.metadata_db_path()).expect("open db");
    let jobs = forge::list_jobs(&db).expect("list jobs");
    assert!(jobs.iter().any(|job| {
        job.description.contains("execute sync plan")
            && matches!(job.kind, forge::JobKind::SyncRun)
            && matches!(job.status, forge::JobStatus::Cancelled)
    }));
}

#[test]
fn retry_job_reuses_sync_job_and_increments_attempts() {
    let dir = tempdir().expect("tempdir");
    let repo = Repository::init(dir.path()).expect("init repo");

    let _ = execute_sync(&repo, Some("missing"), false, false).expect("initial sync run");
    let db = MetadataDb::open(&repo.metadata_db_path()).expect("open db");
    let first_job = forge::list_jobs(&db)
        .expect("list jobs")
        .into_iter()
        .find(|job| matches!(job.kind, forge::JobKind::SyncRun))
        .expect("sync job present");
    assert_eq!(first_job.attempts, 1);

    let outcome = retry_job_at(
        &repo,
        &first_job.id,
        first_job.updated_at_unix_ms + 30_000,
    )
    .expect("retry sync job");
    assert_eq!(outcome.job_id, first_job.id);
    assert_eq!(outcome.new_attempts, 2);

    let refreshed = forge::load_job(&db, &first_job.id)
        .expect("reload job")
        .expect("job still present");
    assert_eq!(refreshed.attempts, 2);
    assert!(matches!(refreshed.status, forge::JobStatus::Cancelled));
}

#[test]
fn retry_job_blocks_until_backoff_window_elapses() {
    let dir = tempdir().expect("tempdir");
    let repo = Repository::init(dir.path()).expect("init repo");

    let _ = execute_sync(&repo, Some("missing"), false, false).expect("initial sync run");
    let db = MetadataDb::open(&repo.metadata_db_path()).expect("open db");
    let first_job = forge::list_jobs(&db)
        .expect("list jobs")
        .into_iter()
        .find(|job| matches!(job.kind, forge::JobKind::SyncRun))
        .expect("sync job present");

    let error = retry_job(&repo, &first_job.id).expect_err("backoff should block immediate retry");
    assert!(error
        .to_string()
        .contains("waiting for retry backoff until"));
}

#[test]
fn remote_health_reports_auth_and_last_job_state() {
    let dir = tempdir().expect("tempdir");
    let repo = Repository::init(dir.path()).expect("init repo");
    let auth = AuthStore::open(&repo.forge_dir).expect("open auth");
    upsert_remote(
        &repo,
        &auth,
        remote_definition(
            "origin",
            RemoteKind::GitHub,
            "https://github.com/acme/forge-demo.git",
            Some("github".to_string()),
            vec![BranchMapping {
                local: "main".to_string(),
                remote: "main".to_string(),
                direction: SyncDirection::Bidirectional,
                enabled: true,
            }],
            true,
        ),
        true,
    )
    .expect("upsert remote");

    let _ = execute_sync(&repo, Some("origin"), false, false).expect("execute sync");
    let db = MetadataDb::open(&repo.metadata_db_path()).expect("open db");
    let health = build_remote_health_report(&repo, &db, &auth).expect("remote health");
    let origin = health
        .into_iter()
        .find(|remote| remote.name == "origin")
        .expect("origin health");

    assert!(!origin.authenticated);
    assert!(matches!(
        origin.last_job_status,
        Some(forge::JobStatus::Cancelled)
    ));
}

#[test]
fn sync_run_pushes_to_local_transport_remote() {
    let source_dir = tempdir().expect("source tempdir");
    let remote_dir = tempdir().expect("remote tempdir");
    let source_repo = Repository::init(source_dir.path()).expect("init source repo");
    let remote_repo = Repository::init(remote_dir.path()).expect("init remote repo");
    let auth = AuthStore::open(&source_repo.forge_dir).expect("open auth");

    upsert_remote(
        &source_repo,
        &auth,
        remote_definition(
            "lan",
            RemoteKind::ForgeTransport,
            &format!("forge+local://{}", remote_dir.path().display()),
            None,
            vec![BranchMapping {
                local: "main".to_string(),
                remote: "main".to_string(),
                direction: SyncDirection::Push,
                enabled: true,
            }],
            true,
        ),
        true,
    )
    .expect("configure transport remote");

    let commit_id = with_repo_dir(source_dir.path(), || {
        fs::write("notes.txt", b"forge transport sync\n").expect("write notes");
        cli::add::run(&["notes.txt".to_string()], false).expect("stage notes");
        cli::commit::run("transport sync fixture").expect("commit notes");
        let repo = Repository::discover(Path::new(".")).expect("discover source repo");
        hex::encode(repo.read_head().expect("read head").expect("head commit"))
    });

    let report = execute_sync(&source_repo, Some("lan"), false, false).expect("execute sync");
    assert!(report
        .results
        .iter()
        .any(|result| result.remote == "lan"
            && matches!(result.kind, forge::SyncActionKind::PushBranch)
            && matches!(result.state, forge::SyncActionState::Executed)));

    assert!(remote_repo.forge_dir.join("manifests").join(&commit_id).exists());
    assert!(remote_repo
        .read_head()
        .expect("read remote head")
        .is_none());
    assert_eq!(
        source_repo
            .read_remote_ref("lan", "main")
            .expect("read remote ref")
            .map(hex::encode),
        Some(commit_id)
    );
}
