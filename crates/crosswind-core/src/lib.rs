pub mod link;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Windows,
    Android,
    MacOs,
    Other,
}

impl Platform {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "linux" => Self::Linux,
            "windows" => Self::Windows,
            "android" => Self::Android,
            "macos" => Self::MacOs,
            _ => Self::Other,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Linux => "Linux",
            Self::Windows => "Windows",
            Self::Android => "Android",
            Self::MacOs => "macOS",
            Self::Other => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwdlRoute {
    NativeApple,
    LinuxMonitorInjection,
    WslUsbPassthrough,
    Esp32Dongle,
    LinuxBridgeDevice,
    RootedMonitorMode,
}

impl AwdlRoute {
    pub fn candidates_for(platform: Platform) -> &'static [AwdlRoute] {
        match platform {
            Platform::MacOs => &[Self::NativeApple],
            Platform::Linux => &[Self::LinuxMonitorInjection, Self::Esp32Dongle],
            Platform::Windows => &[
                Self::WslUsbPassthrough,
                Self::Esp32Dongle,
                Self::LinuxBridgeDevice,
            ],
            Platform::Android => &[Self::RootedMonitorMode, Self::Esp32Dongle],
            Platform::Other => &[Self::Esp32Dongle, Self::LinuxBridgeDevice],
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::NativeApple => "AWDL is built into the OS; speak AirDrop directly",
            Self::LinuxMonitorInjection => {
                "monitor mode and frame injection through nl80211"
            }
            Self::WslUsbPassthrough => {
                "USB Wi-Fi adapter passed into WSL2 with usbipd, running the Linux route"
            }
            Self::Esp32Dongle => "ESP32 over USB runs AWDL on its own radio",
            Self::LinuxBridgeDevice => {
                "nearby Linux box receives AirDrop and forwards files over the network"
            }
            Self::RootedMonitorMode => {
                "rooted device with a Wi-Fi chip that allows monitor mode and injection"
            }
        }
    }

    pub fn status(self) -> ResearchStatus {
        match self {
            Self::NativeApple | Self::LinuxMonitorInjection => ResearchStatus::ProvenElsewhere,
            Self::LinuxBridgeDevice => ResearchStatus::Feasible,
            Self::WslUsbPassthrough | Self::RootedMonitorMode | Self::Esp32Dongle => {
                ResearchStatus::Unexplored
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchStatus {
    ProvenElsewhere,
    Feasible,
    Unexplored,
}

impl ResearchStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::ProvenElsewhere => "proven by prior work",
            Self::Feasible => "feasible with existing tools",
            Self::Unexplored => "unexplored",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_platform_has_at_least_one_route() {
        for platform in [
            Platform::Linux,
            Platform::Windows,
            Platform::Android,
            Platform::MacOs,
            Platform::Other,
        ] {
            assert!(!AwdlRoute::candidates_for(platform).is_empty());
        }
    }

    #[test]
    fn windows_never_relies_on_native_wifi_driver() {
        let windows_routes = AwdlRoute::candidates_for(Platform::Windows);
        assert!(!windows_routes.contains(&AwdlRoute::NativeApple));
        assert!(!windows_routes.contains(&AwdlRoute::LinuxMonitorInjection));
    }
}
