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
        RedroidCommand::View => view(),
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

fn view() -> Result<(), String> {
    if state()?.as_deref() != Some("running") {
        return Err(format!("{NAME} must be running before opening its display"));
    }
    validate_managed_container()?;
    connect_adb()?;

    let serial = ADB;
    let first_frame = capture_frame(serial)?;
    let source_width = first_frame.width();
    let source_height = first_frame.height();
    let scale = (540.0 / source_width as f32)
        .min(900.0 / source_height as f32)
        .min(1.0);
    let width = ((source_width as f32 * scale).round() as u32).max(1);
    let height = ((source_height as f32 * scale).round() as u32).max(1);
    let mut buffer = make_window_buffer(&first_frame, width, height);

    let mut window = minifb::Window::new(
        "Drunk [PROTÓTIPO • até 4 FPS] — Android preview (Esc fecha)",
        width as usize,
        height as usize,
        minifb::WindowOptions::default(),
    )
    .map_err(|error| format!("could not create Android display window: {error}"))?;
    window.set_target_fps(30);

    let mut previous_mouse_down = false;
    let mut last_capture = std::time::Instant::now();
    println!("Android preview opened at {source_width}×{source_height}; input is sent over ADB.");

    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        window
            .update_with_buffer(&buffer, width as usize, height as usize)
            .map_err(|error| format!("could not update Android display window: {error}"))?;

        let mouse_down = window.get_mouse_down(minifb::MouseButton::Left);
        if mouse_down && !previous_mouse_down {
            if let Some((x, y)) = window.get_mouse_pos(minifb::MouseMode::Discard) {
                let android_x = (x / width as f32 * source_width as f32)
                    .clamp(0.0, source_width.saturating_sub(1) as f32)
                    as u32;
                let android_y = (y / height as f32 * source_height as f32)
                    .clamp(0.0, source_height.saturating_sub(1) as f32)
                    as u32;
                send_adb_input(
                    serial,
                    &[
                        "shell",
                        "input",
                        "tap",
                        &android_x.to_string(),
                        &android_y.to_string(),
                    ],
                )?;
            }
        }
        previous_mouse_down = mouse_down;

        for key in window.get_keys_pressed(minifb::KeyRepeat::No) {
            if key == minifb::Key::Escape {
                continue;
            }
            if let Some(keycode) = android_keycode(key) {
                send_adb_input(
                    serial,
                    &["shell", "input", "keyevent", &keycode.to_string()],
                )?;
            }
        }

        if last_capture.elapsed() >= std::time::Duration::from_millis(250) {
            match capture_frame(serial) {
                Ok(frame) if frame.width() == source_width && frame.height() == source_height => {
                    buffer = make_window_buffer(&frame, width, height);
                }
                Ok(_) => {
                    return Err(
                        "Android display dimensions changed; close and reopen the preview".into(),
                    );
                }
                Err(error) => eprintln!("display refresh failed: {error}"),
            }
            last_capture = std::time::Instant::now();
        }
    }
    Ok(())
}

fn connect_adb() -> Result<(), String> {
    let output = execute("adb", &["connect", ADB])?;
    if output.contains("connected to") || output.contains("already connected to") {
        Ok(())
    } else {
        Err(format!("ADB could not connect to {ADB}: {}", output.trim()))
    }
}

fn send_adb_input(serial: &str, command: &[&str]) -> Result<(), String> {
    let mut args = vec!["-s", serial];
    args.extend_from_slice(command);
    execute("adb", &args).map(|_| ())
}

fn capture_frame(serial: &str) -> Result<image::DynamicImage, String> {
    let output = Command::new("adb")
        .args(["-s", serial, "exec-out", "screencap", "-p"])
        .output()
        .map_err(|error| format!("could not execute adb screen capture: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "ADB screen capture failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    image::load_from_memory(&output.stdout)
        .map_err(|error| format!("could not decode Android screenshot: {error}"))
}

fn make_window_buffer(frame: &image::DynamicImage, width: u32, height: u32) -> Vec<u32> {
    let resized =
        image::imageops::resize(frame, width, height, image::imageops::FilterType::Triangle);
    resized
        .pixels()
        .map(|pixel| (u32::from(pixel[0]) << 16) | (u32::from(pixel[1]) << 8) | u32::from(pixel[2]))
        .collect()
}

fn android_keycode(key: minifb::Key) -> Option<u32> {
    use minifb::Key;
    Some(match key {
        Key::A => 29,
        Key::B => 30,
        Key::C => 31,
        Key::D => 32,
        Key::E => 33,
        Key::F => 34,
        Key::G => 35,
        Key::H => 36,
        Key::I => 37,
        Key::J => 38,
        Key::K => 39,
        Key::L => 40,
        Key::M => 41,
        Key::N => 42,
        Key::O => 43,
        Key::P => 44,
        Key::Q => 45,
        Key::R => 46,
        Key::S => 47,
        Key::T => 48,
        Key::U => 49,
        Key::V => 50,
        Key::W => 51,
        Key::X => 52,
        Key::Y => 53,
        Key::Z => 54,
        Key::Key0 => 7,
        Key::Key1 => 8,
        Key::Key2 => 9,
        Key::Key3 => 10,
        Key::Key4 => 11,
        Key::Key5 => 12,
        Key::Key6 => 13,
        Key::Key7 => 14,
        Key::Key8 => 15,
        Key::Key9 => 16,
        Key::Down => 20,
        Key::Left => 21,
        Key::Right => 22,
        Key::Up => 19,
        Key::Backspace => 67,
        Key::Enter => 66,
        Key::Space => 62,
        Key::Tab => 61,
        _ => return None,
    })
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
