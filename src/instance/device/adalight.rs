use async_trait::async_trait;
use palette::{rgb::Rgb, Pixel};
use tokio::io::AsyncWriteExt;
use tokio_serial::{self, SerialPortBuilderExt, SerialStream};

use crate::models;

use super::{common::*, DeviceError};

pub type AdalightDevice = Rewriter<AdalightDeviceImpl>;

const HEADER_SIZE: usize = 6;

pub struct AdalightDeviceImpl {
    /// Handle to UART character device
    dev_handle: SerialStream,
    // Data buffer containing whole UART message
    adalight_data: Vec<u8>,
}

#[async_trait]
impl WritingDevice for AdalightDeviceImpl {
    type Config = models::Adalight;

    fn new(config: &Self::Config) -> Result<Self, DeviceError> {
        let device_name = format!("/dev/{}", config.output);
        let handle = tokio_serial::new(&device_name, config.rate)
            .open_native_async()
            .map_err(|_| DeviceError::FailedOpen(device_name))?;

        // Used for header assembly only
        let total_led_count = config.hardware_led_count - 1;

        let buffer_size: usize = HEADER_SIZE + (config.hardware_led_count as usize * 3);
        let mut buffer = vec![0x00; buffer_size];
        buffer[0] = 'a' as u8;
        buffer[1] = 'd' as u8;
        buffer[2] = 'a' as u8;
        buffer[3] = ((total_led_count & 0xFF00) >> 8) as u8;
        buffer[4] = (total_led_count & 0xFF) as u8;
        // checksum
        buffer[5] = buffer[3] ^ buffer[4] ^ 0x55;

        debug!(
            "Adalight header for {} leds: {}{}{} hi={:#02x} lo={:#02x} chk={:#02x}",
            config.hardware_led_count,
            buffer[0] as char,
            buffer[1] as char,
            buffer[2] as char,
            buffer[3],
            buffer[4],
            buffer[5]
        );

        Ok(Self {
            dev_handle: handle,
            adalight_data: buffer,
        })
    }

    async fn set_let_data(
        &mut self,
        _config: &Self::Config,
        led_data: &[models::Color],
    ) -> Result<(), DeviceError> {
        led_data.into_iter().enumerate().for_each(|(index, &led)| {
            let data: [u8; 3] = Rgb::into_raw(led);
            self.adalight_data[HEADER_SIZE + (index * data.len())
                ..HEADER_SIZE + (index * data.len()) + data.len()]
                .copy_from_slice(&data);
        });

        trace!("Adalight: {} LEDs were set", led_data.len());

        Ok(())
    }

    async fn write(&mut self) -> Result<(), DeviceError> {
        trace!("Adalight: About to write out LED data over serial");
        self.dev_handle
            .write(&self.adalight_data)
            .await
            .map(|_| DeviceError::SerialError("Failed writing LED data".to_string()))?;

        self.dev_handle
            .flush()
            .await
            .map(|_| DeviceError::SerialError("Failed flushing serial".to_string()))?;

        Ok(())
    }
}
