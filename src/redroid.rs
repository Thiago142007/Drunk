use std::{
    fs,
    io::{BufReader, Read},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use crate::cli::RedroidCommand;
use hibernia::h264::{decoder::Decoder, nal_parser::NalParser};

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
        RedroidCommand::StreamProbe { seconds } => stream_probe(seconds),
        RedroidCommand::Stream { seconds } => stream_view(seconds),
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

fn stream_probe(seconds: u8) -> Result<(), String> {
    if state()?.as_deref() != Some("running") {
        return Err(format!(
            "{NAME} must be running before probing its video stream"
        ));
    }
    validate_managed_container()?;
    connect_adb()?;

    let started = Instant::now();
    let mut child = Command::new("adb")
        .args([
            "-s",
            ADB,
            "shell",
            "-T",
            "screenrecord",
            "--output-format=h264",
            &format!("--time-limit={seconds}"),
            "-",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not start Android screenrecord: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "screenrecord stdout pipe was not created".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "screenrecord stderr pipe was not created".to_owned())?;
    let bytes_read = Arc::new(AtomicU64::new(0));
    let first_byte_ms = Arc::new(AtomicU64::new(u64::MAX));
    let observed_nal_types = Arc::new(AtomicU8::new(0));
    let reader_bytes = Arc::clone(&bytes_read);
    let reader_first_byte = Arc::clone(&first_byte_ms);
    let reader_nal_types = Arc::clone(&observed_nal_types);
    let reader = thread::spawn(move || {
        let mut stdout = stdout;
        let mut buffer = [0u8; 16 * 1024];
        let mut zero_count = 0u8;
        let mut expect_nal_header = false;
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    reader_first_byte
                        .compare_exchange(
                            u64::MAX,
                            started.elapsed().as_millis() as u64,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        )
                        .ok();
                    reader_bytes.fetch_add(count as u64, Ordering::Relaxed);
                    for byte in &buffer[..count] {
                        if expect_nal_header {
                            let mask = match byte & 0x1f {
                                7 => 0b001,     // SPS
                                8 => 0b010,     // PPS
                                1 | 5 => 0b100, // coded picture
                                _ => 0,
                            };
                            reader_nal_types.fetch_or(mask, Ordering::Relaxed);
                            expect_nal_header = false;
                        }
                        if *byte == 0 {
                            zero_count = zero_count.saturating_add(1);
                        } else if *byte == 1 && zero_count >= 2 {
                            expect_nal_header = true;
                            zero_count = 0;
                        } else {
                            zero_count = 0;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });
    let stderr_output = Arc::new(Mutex::new(Vec::new()));
    let reader_stderr_output = Arc::clone(&stderr_output);
    let stderr_reader = thread::spawn(move || {
        let mut stderr = stderr;
        let mut buffer = [0u8; 4096];
        loop {
            match stderr.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    let mut captured = reader_stderr_output.lock().unwrap();
                    let remaining = 16 * 1024usize - captured.len();
                    captured.extend_from_slice(&buffer[..count.min(remaining)]);
                }
            }
        }
    });

    let timeout = Duration::from_secs(u64::from(seconds) + 5);
    let exit_status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                let _ = stderr_reader.join();
                return Err(format!("could not check screenrecord status: {error}"));
            }
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            let _ = stderr_reader.join();
            return Err("screenrecord did not stop within its requested time limit".into());
        }
        thread::sleep(Duration::from_millis(50));
    };
    reader
        .join()
        .map_err(|_| "screenrecord stdout reader stopped unexpectedly".to_owned())?;
    stderr_reader
        .join()
        .map_err(|_| "screenrecord stderr reader stopped unexpectedly".to_owned())?;
    if !exit_status.success() {
        let stderr_bytes = stderr_output.lock().unwrap();
        let diagnostic = String::from_utf8_lossy(&stderr_bytes);
        return Err(format!(
            "screenrecord exited with {exit_status}: {}",
            diagnostic.trim()
        ));
    }

    let bytes = bytes_read.load(Ordering::Relaxed);
    if bytes == 0 {
        return Err("screenrecord exited successfully but returned no H.264 bytes".into());
    }
    if observed_nal_types.load(Ordering::Relaxed) & 0b111 != 0b111 {
        return Err(format!(
            "screenrecord returned {bytes} bytes, but the stream did not contain Annex-B SPS, PPS, and picture NAL units"
        ));
    }
    let first = first_byte_ms.load(Ordering::Relaxed);
    let elapsed = started.elapsed().as_secs_f64();
    let mib_per_second = bytes as f64 / (1024.0 * 1024.0) / elapsed;
    println!(
        "Android H.264 stream received: {bytes} bytes in {elapsed:.2}s ({mib_per_second:.2} MiB/s)"
    );
    if first != u64::MAX {
        println!("time to first byte: {first} ms");
    }
    println!("This probe checks transport only; it does not decode or display video.");
    Ok(())
}

