use ambassador::{delegatable_trait, Delegate};
use derive_more::From;
use serde_derive::{Deserialize, Serialize};
use strum_macros::IntoStaticStr;
use validator::Validate;

use super::{default_false, ColorOrder};

#[delegatable_trait]
pub trait DeviceConfig: Sync + Send {
    fn hardware_led_count(&self) -> usize;

    fn rewrite_time(&self) -> Option<std::time::Duration> {
        None
    }

    fn latch_time(&self) -> std::time::Duration {
        Default::default()
    }
}

macro_rules! impl_device_config {
    ($t:ty) => {
        impl DeviceConfig for $t {
            fn hardware_led_count(&self) -> usize {
                self.hardware_led_count as _
            }

            fn rewrite_time(&self) -> Option<std::time::Duration> {
                if self.rewrite_time == 0 {
                    None
                } else {
                    Some(std::time::Duration::from_millis(self.rewrite_time as _))
                }
            }

            fn latch_time(&self) -> std::time::Duration {
                std::time::Duration::from_millis(self.latch_time as _)
            }
        }
    };
}

#[delegatable_trait]
pub trait Rs232DeviceConfig: Sync + Send {
    fn port_name(&self) -> String;
    fn baudrate(&self) -> u32;
}

macro_rules! impl_rs232_device_config {
    ($t:ty) => {
        impl Rs232DeviceConfig for $t {
            fn port_name(&self) -> String {
                self.output.clone()
            }

            fn baudrate(&self) -> u32 {
                self.rate
            }
        }

        impl DeviceConfig for $t {
            fn hardware_led_count(&self) -> usize {
                self.hardware_led_count as _
            }
        }
    };
}

pub enum NetworkProtocol {
    Tcp,
    Udp,
}

#[delegatable_trait]
pub trait NetworkDeviceConfig: Sync + Send {
    fn address(&self) -> String;
    fn port(&self) -> u16;
}

