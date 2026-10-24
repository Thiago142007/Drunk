use std::{fs, path::Path};

use super::{Capability, DoctorReport};

fn unknown(detail: &str) -> Capability {
    Capability::Unknown {
        detail: detail.into(),
    }
}

pub fn collect() -> DoctorReport {
    let linux = std::env::consts::OS == "linux";
    DoctorReport {
        os: if linux {
            Capability::Supported {
                detail: "Linux matches the Sober comparison baseline; no Drunk runtime guarantee."
                    .into(),
            }
        } else {
            Capability::Unsupported {
                detail: format!(
                    "{} does not match the Linux comparison baseline.",
                    std::env::consts::OS
                ),
            }
        },
        architecture: if std::env::consts::ARCH == "x86_64" {
            Capability::Supported {
                detail: "x86_64 matches the Sober comparison baseline; no Drunk runtime guarantee."
                    .into(),
            }
        } else {
            Capability::Unsupported {
                detail: format!(
                    "{} does not match the x86_64 comparison baseline.",
                    std::env::consts::ARCH
                ),
            }
        },
        kernel: if linux {
            kernel()
        } else {
            unknown("Linux kernel probe is unavailable on this OS.")
        },
        cpu_features: if linux {
            cpu_features()
        } else {
            unknown("Linux CPU feature probe is unavailable on this OS.")
        },
        graphics_devices: if linux {
            graphics_devices()
        } else {
            unknown("Linux graphics device probe is unavailable on this OS.")
        },
        graphics_loader: if linux {
            graphics_loader()
        } else {
            unknown("Linux graphics loader probe is unavailable on this OS.")
        },
    }
}

fn kernel() -> Capability {
    let Ok(release) = fs::read_to_string("/proc/sys/kernel/osrelease") else {
        return unknown("Kernel release could not be read.");
    };
    // Only report the numeric version, avoiding arbitrary content from procfs.
    let mut parts = release.trim().split('.');
    let major = parts.next().and_then(|part| part.parse::<u32>().ok());
    let minor = parts.next().and_then(|part| part.parse::<u32>().ok());
    let (Some(major), Some(minor)) = (major, minor) else {
        return unknown("Kernel release could not be parsed.");
    };
    let detail = format!(
        "Kernel {major}.{minor}; Sober comparison baseline is >= 5.11, not a Drunk runtime guarantee."
    );
    if (major, minor) >= (5, 11) {
        Capability::Supported { detail }
    } else {
        Capability::Unsupported { detail }
    }
}

fn cpu_features() -> Capability {
    let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") else {
        return unknown("CPU feature information could not be read.");
    };
    let flags: Vec<_> = cpuinfo
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim() == "flags").then_some(value)
        })
        .collect();
    if flags.is_empty() || flags.iter().any(|entry| entry.trim().is_empty()) {
        return unknown("CPU flags were unavailable; SSE4.1 and SSE4.2 could not be determined.");
    }
    let has = |feature| {
        flags
            .iter()
            .all(|entry| entry.split_whitespace().any(|flag| flag == feature))
    };
    let sse41 = has("sse4_1");
    let sse42 = has("sse4_2");
    let detail = format!(
        "SSE4.1 minimum: {sse41}; SSE4.2 recommended: {sse42} across reported CPU flags; comparison only, and neither result establishes Drunk runtime compatibility."
    );
    if sse41 {
        Capability::Supported { detail }
    } else {
        Capability::Unsupported { detail }
    }
}

fn graphics_devices() -> Capability {
    let Ok(entries) = fs::read_dir("/dev/dri") else {
        return unknown(
            "DRI device directory is unavailable or unreadable; graphics support is undetermined.",
        );
    };
    let mut found = 0;
    for entry in entries {
        let Ok(entry) = entry else {
            return unknown("DRI device entries could not all be inspected.");
        };
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let suffix = name
            .strip_prefix("renderD")
            .or_else(|| name.strip_prefix("card"));
        if suffix.is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
        }) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::FileTypeExt;
                let Ok(kind) = entry.file_type() else {
                    return unknown("DRI device metadata could not be inspected.");
                };
                if kind.is_char_device() {
                    found += 1;
                }
            }
        }
    }
    if found == 0 {
        unknown("No DRI graphics device nodes observed; graphics support is undetermined.")
    } else {
        Capability::Supported {
            detail: format!(
                "{found} DRI graphics device node(s) observed; permissions, driver functionality, and runtime support are unverified."
            ),
        }
    }
}

fn graphics_loader() -> Capability {
    // Inspect standard loader locations only. Never execute a loader or open a device.
    let directories = [
        "/usr/lib",
        "/usr/lib64",
        "/lib",
        "/lib64",
        "/usr/lib/x86_64-linux-gnu",
        "/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
        "/lib/aarch64-linux-gnu",
    ];
    let libraries = ["libvulkan.so.1", "libEGL.so.1", "libGL.so.1"];
    let found: Vec<_> = libraries
        .into_iter()
        .filter(|library| {
            directories.iter().any(|directory| {
                fs::metadata(Path::new(directory).join(library))
                    .is_ok_and(|metadata| metadata.is_file())
            })
        })
        .collect();
    if found.is_empty() {
        unknown(
            "No graphics loader observed in standard library locations; other locations or unreadable files may exist.",
        )
    } else {
        Capability::Supported {
            detail: format!(
                "Observed {}; presence only, API versions and driver/runtime functionality are unverified. Drunk runtime feasibility remains provisional no-go.",
                found.join(", ")
            ),
        }
    }
}