fn stream_view(seconds: u16) -> Result<(), String> {
    if state()?.as_deref() != Some("running") {
        return Err(format!(
            "{NAME} must be running before opening its H.264 stream"
        ));
    }
    validate_managed_container()?;
    connect_adb()?;

    let mut child = Command::new("adb")
        .args([
            "-s",
            ADB,
            "shell",
            "-T",
            "screenrecord",
            "--output-format=h264",
            &format!("--time-limit={seconds}"),
            "-",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not start Android H.264 stream: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "screenrecord stdout pipe was not created".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "screenrecord stderr pipe was not created".to_owned())?;

    let stderr_thread = thread::spawn(move || read_limited(stderr, 16 * 1024));
    let (frame_tx, frame_rx) = crossbeam_channel::bounded(1);
    let stale_frame_rx = frame_rx.clone();
    let (status_tx, status_rx) = std::sync::mpsc::channel();
    let decoder_thread = thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            decode_android_stream(stdout, seconds, &frame_tx, &stale_frame_rx)
        }));
        let message = match outcome {
            Ok(Ok(())) => StreamWorkerMessage::Finished,
            Ok(Err(error)) => StreamWorkerMessage::Error(error),
            Err(payload) => StreamWorkerMessage::Error(format!(
                "Hibernia decoder panicked: {}",
                panic_message(payload)
            )),
        };
        let _ = status_tx.send(message);
    });

    let first_frame = match frame_rx.recv_timeout(Duration::from_secs(20)) {
        Ok(frame) => frame,
        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
            cleanup_stream_child(&mut child);
            let _ = decoder_thread.join();
            let diagnostic = join_stderr(stderr_thread);
            return Err(format!(
                "no decoded H.264 frame arrived within 20 seconds{}",
                if diagnostic.is_empty() {
                    String::new()
                } else {
                    format!("; screenrecord: {diagnostic}")
                }
            ));
        }
        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
            cleanup_stream_child(&mut child);
            let _ = decoder_thread.join();
            let diagnostic = join_stderr(stderr_thread);
            let detail = match status_rx.try_recv() {
                Ok(StreamWorkerMessage::Error(error)) => error,
                _ if !diagnostic.is_empty() => diagnostic,
                _ => "the decoder stopped before producing a frame".to_owned(),
            };
            return Err(detail);
        }
    };

    let source_width = first_frame.width;
    let source_height = first_frame.height;
    let scale = (720.0 / source_width as f32)
        .min(1280.0 / source_height as f32)
        .min(1.0);
    let width = ((source_width as f32 * scale).round() as usize).max(1);
    let height = ((source_height as f32 * scale).round() as usize).max(1);
    let mut buffer = resize_rgb_nearest(&first_frame, width, height);
    let mut window = match minifb::Window::new(
        "Drunk [EXPERIMENTAL • H.264 / Rust] — Android preview (Esc closes)",
        width,
        height,
        minifb::WindowOptions::default(),
    ) {
        Ok(window) => window,
        Err(error) => {
            cleanup_stream_child(&mut child);
            let _ = decoder_thread.join();
            let _ = join_stderr(stderr_thread);
            return Err(format!("could not create H.264 preview window: {error}"));
        }
    };
    window.set_target_fps(60);
    println!(
        "Rust H.264 preview opened ({source_width}×{source_height}); screenrecord stops after {seconds}s. Press Esc to close."
    );

    let display_result = (|| -> Result<bool, String> {
        while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
            if let Ok(frame) = frame_rx.try_recv() {
                if frame.width != source_width || frame.height != source_height {
                    return Err("Android video dimensions changed during the stream".into());
                }
                buffer = resize_rgb_nearest(&frame, width, height);
            }
            window
                .update_with_buffer(&buffer, width, height)
                .map_err(|error| format!("could not update H.264 preview window: {error}"))?;
            match status_rx.try_recv() {
                Ok(StreamWorkerMessage::Error(error)) => return Err(error),
                Ok(StreamWorkerMessage::Finished) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
            if let Some(status) = child
                .try_wait()
                .map_err(|error| format!("could not check screenrecord status: {error}"))?
            {
                if !status.success() {
                    return Err(format!("ADB screenrecord exited with {status}"));
                }
                break;
            }
        }
        Ok(!window.is_open() || window.is_key_down(minifb::Key::Escape))
    })();

    cleanup_stream_child(&mut child);
    let decoder_result = decoder_thread
        .join()
        .map_err(|_| "H.264 decoder worker thread did not exit cleanly".to_owned());
    let worker_message = status_rx.try_recv();
    let diagnostic = join_stderr(stderr_thread);
    let user_closed = display_result?;
    decoder_result?;
    match worker_message {
        Ok(StreamWorkerMessage::Error(_)) if user_closed => {}
        Ok(StreamWorkerMessage::Error(error)) if !diagnostic.is_empty() => {
            return Err(format!("{error}; screenrecord: {diagnostic}"));
        }
        Ok(StreamWorkerMessage::Error(error)) => return Err(error),
        Ok(StreamWorkerMessage::Finished) | Err(std::sync::mpsc::TryRecvError::Empty) => {}
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            return Err("H.264 decoder worker stopped without a status message".into());
        }
    }
    if !diagnostic.is_empty() {
        println!("screenrecord: {diagnostic}");
    }
    Ok(())
}

