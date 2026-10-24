# Drunk Runtime Feasibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish with evidence whether a Rust-owned Linux runtime can start an unmodified official Android Roblox client, and produce the first Rust CLI foundation for environment diagnostics.

**Architecture:** This is the first sub-project, limited to the feasibility gate in the approved design. It separates a documented runtime decision from a small Linux-only Rust `doctor` command; it does not implement a full Android compatibility runtime or claim Roblox compatibility. After the gate, the selected backend will be recorded in an ADR and a second plan will cover the launcher, package handling, desktop integration, and distribution.

**Tech Stack:** Rust stable, Cargo, Linux x86-64, `clap` for CLI parsing, `serde`/`serde_json` for machine-readable reports, and `thiserror` for errors. Runtime candidates and their dependencies must be evaluated from primary documentation and licenses before inclusion.

**Spec:** `docs/superpowers/specs/2026-10-24-drunk-design.md`

## Global Constraints

- All code authored and maintained by Drunk is Rust.
- Target Linux desktop x86-64 first.
- The Roblox client and Android components are external dependencies; do not modify or redistribute proprietary binaries without authorization.
- Do not bypass anti-cheat, integrity checks, Android Remote Attestation, DRM, or platform controls.
- Do not promise compatibility with all games, GPUs, Linux distributions, ARM, VR, or virtual machines.
- Flatpak is a candidate, not an already-fixed architecture decision.
- Runtime feasibility is a blocker: stop before building the product runtime if no documented and sustainable integration path is demonstrated.
- No automated tests are included in this first plan; add or run them only if requested by the user.

## Review Focus

- Missing or incompatible CPU/GPU/kernel support must produce an actionable diagnostic instead of a crash — manual check: run `drunk doctor` on x86-64 and non-x86-64 or with missing GPU information, and confirm each unavailable capability is reported distinctly.
- Runtime/API incompatibility must not be misreported as Roblox support — manual check: inspect the feasibility report and verify it names the exact tested Android app, runtime, host, and result.
- Package integrity and provenance must remain explicit — manual check: verify the report does not imply that Drunk can redistribute an APK unless its source and rights are established.
- Authentication or session information must not appear in logs — manual check: review emitted diagnostics and confirm no credential/session fields are read or printed.
- Runtime security boundaries must remain unmodified — manual check: confirm the candidate evaluation contains no steps to evade integrity checks or falsify device attestation.

---

## File Structure

- `Cargo.toml` — package metadata and pinned direct dependency ranges.
- `src/main.rs` — CLI entry point and command dispatch.
- `src/cli.rs` — CLI argument and output format definitions.
- `src/doctor/mod.rs` — host capability collection and report assembly.
- `src/doctor/linux.rs` — Linux kernel, CPU architecture/features, GPU device, and graphics loader probes.
- `src/doctor/report.rs` — stable report model and human/JSON rendering.
- `docs/runtime-evaluation.md` — candidate comparison, evidence, limitations, and go/no-go decision.
- `docs/adr/0001-android-runtime-backend.md` — selected backend decision or documented no-go.

## Task 1: Establish the runtime feasibility decision

**Files:**
- Create: `docs/runtime-evaluation.md`
- Create: `docs/adr/0001-android-runtime-backend.md`

**Interfaces:**
- Consumes: approved design in `docs/superpowers/specs/2026-10-24-drunk-design.md`; official Sober and Android runtime documentation.
- Produces: a documented choice of backend or an explicit no-go, with a reproducible manual validation protocol.

- [ ] **Step 1: Record the boundary of public Sober evidence**

  Document that Sober's public GitHub repository is an issue tracker and that its FAQ/site describe an Android-client runtime for Linux. Separate confirmed statements from assumptions; do not claim knowledge of private internals.

- [ ] **Step 2: Compare candidate runtime models**

  Compare only primary-source-documented options. At minimum include a custom Linux adapter consuming Android components, Waydroid as a reference/integration candidate, and a full Android VM/container approach. Waydroid's official docs describe a full Android system running in a Linux container using namespaces; assess that model as a reference or possible external backend and state clearly whether selecting it would amount to changing the approved option B. For each, record ownership boundary, x86-64 support, graphics/window path, input/audio path, privilege/kernel requirements, release health, licensing/distribution constraints, and whether it fits option B. Reference: [Waydroid documentation](https://docs.waydro.id/).

- [ ] **Step 3: Define a manual compatibility protocol**

  Specify a non-Roblox Android test app that can be lawfully used; record host distro/kernel, CPU flags, GPU/driver, runtime/image version, API level, graphics backend, input, audio, launch/exit behavior, and logs. Define a separate later Roblox validation that uses the unmodified official app and never attempts to evade security checks.

- [ ] **Step 4: Decide go/no-go and write the ADR**

  Choose an integration only if primary documentation, license terms, and a manual app launch demonstrate a maintainable fit. If the only viable path is simply launching Waydroid as-is, explain that it conflicts with the approved option B and request a new design decision instead of silently switching architecture. If no path is demonstrated, record a no-go and do not start the product runtime.

- [ ] **Step 5: Review the evidence in both documents**

  Confirm every claim has a source or is explicitly labeled inference, and that unresolved risks and the decision are visible at the top of the ADR.

**Acceptance:** A reader can reproduce the investigation, see whether B is technically viable, and understand the selected integration or why work must pause.