macro_rules! impl_network_device_config {
    ($t:ty) => {
        impl NetworkDeviceConfig for $t {
            fn address(&self) -> String {
                self.host.clone()
            }

            fn port(&self) -> u16 {
                self.port
            }
        }

        impl DeviceConfig for $t {
            fn hardware_led_count(&self) -> usize {
                self.hardware_led_count as _
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DummyDeviceMode {
    Text,
    Ansi,
}

impl Default for DummyDeviceMode {
    fn default() -> Self {
        Self::Text
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Dummy {
    #[validate(range(min = 1))]
    pub hardware_led_count: u32,
    pub rewrite_time: u32,
    pub latch_time: u32,
    pub mode: DummyDeviceMode,
}

impl_device_config!(Dummy);

impl Default for Dummy {
    fn default() -> Self {
        Self {
            hardware_led_count: 1,
            rewrite_time: 0,
            latch_time: 0,
            mode: Default::default(),
        }
    }
}

fn default_ws_spi_rate() -> i32 {
    3000000
}

fn default_ws_spi_rewrite_time() -> u32 {
    1000
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ws2812Spi {
    #[serde(default = "Default::default")]
    pub color_order: ColorOrder,
    #[validate(range(min = 1))]
    pub hardware_led_count: u32,
    #[serde(default = "default_false")]
    pub invert: bool,
    #[serde(default = "Default::default")]
    pub latch_time: u32,
    pub output: String,
    #[serde(default = "default_ws_spi_rate")]
    pub rate: i32,
    #[serde(default = "default_ws_spi_rewrite_time")]
    pub rewrite_time: u32,
}

impl_device_config!(Ws2812Spi);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhilipsHue {
    pub black_lights_timeout: i32,
    pub brightness_factor: f32,
    pub brightness_max: f32,
    pub brightness_min: f32,
    pub brightness_threshold: f32,
    #[serde(rename = "clientkey")]
    pub client_key: String,
    pub color_order: ColorOrder,
    pub debug_level: String,
    pub debug_streamer: bool,
    pub group_id: i32,
    #[validate(range(min = 1))]
    pub hardware_led_count: u32,
    pub light_ids: Vec<String>,
    pub output: String,
    pub restore_original_state: bool,
    #[serde(rename = "sslHSTimeoutMax")]
    pub ssl_hs_timeout_max: i32,
    #[serde(rename = "sslHSTimeoutMin")]
    pub ssl_hs_timeout_min: i32,
    pub ssl_read_timeout: i32,
    pub switch_off_on_black: bool,
    #[serde(rename = "transitiontime")]
    pub transition_time: f32,
    #[serde(rename = "useEntertainmentAPI")]
    pub use_entertainment_api: bool,
    pub username: String,
    pub verbose: bool,
}

impl DeviceConfig for PhilipsHue {
    fn hardware_led_count(&self) -> usize {
        self.hardware_led_count as _
    }
}

fn default_file_rewrite_time() -> u32 {
    1000
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct File {
    #[serde(default = "Default::default")]
    pub color_order: ColorOrder,
    #[validate(range(min = 1))]
    pub hardware_led_count: u32,
    #[serde(default = "Default::default")]
    pub delay_after_connect: u32,
    #[serde(default = "Default::default")]
    pub latch_time: u32,
    pub output: String,
    #[serde(default = "default_file_rewrite_time")]
    pub rewrite_time: u32,
    #[serde(default = "Default::default")]
    pub print_time_stamp: bool,
}

impl DeviceConfig for File {
    fn hardware_led_count(&self) -> usize {
        self.hardware_led_count as _
    }
}

fn default_adalight_delay_after_connect() -> u32 {
    0
}

fn default_adalight_rewrite_time() -> u32 {
    1000
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Adalight {
    #[serde(default = "Default::default")]
    pub color_order: ColorOrder,
    #[validate(range(min = 1))]
    pub hardware_led_count: u32,
    #[serde(default = "default_adalight_delay_after_connect")]
    pub delay_after_connect: u32,
    #[serde(rename = "lightberry_apa102_mode")]
    pub lightberry_apa102_mode: bool,
    pub output: String,
    pub rate: u32,
    #[serde(default = "default_adalight_rewrite_time")]
    pub rewrite_time: u32,
}

impl_rs232_device_config!(Adalight);

fn default_tpm2_rate() -> u32 {
    115200
}

fn default_tpm2_rewrite_time() -> u32 {
    1000
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tpm2 {
    #[serde(default = "Default::default")]
    pub color_order: ColorOrder,
    #[validate(range(min = 1))]
    pub hardware_led_count: u32,
    #[serde(default = "Default::default")]
    pub delay_after_connect: u32,
    #[serde(default = "Default::default")]
    pub latch_time: u32,
    pub output: String,
    #[serde(default = "default_tpm2_rate")]
    pub rate: u32,
    #[serde(default = "default_tpm2_rewrite_time")]
    pub rewrite_time: u32,
}

impl_rs232_device_config!(Tpm2);

fn default_tpm2net_max_packet_count() -> u32 {
    170
}

fn default_tpm2net_port() -> u16 {
    65506
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tpm2Net {
    #[serde(default = "Default::default")]
    pub color_order: ColorOrder,
    #[validate(range(min = 1))]
    pub hardware_led_count: u32,
    #[serde(default = "Default::default")]
    pub latch_time: u32,
    #[serde(rename = "max-packet", default = "default_tpm2net_max_packet_count")]
    pub max_packet: u32,
    pub host: String,
    #[serde(default = "default_tpm2net_port")]
    pub port: u16,
}

impl_network_device_config!(Tpm2Net);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, IntoStaticStr, Delegate, From)]
#[serde(rename_all = "lowercase", tag = "type", deny_unknown_fields)]
#[delegate(DeviceConfig)]
pub enum Device {
    Dummy(Dummy),
    Ws2812Spi(Ws2812Spi),
    PhilipsHue(PhilipsHue),
    File(File),
    Adalight(Adalight),
    Tpm2(Tpm2),
    Tpm2Net(Tpm2Net),
}

impl Default for Device {
    fn default() -> Self {
        Self::Dummy(Dummy::default())
    }
}

impl Validate for Device {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            Device::Dummy(device) => device.validate(),
            Device::Ws2812Spi(device) => device.validate(),
            Device::PhilipsHue(device) => device.validate(),
            Device::File(device) => device.validate(),
            Device::Adalight(device) => device.validate(),
            Device::Tpm2(device) => device.validate(),
            Device::Tpm2Net(device) => device.validate(),
        }
    }
}
