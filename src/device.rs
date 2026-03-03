use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceInfo {
    pub machine_id: String,
    pub device_name: String,
    pub os: String,
    pub os_version: String,
}

pub fn get_current_device_info() -> DeviceInfo {
    let machine_id = machine_uid::get().unwrap_or_else(|_| "unknown-machine-id".to_string());

    let device_name = hostname::get()
        .map(|s: std::ffi::OsString| s.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown-host".to_string());

    let info = os_info::get();

    let os = match info.os_type() {
        os_info::Type::Windows => "windows".to_string(),
        os_info::Type::Macos => "macos".to_string(),
        os_info::Type::Linux => "linux".to_string(),
        _ => info.os_type().to_string(),
    };

    let os_version = info.version().to_string();

    DeviceInfo {
        machine_id,
        device_name,
        os,
        os_version,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_collection() {
        let info = get_current_device_info();

        assert!(!info.machine_id.is_empty());
        assert!(!info.os.is_empty());
    }
}
