# Task 3 report: read-only Linux diagnostics

Status: implemented and committed.

Commit: `dd4b143` — `feat: add read-only Linux doctor diagnostics`.

## Changes

- Replaced the temporary doctor command branch with `doctor::run(json)` and error-to-exit-code handling.
- Added the requested public `DoctorReport`, `Capability`, `DoctorError`, and `run(json: bool) -> Result<(), DoctorError>` interfaces.
- JSON is one object with the six required snake_case fields. Each capability is an object containing `status` (`supported`, `unsupported`, or `unknown`) and `detail`.
- Human output prints six capability rows and labels the Sober Linux x86-64/kernel >= 5.11 comparison baseline. The output explicitly preserves the provisional no-go decision and makes no Roblox or Android runtime support claim.
- Linux probes use compile-time OS/architecture constants, read kernel release and CPU flags from procfs, enumerate DRI device metadata, and inspect standard graphics library file metadata. No external commands, loader execution, device opening, environment access, or system writes occur in doctor.
- Reports include only selected capability facts; no identity, home paths, environment variables, session data, or raw procfs content is emitted.

## Manual checks

Ran `cargo fmt` and `cargo build` to prepare the binary; compilation succeeded with the existing dependencies and no added dependencies. No automated tests were added or run.

Ran `target/debug/drunk doctor` and `target/debug/drunk doctor --json`, both exiting successfully. The human report contained one readable row per required capability. The JSON output parsed successfully with `JSON.parse` in the tool orchestration layer and contained exactly six capability objects.

Observed host facts: Linux x86-64, kernel 7.0, SSE4.1 and SSE4.2 present across reported CPU flags, two DRI nodes, and Vulkan/EGL/GL loader files present. Those are capability hints only. No Roblox client, Android runtime, or alternate executable was launched.

The host exposed every probe, so no runtime `unknown` case occurred during these two command checks. Source review confirmed that missing/unreadable kernel and CPU files, missing/empty CPU flags, missing/unreadable DRI entries, no observed DRI nodes, and absent standard-location loader files produce `Unknown`. No host state was modified to simulate missing probes.

## Self-review

- Inspected all new modules and the main command integration.
- Confirmed probe failures become entries rather than panics, and output/serialization failures return errors.
- Confirmed kernel parsing emits only numeric major/minor values, CPU reporting emits only requested feature booleans, and DRI reporting emits only a node count.
- Confirmed missing files do not imply unsupported graphics or CPU capabilities. Missing CPU flags remain unknown; explicitly reported flags missing SSE features produce unsupported.
- Confirmed graphics loader presence does not claim API compatibility, driver functionality, permissions, or runtime viability.
- The doctor implementation contains only reads, metadata inspection, and stdout output; it invokes no privileged commands and performs no installation, permission modification, instruction emulation, or runtime launch.

## Concerns and limits

- Missing-probe handling was inspected in code but could not be observed on this host without modifying its available probes. No such modification was attempted.
- Graphics loader detection covers common Linux library directories. Other loader locations produce unknown if none of the standard files are observed. A file's presence does not prove it can load or support required API versions.
- No runtime support claim is warranted; the feasibility gate remains provisional no-go.
- Build output left an untracked `target/` directory because the repository has no ignore rule for it. It was not staged or committed.
- Repository Git author identity was unset. The commit used the existing commit identity, `Codex <codex@localhost>`, via command-local options; no Git configuration was changed.

## Final-review fix: CPU minimum vs recommendation

- Changed the CPU comparison status to `supported` when SSE4.1 is present and `unsupported` only when CPU flags are available and SSE4.1 is absent. Unavailable CPU flag probes remain `unknown`.
- CPU detail reports SSE4.1 as the minimum and SSE4.2 separately as recommended, and states that neither establishes Drunk runtime compatibility. The six-field JSON shape and provisional no-go decision are unchanged.
- Limitation: this host reports SSE4.2, so the absent-recommended branch cannot be observed here.

Manual check: `cargo run -- doctor`

Exact command output:

```text
   Compiling drunk v0.1.0 (/home/dream/Desktop/Drunk/.worktrees/runtime-feasibility)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.34s
     Running `target/debug/drunk doctor`
Drunk read-only Linux diagnostics
Comparison baseline: Sober public requirements (Linux x86-64, kernel >= 5.11).
Statuses describe individual hints only; Drunk runtime feasibility remains provisional no-go.
These results do not establish Roblox or Android runtime support.
os                 supported   Linux matches the Sober comparison baseline; no Drunk runtime guarantee.
architecture       supported   x86_64 matches the Sober comparison baseline; no Drunk runtime guarantee.
kernel             supported   Kernel 7.0; Sober comparison baseline is >= 5.11, not a Drunk runtime guarantee.
cpu_features       supported   SSE4.1 minimum: true; SSE4.2 recommended: true across reported CPU flags; comparison only, and neither result establishes Drunk runtime compatibility.
graphics_devices   supported   2 DRI graphics device node(s) observed; permissions, driver functionality, and runtime support are unverified.
graphics_loader    supported   Observed libvulkan.so.1, libEGL.so.1, libGL.so.1; presence only, API versions and driver/runtime functionality are unverified. Drunk runtime feasibility remains provisional no-go.
```
