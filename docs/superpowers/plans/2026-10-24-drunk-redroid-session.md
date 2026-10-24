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