enum StreamWorkerMessage {
    Finished,
    Error(String),
}

struct RgbFrame {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}

fn decode_android_stream(
    stdout: std::process::ChildStdout,
    seconds: u16,
    frames: &crossbeam_channel::Sender<RgbFrame>,
    stale_frame_rx: &crossbeam_channel::Receiver<RgbFrame>,
) -> Result<(), String> {
    let reader = BufReader::with_capacity(128 * 1024, stdout);
    let mut parser = NalParser::new(reader);
    let mut decoder = Decoder::new();
    let mut decoded_frames = 0usize;
    while let Some(nal) = parser.next() {
        let nal = nal.map_err(|error| format!("could not read H.264 NAL unit: {error}"))?;
        decoder
            .decode(&nal)
            .map_err(|error| format!("Hibernia could not decode H.264 NAL unit: {error:?}"))?;
        while let Some(frame) = decoder.retrieve_frame() {
            decoded_frames += 1;
            let frame = rgb_from_yuv420(&frame)?;
            publish_latest_frame(frames, stale_frame_rx, frame);
        }
    }
    decoder
        .flush()
        .map_err(|error| format!("Hibernia could not flush the last frame: {error:?}"))?;
    while let Some(frame) = decoder.retrieve_frame() {
        decoded_frames += 1;
        let frame = rgb_from_yuv420(&frame)?;
        publish_latest_frame(frames, stale_frame_rx, frame);
    }
    if decoded_frames == 0 {
        return Err(format!(
            "screenrecord ended without a decoded picture within its {seconds}s time limit"
        ));
    }
    Ok(())
}

fn publish_latest_frame(
    sender: &crossbeam_channel::Sender<RgbFrame>,
    stale_frame_rx: &crossbeam_channel::Receiver<RgbFrame>,
    frame: RgbFrame,
) {
    match sender.try_send(frame) {
        Ok(()) => {}
        Err(crossbeam_channel::TrySendError::Full(frame)) => {
            let _ = stale_frame_rx.try_recv();
            let _ = sender.try_send(frame);
        }
        Err(crossbeam_channel::TrySendError::Disconnected(_)) => {}
    }
}

fn rgb_from_yuv420(frame: &hibernia::h264::decoder::VideoFrame) -> Result<RgbFrame, String> {
    let y = &frame.planes[0];
    let u = &frame.planes[1];
    let v = &frame.planes[2];
    let width = y.cfg.width;
    let height = y.cfg.height;
    if width == 0 || height == 0 || u.cfg.width == 0 || v.cfg.width == 0 {
        return Err("Hibernia produced an empty YUV frame".into());
    }
    let capacity = width
        .checked_mul(height)
        .ok_or_else(|| "decoded frame dimensions exceed supported buffer size".to_owned())?;
    let mut pixels = Vec::with_capacity(capacity);
    for out_y in 0..height {
        let y_row = (y.cfg.yorigin + out_y) * y.cfg.stride;
        let u_row = (u.cfg.yorigin + out_y / 2) * u.cfg.stride;
        let v_row = (v.cfg.yorigin + out_y / 2) * v.cfg.stride;
        for out_x in 0..width {
            let luminance = i32::from(y.data[y_row + y.cfg.xorigin + out_x]);
            let cb = i32::from(u.data[u_row + u.cfg.xorigin + out_x / 2]) - 128;
            let cr = i32::from(v.data[v_row + v.cfg.xorigin + out_x / 2]) - 128;
            let c = (luminance - 16).max(0);
            let red = ((298 * c + 409 * cr + 128) >> 8).clamp(0, 255) as u32;
            let green = ((298 * c - 100 * cb - 208 * cr + 128) >> 8).clamp(0, 255) as u32;
            let blue = ((298 * c + 516 * cb + 128) >> 8).clamp(0, 255) as u32;
            pixels.push((red << 16) | (green << 8) | blue);
        }
    }
    Ok(RgbFrame {
        width,
        height,
        pixels,
    })
}

fn resize_rgb_nearest(frame: &RgbFrame, width: usize, height: usize) -> Vec<u32> {
    let mut output = Vec::with_capacity(width * height);
    for y in 0..height {
        let source_y = y * frame.height / height;
        for x in 0..width {
            let source_x = x * frame.width / width;
            output.push(frame.pixels[source_y * frame.width + source_x]);
        }
    }
    output
}

fn cleanup_stream_child(child: &mut std::process::Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn read_limited(mut reader: impl Read, limit: usize) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(count) => {
                let remaining = limit.saturating_sub(output.len());
                output.extend_from_slice(&buffer[..count.min(remaining)]);
            }
        }
    }
    output
}

fn join_stderr(thread: thread::JoinHandle<Vec<u8>>) -> String {
    thread
        .join()
        .map(|output| String::from_utf8_lossy(&output).trim().to_owned())
        .unwrap_or_else(|_| "could not collect screenrecord diagnostics".to_owned())
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else {
        "unknown panic payload".to_owned()
    }
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
