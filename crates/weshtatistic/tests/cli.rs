// -- Clippy Denies --
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::{path::PathBuf, process::Command};

/// Creates a fresh, uniquely named temp dir under the crate's `target/` dir.
fn fresh_temp_dir(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = std::env::current_dir()?.join("target").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn weshtatistic_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_weshtatistic"))
}

/// `--benchmark` without a positional path must fail with a usage error.
#[test]
fn test_benchmark_without_path_errors() -> Result<(), Box<dyn std::error::Error>> {
    let output = weshtatistic_cli().arg("--benchmark").output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("path must be provided"),
        "unexpected stderr: {stderr}"
    );
    Ok(())
}

/// `--benchmark` on a directory with two files succeeds and prints traversal stats.
#[test]
fn test_benchmark_on_dir_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = fresh_temp_dir("test_cli_benchmark_dir")?;
    std::fs::write(temp_dir.join("file_a.txt"), b"hello")?;
    std::fs::write(temp_dir.join("file_b.txt"), b"world!")?;

    let output = weshtatistic_cli().arg("--benchmark").arg(&temp_dir).output()?;

    assert!(
        output.status.success(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Files scanned: 2"),
        "unexpected stdout: {stdout}"
    );
    assert!(
        stdout.contains("Directories scanned:"),
        "unexpected stdout: {stdout}"
    );
    assert!(
        stdout.contains("Total bytes:"),
        "unexpected stdout: {stdout}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

/// `--benchmark` on a regular file (not a dir, not `$MFT`) must fail.
#[test]
fn test_benchmark_on_regular_file_errors() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = fresh_temp_dir("test_cli_benchmark_file")?;
    let file_path = temp_dir.join("regular_file.txt");
    std::fs::write(&file_path, b"data")?;

    let output = weshtatistic_cli().arg("--benchmark").arg(&file_path).output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not a directory"),
        "unexpected stderr: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

/// `--to <dest>` without an extension saves a compressed `<dest>.edst.zst` snapshot
/// that round-trips through `load_snapshot`.
#[test]
fn test_to_dest_saves_compressed_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = fresh_temp_dir("test_cli_to_compressed")?;
    let scan_dir = temp_dir.join("scan_input");
    std::fs::create_dir_all(&scan_dir)?;
    std::fs::write(scan_dir.join("known_file.txt"), b"known contents")?;
    let dest = temp_dir.join("snapshot_out");

    // `--no-docker`: never touch the host's Docker installation from a test.
    let output = weshtatistic_cli()
        .arg("--to")
        .arg(&dest)
        .arg("--no-docker")
        .arg(&scan_dir)
        .output()?;

    assert!(
        output.status.success(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let snapshot_path = temp_dir.join("snapshot_out.edst.zst");
    let bytes = std::fs::read(&snapshot_path)?;
    assert_eq!(bytes.get(..4), Some(&[0x28, 0xB5, 0x2F, 0xFD][..]));

    let (arena, string_pool) = weshtatistic::snapshot::load_snapshot(&snapshot_path)?;
    assert!(
        arena
            .nodes()
            .iter()
            .any(|node| string_pool.get(node.name_id) == Some("known_file.txt")),
        "scanned file name missing from snapshot string pool"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

/// `--to <dest> --no-compression` saves an uncompressed `<dest>.edst` snapshot
/// with the raw `EDST` magic that round-trips through `load_snapshot`.
#[test]
fn test_to_dest_saves_uncompressed_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = fresh_temp_dir("test_cli_to_uncompressed")?;
    let scan_dir = temp_dir.join("scan_input");
    std::fs::create_dir_all(&scan_dir)?;
    std::fs::write(scan_dir.join("known_file.txt"), b"known contents")?;
    let dest = temp_dir.join("snapshot_out");

    let output = weshtatistic_cli()
        .arg("--to")
        .arg(&dest)
        .arg("--no-compression")
        .arg("--no-docker")
        .arg(&scan_dir)
        .output()?;

    assert!(
        output.status.success(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let snapshot_path = temp_dir.join("snapshot_out.edst");
    let bytes = std::fs::read(&snapshot_path)?;
    assert_eq!(bytes.get(..4), Some(b"EDST".as_slice()));

    let (arena, string_pool) = weshtatistic::snapshot::load_snapshot(&snapshot_path)?;
    assert!(
        arena
            .nodes()
            .iter()
            .any(|node| string_pool.get(node.name_id) == Some("known_file.txt")),
        "scanned file name missing from snapshot string pool"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

/// `--to <dest>` without a positional scan path exits with code 1 and a usage error.
#[test]
fn test_to_without_scan_path_errors() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = fresh_temp_dir("test_cli_to_no_path")?;
    let dest = temp_dir.join("snapshot_out");

    let output = weshtatistic_cli().arg("--to").arg(&dest).output()?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("path to scan must be provided"),
        "unexpected stderr: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

/// `--no-docker` is accepted by the CLI (clap plumbing): the benchmark path
/// still fails on the missing path, proving the flag parsed rather than
/// clap rejecting an unknown argument.
#[test]
fn test_no_docker_flag_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    let output = weshtatistic_cli()
        .arg("--benchmark")
        .arg("--no-docker")
        .output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("path must be provided"),
        "unexpected stderr: {stderr}"
    );
    Ok(())
}

/// `--no-docker --to` saves a snapshot without the Docker extension — even on
/// hosts where Docker is present, so this assertion is machine-independent.
#[test]
fn test_to_with_no_docker_skips_docker_extension() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = fresh_temp_dir("test_cli_to_no_docker")?;
    let scan_dir = temp_dir.join("scan_input");
    std::fs::create_dir_all(&scan_dir)?;
    std::fs::write(scan_dir.join("known_file.txt"), b"known contents")?;
    let dest = temp_dir.join("snapshot_out");

    let output = weshtatistic_cli()
        .arg("--to")
        .arg(&dest)
        .arg("--no-docker")
        .arg(&scan_dir)
        .output()?;

    assert!(
        output.status.success(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let snapshot_path = temp_dir.join("snapshot_out.edst.zst");
    let loaded = weshtatistic::snapshot::load_snapshot_full(&snapshot_path)?;
    assert!(
        loaded
            .extensions
            .get(weshtatistic::extensions::EXT_DOCKER_INVENTORY)
            .is_none(),
        "--no-docker must not attach a Docker inventory"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}
