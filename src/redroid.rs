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
    "redroid/redroid@sha256:36d6d21bcf7e92d78eabaa6f1748e5cf0e9fb15176c0091794168be012c5c22e";
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
            let exit_code = docker(&[
                "container",
                "inspect",
                "--format",
                "{{.State.ExitCode}}",
                NAME,
            ])?
            .trim()
            .parse::<i32>()
            .map_err(|error| format!("could not parse {NAME} exit code after stopping: {error}"))?;
            println!("{}", stopped_summary(exit_code));
            Ok(())
        }
        RedroidCommand::Demo => demo(),
        RedroidCommand::View => view(),
        RedroidCommand::StreamProbe { seconds } => stream_probe(seconds),
        RedroidCommand::Stream { seconds } => stream_view(seconds),
        RedroidCommand::Audio => audio_bridge(),
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
        validate_managed_container()?;
        if state == "running" {
            println!("{NAME}: already running");
            return Ok(());
        }
        if !stopped_container_can_be_recreated(&state) {
            return Err(format!(
                "{NAME} is in state {state}; only exited, dead, or created managed containers can be recreated automatically"
            ));
        }
        docker(&["container", "rm", NAME])?;
        println!("{NAME}: removed stopped container; reusing its Android data volume and image");
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
    let (width, height) = preview_dimensions(source_width as usize, source_height as usize);
    let width = width as u32;
    let height = height as u32;
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
    println!(
        "Android preview opened ({source_width}×{source_height} source, {width}×{height} window); input is sent over ADB."
    );

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

    let device_frame = capture_frame(ADB)?;
    let (stream_width, stream_height) = screenrecord_dimensions(
        device_frame.width() as usize,
        device_frame.height() as usize,
    );
    let stream_size = format!("{stream_width}x{stream_height}");

    let mut child = Command::new("adb")
        .args([
            "-s",
            ADB,
            "shell",
            "-T",
            "screenrecord",
            "--size",
            &stream_size,
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

    let source_width = first_frame.source_width;
    let source_height = first_frame.source_height;
    let android_width = device_frame.width() as usize;
    let android_height = device_frame.height() as usize;
    let width = first_frame.width;
    let height = first_frame.height;
    let mut buffer = first_frame.pixels;
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
        "Rust H.264 preview opened ({source_width}×{source_height} source, {width}×{height} window); click sends Android taps and supported keys are forwarded; screenrecord stops after {seconds}s. Press Esc to close."
    );

    let mut received_worker_message = None;
    let mut previous_mouse_down = false;
    let display_result = (|| -> Result<bool, String> {
        while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
            if let Ok(frame) = frame_rx.try_recv() {
                if frame.source_width != source_width || frame.source_height != source_height {
                    return Err("Android video dimensions changed during the stream".into());
                }
                buffer = frame.pixels;
            }
            window
                .update_with_buffer(&buffer, width, height)
                .map_err(|error| format!("could not update H.264 preview window: {error}"))?;
            let mouse_down = window.get_mouse_down(minifb::MouseButton::Left);
            if mouse_down && !previous_mouse_down {
                if let Some((x, y)) = window.get_mouse_pos(minifb::MouseMode::Discard) {
                    let (android_x, android_y) =
                        map_preview_to_android(x, y, width, height, android_width, android_height);
                    let x = android_x.to_string();
                    let y = android_y.to_string();
                    send_adb_input(ADB, &["shell", "input", "tap", &x, &y])?;
                }
            }
            previous_mouse_down = mouse_down;
            for key in window.get_keys_pressed(minifb::KeyRepeat::No) {
                if key == minifb::Key::Escape {
                    continue;
                }
                if let Some(keycode) = android_keycode(key) {
                    let keycode = keycode.to_string();
                    send_adb_input(ADB, &["shell", "input", "keyevent", &keycode])?;
                }
            }
            match status_rx.try_recv() {
                Ok(StreamWorkerMessage::Error(error)) => return Err(error),
                Ok(message @ StreamWorkerMessage::Finished) => {
                    received_worker_message = Some(message);
                    break;
                }
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
    let worker_message = received_worker_message.or_else(|| worker_message.ok());
    match worker_message {
        Some(StreamWorkerMessage::Error(_)) if user_closed => {}
        Some(StreamWorkerMessage::Error(error)) if !diagnostic.is_empty() => {
            return Err(format!("{error}; screenrecord: {diagnostic}"));
        }
        Some(StreamWorkerMessage::Error(error)) => return Err(error),
        Some(StreamWorkerMessage::Finished) => {}
        None => {
            return Err("H.264 decoder worker stopped without a status message".into());
        }
    }
    if !diagnostic.is_empty() {
        println!("screenrecord: {diagnostic}");
    }
    Ok(())
}

fn audio_bridge() -> Result<(), String> {
    if state()?.as_deref() != Some("running") {
        return Err(format!(
            "{NAME} must be running before forwarding Android audio"
        ));
    }
    validate_managed_container()?;
    println!("Forwarding Android output audio through scrcpy; press Ctrl+C to stop.");
    let status = Command::new("scrcpy")
        .args(scrcpy_audio_args())
        .status()
        .map_err(|error| {
            format!(
                "could not execute scrcpy; install scrcpy and ensure it is available in PATH: {error}"
            )
        })?;
    if !status.success() {
        return Err(format!("scrcpy audio bridge exited with {status}"));
    }
    Ok(())
}

fn scrcpy_audio_args() -> [&'static str; 9] {
    [
        "--serial",
        ADB,
        "--no-control",
        "--no-window",
        "--max-size=320",
        "--record=/dev/null",
        "--record-format=mkv",
        "--audio-source=playback",
        "--require-audio",
    ]
}

enum StreamWorkerMessage {
    Finished,
    Error(String),
}

struct RgbFrame {
    width: usize,
    height: usize,
    source_width: usize,
    source_height: usize,
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
    let mut initial_frame_decoder = InitialFrameDecoder::new();
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
        if let Some(frame) = initial_frame_decoder.decode_until_idr(&nal)? {
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

struct InitialFrameDecoder {
    decoder: Option<Decoder>,
}

impl InitialFrameDecoder {
    fn new() -> Self {
        Self {
            decoder: Some(Decoder::new()),
        }
    }

    fn decode_until_idr(
        &mut self,
        nal: &[u8],
    ) -> Result<Option<hibernia::h264::decoder::VideoFrame>, String> {
        let Some(decoder) = self.decoder.as_mut() else {
            return Ok(None);
        };
        decoder
            .decode(nal)
            .map_err(|error| format!("Hibernia could not decode initial H.264 frame: {error:?}"))?;
        if nal.first().is_some_and(|header| header & 0x1f == 5) {
            decoder.flush().map_err(|error| {
                format!("Hibernia could not flush initial H.264 frame: {error:?}")
            })?;
            let frame = decoder.retrieve_frame();
            self.decoder = None;
            return Ok(frame);
        }
        Ok(None)
    }
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

fn preview_dimensions(source_width: usize, source_height: usize) -> (usize, usize) {
    const MAX_WIDTH: f32 = 540.0;
    const MAX_HEIGHT: f32 = 680.0;

    let scale = (MAX_WIDTH / source_width as f32)
        .min(MAX_HEIGHT / source_height as f32)
        .min(1.0);
    (
        ((source_width as f32 * scale).round() as usize).max(1),
        ((source_height as f32 * scale).round() as usize).max(1),
    )
}

fn map_preview_to_android(
    x: f32,
    y: f32,
    preview_width: usize,
    preview_height: usize,
    android_width: usize,
    android_height: usize,
) -> (u32, u32) {
    let android_x = (x / preview_width.max(1) as f32 * android_width as f32)
        .clamp(0.0, android_width.saturating_sub(1) as f32) as u32;
    let android_y = (y / preview_height.max(1) as f32 * android_height as f32)
        .clamp(0.0, android_height.saturating_sub(1) as f32) as u32;
    (android_x, android_y)
}

fn screenrecord_dimensions(source_width: usize, source_height: usize) -> (usize, usize) {
    let (width, height) = preview_dimensions(source_width, source_height);
    (
        align_video_dimension(width, 540),
        align_video_dimension(height, 680),
    )
}

fn align_video_dimension(dimension: usize, maximum: usize) -> usize {
    const MACROBLOCK: usize = 16;
    let lower = (dimension / MACROBLOCK) * MACROBLOCK;
    let upper = lower.saturating_add(MACROBLOCK);
    let aligned = if dimension.saturating_sub(lower) > MACROBLOCK / 2 && upper <= maximum {
        upper
    } else {
        lower
    };
    aligned.max(MACROBLOCK)
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
    let (preview_width, preview_height) = preview_dimensions(width, height);
    let capacity = preview_width
        .checked_mul(preview_height)
        .ok_or_else(|| "decoded preview dimensions exceed supported buffer size".to_owned())?;
    let y_plane = YuvPlane {
        data: &y.data,
        stride: y.cfg.stride,
        xorigin: y.cfg.xorigin,
        yorigin: y.cfg.yorigin,
    };
    let u_plane = YuvPlane {
        data: &u.data,
        stride: u.cfg.stride,
        xorigin: u.cfg.xorigin,
        yorigin: u.cfg.yorigin,
    };
    let v_plane = YuvPlane {
        data: &v.data,
        stride: v.cfg.stride,
        xorigin: v.cfg.xorigin,
        yorigin: v.cfg.yorigin,
    };
    let pixels = yuv420_to_rgb_preview(
        &y_plane,
        &u_plane,
        &v_plane,
        width,
        height,
        preview_width,
        preview_height,
    );
    debug_assert_eq!(pixels.len(), capacity);
    Ok(RgbFrame {
        width: preview_width,
        height: preview_height,
        source_width: width,
        source_height: height,
        pixels,
    })
}

#[derive(Clone, Copy)]
struct YuvPlane<'a> {
    data: &'a [u8],
    stride: usize,
    xorigin: usize,
    yorigin: usize,
}

fn yuv420_to_rgb_preview(
    y: &YuvPlane<'_>,
    u: &YuvPlane<'_>,
    v: &YuvPlane<'_>,
    source_width: usize,
    source_height: usize,
    output_width: usize,
    output_height: usize,
) -> Vec<u32> {
    let mut output = Vec::with_capacity(output_width.saturating_mul(output_height));
    for output_y in 0..output_height {
        let source_y = output_y * source_height / output_height;
        let y_row = (y.yorigin + source_y) * y.stride + y.xorigin;
        let chroma_y = source_y / 2;
        let u_row = (u.yorigin + chroma_y) * u.stride + u.xorigin;
        let v_row = (v.yorigin + chroma_y) * v.stride + v.xorigin;
        for output_x in 0..output_width {
            let source_x = output_x * source_width / output_width;
            let luminance = i32::from(y.data[y_row + source_x]);
            let cb = i32::from(u.data[u_row + source_x / 2]) - 128;
            let cr = i32::from(v.data[v_row + source_x / 2]) - 128;
            let c = (luminance - 16).max(0);
            let red = ((298 * c + 409 * cr + 128) >> 8).clamp(0, 255) as u32;
            let green = ((298 * c - 100 * cb - 208 * cr + 128) >> 8).clamp(0, 255) as u32;
            let blue = ((298 * c + 516 * cb + 128) >> 8).clamp(0, 255) as u32;
            output.push((red << 16) | (green << 8) | blue);
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
    if let Some(rgba) = frame.as_rgba8() {
        return rgba_to_window_buffer_nearest(rgba, width, height);
    }
    let rgba = frame.to_rgba8();
    rgba_to_window_buffer_nearest(&rgba, width, height)
}

fn rgba_to_window_buffer_nearest(frame: &image::RgbaImage, width: u32, height: u32) -> Vec<u32> {
    let source_width = frame.width() as usize;
    let source_height = frame.height() as usize;
    let width = width as usize;
    let height = height as usize;
    let source = frame.as_raw();
    let mut output = Vec::with_capacity(width.saturating_mul(height));

    for y in 0..height {
        let source_y = y * source_height / height.max(1);
        for x in 0..width {
            let source_x = x * source_width / width.max(1);
            let offset = (source_y * source_width + source_x) * 4;
            output.push(
                (u32::from(source[offset]) << 16)
                    | (u32::from(source[offset + 1]) << 8)
                    | u32::from(source[offset + 2]),
            );
        }
    }
    output
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

fn stopped_container_can_be_recreated(state: &str) -> bool {
    matches!(state, "exited" | "dead" | "created")
}

fn stopped_summary(exit_code: i32) -> String {
    if exit_code == 137 {
        format!(
            "{NAME}: stopped after Docker forced SIGKILL (exit 137); data volume and image preserved"
        )
    } else {
        format!(
            "{NAME}: stopped (container exit code {exit_code}); data volume and image preserved"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InitialFrameDecoder, YuvPlane, make_window_buffer, map_preview_to_android,
        preview_dimensions, scrcpy_audio_args, screenrecord_dimensions,
        stopped_container_can_be_recreated, stopped_summary, yuv420_to_rgb_preview,
    };
    use image::{DynamicImage, RgbaImage};
    use std::io::Cursor;

    #[test]
    fn screenshot_preview_uses_nearest_source_pixel_without_blending() {
        let image = RgbaImage::from_raw(
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )
        .unwrap();

        assert_eq!(
            make_window_buffer(&DynamicImage::ImageRgba8(image), 1, 1),
            vec![0x00ff_0000]
        );
    }

    #[test]
    fn yuv_preview_converts_only_the_sampled_output_pixel() {
        let y_data = [100, 235, 16, 16];
        let u_data = [90];
        let v_data = [180];
        let y = YuvPlane {
            data: &y_data,
            stride: 2,
            xorigin: 0,
            yorigin: 0,
        };
        let u = YuvPlane {
            data: &u_data,
            stride: 1,
            xorigin: 0,
            yorigin: 0,
        };
        let v = YuvPlane {
            data: &v_data,
            stride: 1,
            xorigin: 0,
            yorigin: 0,
        };

        assert_eq!(
            yuv420_to_rgb_preview(&y, &u, &v, 2, 2, 1, 1),
            vec![0x00b5_4615]
        );
    }

    #[test]
    fn static_h264_stream_exposes_its_initial_idr_frame_without_waiting_for_another_picture() {
        let bitstream = include_bytes!("../tests/fixtures/native-audio-static.h264");
        let mut parser = hibernia::h264::nal_parser::NalParser::new(Cursor::new(&bitstream[..]));
        let mut initial_frame_decoder = InitialFrameDecoder::new();

        let first_frame = parser.find_map(|nal| {
            let nal = nal.expect("captured static Android H.264 NALs parse");
            initial_frame_decoder
                .decode_until_idr(&nal)
                .expect("captured baseline IDR decodes")
        });

        let first_frame = first_frame.expect("the initial IDR should be displayed immediately");
        assert_eq!(first_frame.planes[0].cfg.width, 720);
        assert_eq!(first_frame.planes[0].cfg.height, 1280);
    }

    #[test]
    fn screenrecord_sized_static_stream_exposes_its_initial_idr_frame() {
        let bitstream = include_bytes!("../tests/fixtures/native-audio-static-384x672.h264");
        let mut parser = hibernia::h264::nal_parser::NalParser::new(Cursor::new(&bitstream[..]));
        let mut initial_frame_decoder = InitialFrameDecoder::new();

        let first_frame = parser.find_map(|nal| {
            let nal = nal.expect("captured scaled H.264 NALs parse");
            initial_frame_decoder
                .decode_until_idr(&nal)
                .expect("captured scaled baseline IDR decodes")
        });

        let first_frame = first_frame.expect("the scaled initial IDR should be displayed");
        assert_eq!(first_frame.planes[0].cfg.width, 384);
        assert_eq!(first_frame.planes[0].cfg.height, 672);
    }

    #[test]
    fn scrcpy_audio_bridge_uses_playback_capture_without_showing_a_second_window() {
        assert_eq!(
            scrcpy_audio_args(),
            [
                "--serial",
                "127.0.0.1:5555",
                "--no-control",
                "--no-window",
                "--max-size=320",
                "--record=/dev/null",
                "--record-format=mkv",
                "--audio-source=playback",
                "--require-audio",
            ]
        );
    }

    #[test]
    fn preview_dimensions_fit_portrait_android_to_host_height() {
        assert_eq!(preview_dimensions(720, 1280), (383, 680));
    }

    #[test]
    fn preview_dimensions_preserve_landscape_aspect_ratio() {
        assert_eq!(preview_dimensions(1920, 1080), (540, 304));
    }

    #[test]
    fn preview_dimensions_do_not_upscale_small_frames() {
        assert_eq!(preview_dimensions(320, 240), (320, 240));
    }

    #[test]
    fn h264_preview_coordinates_map_back_to_full_android_resolution() {
        assert_eq!(
            map_preview_to_android(192.0, 336.0, 384, 672, 720, 1280),
            (360, 640)
        );
        assert_eq!(
            map_preview_to_android(383.9, 671.9, 384, 672, 720, 1280),
            (719, 1279)
        );
    }

    #[test]
    fn screenrecord_dimensions_fit_the_window_and_are_encoder_aligned() {
        assert_eq!(screenrecord_dimensions(720, 1280), (384, 672));
        assert_eq!(screenrecord_dimensions(1920, 1080), (528, 304));
        assert_eq!(screenrecord_dimensions(320, 240), (320, 240));
    }

    #[test]
    fn stopped_container_recreation_is_limited_to_inactive_states() {
        assert!(stopped_container_can_be_recreated("exited"));
        assert!(stopped_container_can_be_recreated("dead"));
        assert!(stopped_container_can_be_recreated("created"));
        assert!(!stopped_container_can_be_recreated("running"));
        assert!(!stopped_container_can_be_recreated("paused"));
    }

    #[test]
    fn stop_summary_discloses_docker_forced_kill() {
        assert_eq!(
            stopped_summary(137),
            "drunk-redroid: stopped after Docker forced SIGKILL (exit 137); data volume and image preserved"
        );
    }

    #[test]
    fn stop_summary_reports_other_container_exit_codes() {
        assert_eq!(
            stopped_summary(0),
            "drunk-redroid: stopped (container exit code 0); data volume and image preserved"
        );
    }
}