## Task 2: Create the Rust CLI foundation

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/cli.rs`

**Interfaces:**
- Produces: `enum Command { Doctor { json: bool } }`; `clap` provides `--version` from package metadata. Until Task 3 supplies the module, `main()` accepts `Doctor`, prints `doctor implementation pending` to stderr, and exits with code 2.

- [ ] **Step 1: Create the Cargo package**

  Use crate/package name `drunk`, Rust edition 2024, and a binary target named `drunk`. Include only the planned direct dependencies: `clap` derive, `serde` derive, `serde_json`, and `thiserror`. Add no GTK/UI/runtime dependency in this step.

- [ ] **Step 2: Define `Command` and parse arguments in `src/cli.rs`**

  Provide `drunk doctor [--json]`; configure the root Clap command with `version = env!("CARGO_PKG_VERSION")` so `drunk --version` is generated automatically. Unknown commands/options should be reported by Clap with a non-zero exit code.

- [ ] **Step 3: Implement dispatch in `src/main.rs`**

  Route `Doctor` to a temporary `doctor implementation pending` message on stderr and exit code 2; Task 3 replaces this branch with `doctor::run(json)`. Keep stdout reserved for reports and rely on Clap for `--version`.

- [ ] **Step 4: Confirm the CLI surface manually**

  Run `cargo run -- --help`, `cargo run -- --version`, and `cargo run -- doctor --help`; confirm command names and unsupported invocation behavior match the interface above. Confirm `doctor` reports that its implementation is pending until Task 3.

**Acceptance:** The Rust binary exposes the agreed command surface and has an explicit temporary response for `doctor` without starting an Android runtime or touching user session data.

## Task 3: Implement read-only Linux diagnostics

**Files:**
- Create: `src/doctor/mod.rs`
- Create: `src/doctor/linux.rs`
- Create: `src/doctor/report.rs`
- Modify: `src/main.rs`

**Interfaces:**
- `pub fn run(json: bool) -> Result<(), DoctorError>`
- `pub struct DoctorReport { pub os: Capability, pub architecture: Capability, pub kernel: Capability, pub cpu_features: Capability, pub graphics_devices: Capability, pub graphics_loader: Capability }`
- `pub enum Capability { Supported { detail: String }, Unsupported { detail: String }, Unknown { detail: String } }`

- [ ] **Step 1: Define the report types and stable JSON shape**

  Implement `Capability` and `DoctorReport`; serialize using snake_case fields. Do not include username, home-directory paths, environment variables, cookies, account identity, or session data.

- [ ] **Step 2: Collect architecture and kernel details**

  Read `std::env::consts::ARCH` and `/proc/sys/kernel/osrelease`. Report Linux x86-64 and kernel >= 5.11 as the reference baseline from Sober's public requirements, clearly labeled as a comparison baseline rather than a Drunk guarantee. If a file cannot be read, report `Unknown`.

- [ ] **Step 3: Collect CPU and graphics capability hints**

  Parse `/proc/cpuinfo` for `sse4_1` and `sse4_2`; inspect `/dev/dri` without changing permissions; detect graphics loader presence without invoking privileged commands. Report `Unknown` rather than inferring support from missing files. Do not emulate instructions or load a different client binary.

- [ ] **Step 4: Render human and JSON reports**

  Human output must show one row per capability and distinguish supported, unsupported, and unknown. `--json` writes one JSON object to stdout. Probe errors become report entries when possible; they do not panic.

- [ ] **Step 5: Review command behavior manually**

  Run `drunk doctor` and `drunk doctor --json`; confirm the human report is readable, JSON parses, missing probes are `unknown`, and the program makes no system changes.

**Acceptance:** `drunk doctor` gives useful, read-only Linux diagnostics with no claims that the Roblox client or a runtime is already supported.

## Task 4: Publish the first milestone and stop at the gate

**Files:**
- Modify: `docs/runtime-evaluation.md`
- Modify: `docs/adr/0001-android-runtime-backend.md`

- [ ] **Step 1: Capture environment-specific results**

  Record the host used for the manual Android-app protocol and any unverified hardware/platform assumptions. Do not store personal identifiers or credentials.

- [ ] **Step 2: Confirm the gate outcome**

  If the runtime and sample app launch are demonstrated, document the exact backend and prepare a follow-on plan for Roblox package installation, rendering/input/audio integration, and Flatpak. If not, record what failed and stop before implementing the Roblox runtime.

- [ ] **Step 3: Present the milestone for user review**

  Summarize the evidence, limitations, selected backend or no-go, and any design decision that would need the user's input. Continue into the follow-on product plan only after the feasibility result supports option B.

**Acceptance:** The milestone ends with a reviewable technical decision and no unsupported claim of Sober parity or Roblox compatibility.

## Coverage and self-review

- The Sober research boundary and source credibility are handled in Task 1.
- The approved B architecture and explicit runtime feasibility gate are handled in Tasks 1 and 4.
- Rust-only Drunk code and documented external components are handled in Tasks 2–3 and the global constraints.
- Linux x86-64, kernel, CPU, GPU/graphics diagnostics are handled in Task 3.
- Package handling, session safety, and no security bypass are preserved as constraints in Tasks 1, 3, and 4; package installation is deliberately deferred until the runtime is proven.
- Desktop integration, Flatpak packaging, full installer, and game-session lifecycle are not part of this first sub-project because their interfaces depend on the runtime decision. A follow-on plan must cover them before that implementation begins.
