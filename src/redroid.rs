use std::{fs, process::Command};

use crate::cli::RedroidCommand;

const NAME: &str = "drunk-redroid";
const IMAGE: &str =
    "redroid/redroid@sha256:a6c464bbedcf1dcb67dbf91f329fbb19bee5b50631f0ca6bda6ed7c41b0e64e2";
const ADB: &str = "127.0.0.1:5555";

pub fn run(command: RedroidCommand) -> Result<(), String> {
    ensure_local_docker()?;
    match command {
        RedroidCommand::Start { allow_privileged } => start(allow_privileged),
        RedroidCommand::Status => {
            match state()? {
                Some(state) => println!("{NAME}: {state}"),
                None => println!("{NAME}: absent"),
            }
            Ok(())
        }
        RedroidCommand::Stop => {
            let Some(state) = state()? else {
                println!("{NAME}: absent (nothing to stop)");
                return Ok(());
            };
            validate_managed_container()?;
            if state != "running" {
                println!("{NAME}: already {state}; data volume and image preserved");
                return Ok(());
            }
            docker(&["container", "stop", NAME])?;
            println!("{NAME}: stopped; data volume and image preserved");
            Ok(())
        }
        RedroidCommand::Demo => demo(),
    }
}

fn ensure_local_docker() -> Result<(), String> {
    if std::env::var_os("DOCKER_HOST").is_some() || std::env::var_os("DOCKER_CONTEXT").is_some() {
        return Err(
            "DOCKER_HOST/DOCKER_CONTEXT overrides are unsupported; refusing to target an unknown Docker host".into(),
        );
    }
    let context = docker(&["context", "show"])?;
    if context.trim() != "default" {
        return Err(format!(
            "Docker context {} is selected; this prototype only supports the default local context",
            context.trim()
        ));
    }
    let endpoint = docker(&[
        "context",
        "inspect",
        "default",
        "--format",
        "{{.Endpoints.docker.Host}}",
    ])?;
    if !endpoint.trim().starts_with("unix://") {
        return Err(format!(
            "Docker endpoint {} is not a local Unix socket; refusing to manage redroid there",
            endpoint.trim()
        ));
    }
    Ok(())
}

fn start(allow_privileged: bool) -> Result<(), String> {
    if !allow_privileged {
        return Err(
            "start requires --allow-privileged: redroid needs Docker privileged mode".into(),
        );
    }
    let modules = fs::read_to_string("/proc/modules")
        .map_err(|error| format!("cannot read /proc/modules: {error}"))?;
    if !modules
        .lines()
        .any(|line| line.split_whitespace().next() == Some("binder_linux"))
    {
        return Err(
            "binder_linux is not loaded; load the host module before starting redroid".into(),
        );
    }
    docker(&["info"])?;
    if let Some(state) = state()? {
        if state == "running" {
            validate_managed_container()?;
            println!("{NAME}: already running");
            return Ok(());
        }
        return Err(format!(
            "{NAME} already exists in state {state}; inspect it manually before removing or restarting it"
        ));
    }
    docker(&[
        "run",
        "--detach",
        "--name",
        NAME,
        "--platform",
        "linux/amd64",
        "--privileged",
        "--restart=no",
        "--volume",
        "drunk-redroid-data:/data",
        "--publish",
        "127.0.0.1:5555:5555",
        IMAGE,
    ])?;
    println!("{NAME}: started; Android boot may take time (ADB {ADB})");
    Ok(())
}

fn state() -> Result<Option<String>, String> {
    let ids = docker(&[
        "container",
        "ls",
        "--all",
        "--filter",
        "name=^/drunk-redroid$",
        "--format",
        "{{.ID}}",
    ])?;
    if ids.trim().is_empty() {
        return Ok(None);
    }
    let state = docker(&[
        "container",
        "inspect",
        "--format",
        "{{.State.Status}}",
        NAME,
    ])?;
    Ok(Some(state.trim().to_owned()))
}

fn demo() -> Result<(), String> {
    if state()?.as_deref() != Some("running") {
        return Err(format!(
            "{NAME} must be running before opening Android Settings"
        ));
    }
    validate_managed_container()?;
    let connection = execute("adb", &["connect", ADB])?;
    if !(connection.contains("connected to") || connection.contains("already connected to")) {
        return Err(format!(
            "ADB could not connect to {ADB}: {}",
            connection.trim()
        ));
    }
    let output = execute(
        "adb",
        &[
            "-s",
            ADB,
            "shell",
            "am",
            "start",
            "-a",
            "android.settings.SETTINGS",
        ],
    )?;
    if output.contains("Error:") || output.contains("Exception") {
        return Err(format!("Android Settings failed: {}", output.trim()));
    }
    println!("{}", output.trim());
    println!("Android Settings requested; this spike does not provide a display window.");
    Ok(())
}

fn validate_managed_container() -> Result<(), String> {
    let output = docker(&["container", "inspect", "--format", "{{json .}}", NAME])?;
    let inspect: serde_json::Value = serde_json::from_str(&output)
        .map_err(|error| format!("could not parse Docker inspect output: {error}"))?;

    let expected_ports = serde_json::json!({
        "5555/tcp": [{"HostIp": "127.0.0.1", "HostPort": "5555"}]
    });
    let expected_mount = inspect
        .get("Mounts")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|mounts| {
            mounts.iter().any(|mount| {
                mount.get("Type").and_then(serde_json::Value::as_str) == Some("volume")
                    && mount.get("Name").and_then(serde_json::Value::as_str)
                        == Some("drunk-redroid-data")
                    && mount.get("Destination").and_then(serde_json::Value::as_str) == Some("/data")
            })
        });
    let managed = inspect
        .pointer("/Config/Image")
        .and_then(serde_json::Value::as_str)
        == Some(IMAGE)
        && inspect
            .pointer("/HostConfig/Privileged")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        && inspect
            .pointer("/HostConfig/NetworkMode")
            .and_then(serde_json::Value::as_str)
            == Some("bridge")
        && inspect
            .pointer("/HostConfig/PortBindings")
            .is_some_and(|ports| ports == &expected_ports)
        && inspect
            .pointer("/HostConfig/RestartPolicy/Name")
            .and_then(serde_json::Value::as_str)
            == Some("no")
        && expected_mount;

    if !managed {
        return Err(format!(
            "{NAME} does not match the pinned image, privileged bridge mode, loopback-only ADB, no-restart policy, and named data volume; refusing to reuse it"
        ));
    }
    Ok(())
}

fn execute(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("could not execute {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} failed ({}): {}{}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim(),
            if output.stderr.is_empty() {
                String::from_utf8_lossy(&output.stdout).trim().to_owned()
            } else {
                String::new()
            }
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("{program} returned invalid UTF-8: {error}"))
}

fn docker(args: &[&str]) -> Result<String, String> {
    let mut pinned_args = vec!["--context", "default"];
    pinned_args.extend_from_slice(args);
    execute("docker", &pinned_args)
}
