use serde_json::Value;
use std::{fs, process::Command};
use tempfile::tempdir;

#[test]
fn mosaic_help_exposes_agentic_control_surface() {
    let output = Command::new(env!("CARGO_BIN_EXE_mosaic"))
        .arg("--help")
        .output()
        .expect("mosaic --help should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Open Mosaic agentic terminal workspace control CLI"));
    assert!(stdout.contains("prompt"));
    assert!(stdout.contains("subscribe"));
}

#[test]
fn prompt_dry_run_emits_versioned_receipt_without_connecting() {
    let state_dir = tempdir().expect("state tempdir");
    let output = Command::new(env!("CARGO_BIN_EXE_mosaic"))
        .env("XDG_STATE_HOME", state_dir.path())
        .args([
            "--session",
            "test-session",
            "--dry-run",
            "prompt",
            "send",
            "--pane-id",
            "terminal_1",
            "--text",
            "hello",
        ])
        .output()
        .expect("mosaic prompt dry-run should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let receipt: Value = serde_json::from_str(stdout.trim()).expect("receipt json");
    assert_eq!(receipt["schema_version"], "mosaic.control.v1");
    assert_eq!(receipt["operation"], "prompt.send");
    assert_eq!(receipt["status"], "dry_run");
    assert_eq!(receipt["pane_id"], "terminal_1");
}

#[test]
fn prompt_queue_writes_ndjson_queue_record() {
    let state_dir = tempdir().expect("state tempdir");
    let output = Command::new(env!("CARGO_BIN_EXE_mosaic"))
        .env("XDG_STATE_HOME", state_dir.path())
        .args([
            "--session",
            "queued-session",
            "prompt",
            "send",
            "--pane-id",
            "terminal_1",
            "--queue",
            "--text",
            "line one\nline two",
        ])
        .output()
        .expect("mosaic prompt queue should run");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let receipt: Value = serde_json::from_str(stdout.trim()).expect("receipt json");
    assert_eq!(receipt["status"], "queued");

    let queue_path = state_dir
        .path()
        .join("open-mosaic")
        .join("queues")
        .join("queued-session")
        .join("terminal_1.ndjson");
    let queue = fs::read_to_string(queue_path).expect("queue file");
    let record: Value = serde_json::from_str(queue.trim()).expect("queue json");
    assert_eq!(record["schema_version"], "mosaic.control.v1");
    assert_eq!(record["event"], "queued_prompt");
    assert_eq!(record["prompt"], "line one\nline two");
}

#[cfg(unix)]
#[test]
fn prompt_queue_uses_private_unix_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let state_dir = tempdir().expect("state tempdir");
    let output = Command::new(env!("CARGO_BIN_EXE_mosaic"))
        .env("XDG_STATE_HOME", state_dir.path())
        .args([
            "--session",
            "private-session",
            "prompt",
            "send",
            "--pane-id",
            "terminal_1",
            "--queue",
            "--text",
            "secret",
        ])
        .output()
        .expect("mosaic prompt queue should run");
    assert!(output.status.success());

    let queue_dir = state_dir
        .path()
        .join("open-mosaic")
        .join("queues")
        .join("private-session");
    let queue_file = queue_dir.join("terminal_1.ndjson");
    assert_eq!(
        fs::metadata(queue_dir)
            .expect("queue dir metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(queue_file)
            .expect("queue file metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn prompt_queue_rejects_session_path_traversal() {
    let state_dir = tempdir().expect("state tempdir");
    let output = Command::new(env!("CARGO_BIN_EXE_mosaic"))
        .env("XDG_STATE_HOME", state_dir.path())
        .args([
            "--session",
            "../../escape",
            "prompt",
            "send",
            "--pane-id",
            "terminal_1",
            "--queue",
            "--text",
            "do not write",
        ])
        .output()
        .expect("mosaic prompt queue should run");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: Value = serde_json::from_str(stderr.trim()).expect("error json");
    assert_eq!(error["code"], "invalid_path_component");
    assert!(!state_dir.path().join("escape").exists());
}

#[test]
fn prompt_queue_rejects_invalid_pane_id_before_receipt() {
    let state_dir = tempdir().expect("state tempdir");
    let output = Command::new(env!("CARGO_BIN_EXE_mosaic"))
        .env("XDG_STATE_HOME", state_dir.path())
        .args([
            "--session",
            "queued-session",
            "prompt",
            "send",
            "--pane-id",
            "../pane",
            "--queue",
            "--text",
            "do not write",
        ])
        .output()
        .expect("mosaic prompt queue should run");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: Value = serde_json::from_str(stderr.trim()).expect("error json");
    assert_eq!(error["code"], "invalid_pane_id");
}

#[test]
fn prompt_queue_does_not_emit_success_receipt_when_persistence_fails() {
    let state_dir = tempdir().expect("state tempdir");
    let blocking_file = state_dir.path().join("not-a-directory");
    fs::write(&blocking_file, "blocks state dir").expect("blocking file");

    let output = Command::new(env!("CARGO_BIN_EXE_mosaic"))
        .env("XDG_STATE_HOME", &blocking_file)
        .args([
            "--session",
            "queued-session",
            "prompt",
            "send",
            "--pane-id",
            "terminal_1",
            "--queue",
            "--text",
            "do not report queued",
        ])
        .output()
        .expect("mosaic prompt queue should run");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());

    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: Value = serde_json::from_str(stderr.trim()).expect("error json");
    assert_eq!(error["code"], "state_write_failed");
}

#[test]
fn runtime_errors_are_machine_readable_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_mosaic"))
        .args(["--session", "missing-session", "panes", "list"])
        .output()
        .expect("mosaic panes list should run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: Value = serde_json::from_str(stderr.trim()).expect("error json");
    assert_eq!(error["schema_version"], "mosaic.control.v1");
    assert_eq!(error["event"], "error");
}
