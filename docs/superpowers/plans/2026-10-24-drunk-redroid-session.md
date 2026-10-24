# Drunk Android Session Vertical Slice

## Decision and scope

Continue the redroid external-Android prototype as a supervised runtime, while
keeping Drunk-owned code in Rust. This is a deliberate prototype direction; it
does not approve redroid as the distributable product backend or claim Roblox
compatibility. The next vertical slice combines the already demonstrated H.264
viewer and Android 13 playback audio in one command.

## Implementation steps

1. Add `drunk redroid session [--seconds N]` with an explicit finite capture
   limit and CLI help. Keep the existing separate `stream` and `audio` commands
   for diagnostics.
2. Start the scrcpy playback bridge as a child process, run the Rust H.264
   viewer concurrently, then always stop and reap the audio child when the
   viewer closes, reaches its limit, or returns an error.
3. Add focused CLI/unit coverage for the new command and its defaults. Run the
   Rust format, test, release-build and diff checks.
4. Exercise the combined session against the authorized local Android 13
   sample runtime, confirm video and audio overlap, then stop the temporary
   container while preserving its data volume.

## Acceptance and limits

- Closing the preview or reaching the requested duration stops the audio
  helper; repeated session runs do not leave scrcpy children behind.
- A test sample can render in the Rust window while its playback is audible on
  the selected host sink.
- State measured here is a prototype result only. Continuous game controls,
  performance for games, graphics acceleration, graceful Android shutdown,
  licensing/distribution and Roblox behavior remain separate gates.

## Result (2026-10-24)

Implemented `drunk redroid session --seconds N` with a 30-second default and
1–180-second range. The Android 13 session ran for 15 seconds, rendered the
384×672 H.264 preview, and forwarded the Native Audio sample concurrently.
During HELLO, HDMI monitoring recorded 41,088 nonzero samples in three seconds;
the user confirmed hearing it. The session returned success at its time limit,
and no scrcpy server/client process remained. The data volume and image remain
available; the managed container was stopped after the test.

## Follow-up: preview-rate baseline (2026-10-24)

The viewer now reports new frames presented to its window and their average
updates per second when a session ends. This measures Rust preview updates, not
the Android app's rendered FPS, input-to-display latency, or audio latency.
Only distinct frames received from the bounded latest-frame channel are
counted; frames dropped before presentation are not counted.

With Hello GL2 active, one 30-second Android 13 release session presented 449
frames in 29.94 seconds (15.00 updates/s). `/usr/bin/time -v` measured 5.52 s
user CPU and 1.14 s system CPU over 31.31 s wall time (21% of one logical CPU)
and 78,040 KiB maximum RSS for the command and its children. The run exited 0;
the container was then stopped and Docker reported exit 137, preserving its
volume and image. This is a single-run baseline, not an acceptance threshold or
a product-performance claim.

## Follow-up: audio-child cleanup on early return (2026-10-24)

Started a 120-second session and confirmed both scrcpy and Android
`screenrecord` were active. The available computer-use bridge did not expose
native windows (`getState()` returned no apps and `listWindows` was unavailable),
so closing the preview through its window controls could not be exercised.
Instead, a controlled termination of Android `screenrecord` made the viewer
return with exit status 143. The session returned the corresponding ADB error,
and a process check confirmed that the host scrcpy child had also been reaped.
The redroid container was stopped after the check (Docker exit 137; volume and
image preserved). This verifies audio-child cleanup on a viewer error path, but
does not prove cleanup after the user closes the window; that check remains
open.
