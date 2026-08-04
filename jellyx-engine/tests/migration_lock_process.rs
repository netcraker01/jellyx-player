use jellyx_engine::migration_lock::{MigrationLock, MigrationLockError};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const HELPER_MODE: &str = "JELLYX_LOCK_HELPER";

#[test]
fn lock_helper() {
    let Ok(mode) = env::var(HELPER_MODE) else {
        return;
    };
    let path = PathBuf::from(env::var("JELLYX_LOCK_DB").unwrap());
    let timeout =
        Duration::from_millis(env::var("JELLYX_LOCK_TIMEOUT_MS").unwrap().parse().unwrap());
    match (mode.as_str(), MigrationLock::acquire(&path, timeout)) {
        ("owner", Ok(_guard)) => {
            println!("LOCKED");
            std::io::stdout().flush().unwrap();
            std::thread::sleep(Duration::from_millis(600));
        }
        ("contender", Err(MigrationLockError::Contended { .. })) => println!("CONTENDED"),
        ("contender", Ok(_guard)) => println!("ACQUIRED"),
        (_, Err(error)) => panic!("unexpected helper error: {error}"),
        _ => panic!("unexpected helper mode"),
    }
}

#[test]
fn subprocess_lock_contract() {
    let root = unique_temp_dir();
    fs::create_dir(&root).unwrap();
    let database = root.join("library.sqlite3");
    assert!(
        !database.exists(),
        "the lock must work before SQLite creates the DB"
    );

    let mut owner = helper("owner", &database, 0)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut owner_output = BufReader::new(owner.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        assert_ne!(owner_output.read_line(&mut line).unwrap(), 0);
        if line.contains("LOCKED") {
            break;
        }
    }

    let started = Instant::now();
    let blocked = helper("contender", &database, 150).output().unwrap();
    assert!(blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stdout).contains("CONTENDED"));
    assert!(started.elapsed() >= Duration::from_millis(140));
    assert!(started.elapsed() < Duration::from_millis(450));

    let still_owned = helper("contender", &database, 0).output().unwrap();
    assert!(
        String::from_utf8_lossy(&still_owned.stdout).contains("CONTENDED"),
        "a failed contender must not release the owner's lock"
    );
    assert!(owner.wait().unwrap().success());

    let released = helper("contender", &database, 0).output().unwrap();
    assert!(String::from_utf8_lossy(&released.stdout).contains("ACQUIRED"));
    assert!(!database.exists());

    let lock_path = root.join("library.sqlite3.migration.lock");
    fs::write(&lock_path, b"preserve-me").unwrap();
    drop(MigrationLock::acquire(&database, Duration::ZERO).unwrap());
    assert_eq!(fs::read(&lock_path).unwrap(), b"preserve-me");

    let missing_parent = root.join("missing").join("db.sqlite3");
    assert!(matches!(
        MigrationLock::acquire(&missing_parent, Duration::ZERO),
        Err(MigrationLockError::InvalidPath(_))
    ));
    let io_database = root.join("io.sqlite3");
    fs::create_dir(root.join("io.sqlite3.migration.lock")).unwrap();
    assert!(matches!(
        MigrationLock::acquire(&io_database, Duration::ZERO),
        Err(MigrationLockError::Io(_))
    ));
    reject_symlink_paths(&root, &database);
    fs::remove_dir_all(root).unwrap();
}

fn helper(mode: &str, database: &Path, timeout_ms: u64) -> Command {
    let mut command = Command::new(env::current_exe().unwrap());
    command
        .args(["--exact", "lock_helper", "--nocapture"])
        .env(HELPER_MODE, mode)
        .env("JELLYX_LOCK_DB", database)
        .env("JELLYX_LOCK_TIMEOUT_MS", timeout_ms.to_string());
    command
}

fn unique_temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    env::temp_dir().join(format!("jellyx-lock-{}-{nonce}", std::process::id()))
}

#[cfg(unix)]
fn reject_symlink_paths(root: &Path, database: &Path) {
    use std::os::unix::fs::symlink;

    let target = root.join("target");
    fs::write(&target, b"target").unwrap();
    let lock_path = root.join("library.sqlite3.migration.lock");
    fs::remove_file(&lock_path).unwrap();
    symlink(&target, &lock_path).unwrap();
    assert!(matches!(
        MigrationLock::acquire(database, Duration::ZERO),
        Err(MigrationLockError::InvalidPath(_))
    ));
    fs::remove_file(&lock_path).unwrap();
    let linked_database = root.join("linked.sqlite3");
    symlink(&target, &linked_database).unwrap();
    assert!(matches!(
        MigrationLock::acquire(&linked_database, Duration::ZERO),
        Err(MigrationLockError::InvalidPath(_))
    ));
}

#[cfg(not(unix))]
fn reject_symlink_paths(_root: &Path, _database: &Path) {}
