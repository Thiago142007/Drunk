use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Capability {
    Supported { detail: String },
    Unsupported { detail: String },
    Unknown { detail: String },
}

impl Capability {
    pub fn status_and_detail(&self) -> (&'static str, &str) {
        match self {
            Self::Supported { detail } => ("supported", detail),
            Self::Unsupported { detail } => ("unsupported", detail),
            Self::Unknown { detail } => ("unknown", detail),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub os: Capability,
    pub architecture: Capability,
    pub kernel: Capability,
    pub cpu_features: Capability,
    pub graphics_devices: Capability,
    pub graphics_loader: Capability,
}

impl DoctorReport {
    pub fn rows(&self) -> [(&'static str, &Capability); 6] {
        [
            ("os", &self.os),
            ("architecture", &self.architecture),
            ("kernel", &self.kernel),
            ("cpu_features", &self.cpu_features),
            ("graphics_devices", &self.graphics_devices),
            ("graphics_loader", &self.graphics_loader),
        ]
    }
}
