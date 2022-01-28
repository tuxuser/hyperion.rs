use async_trait::async_trait;
use palette::{rgb::Rgb, Pixel};

use crate::models;

use super::{common::*, DeviceError};

pub type Tpm2Device = Rewriter<Tpm2DeviceImpl>;

const HEADER_SIZE: usize = 4;

pub struct Tpm2DeviceImpl {
    /// Handle to UART character device
    dev_handle: Rs232Provider,
    tpm2_data: Vec<u8>,
}

#[async_trait]
impl WritingDevice for Tpm2DeviceImpl {
    type Config = models::Tpm2;

    fn new(config: &Self::Config) -> Result<Self, DeviceError> {
        let led_frame_length = (config.hardware_led_count * 3) as usize;

        let mut buffer = vec![0u8; HEADER_SIZE + led_frame_length + 1];
        buffer[0] = 0xC9; // block-start byte
        buffer[1] = 0xDA; // DATA frame
        buffer[2] = ((led_frame_length >> 8) & 0xFF) as u8; // frame size high byte
        buffer[3] = (led_frame_length & 0xFF) as u8; // frame size low byte
        buffer[HEADER_SIZE + led_frame_length] = 0x36; // block-end byte

        Ok(Self {
            dev_handle: Rs232Provider::from_config(config)?,
            tpm2_data: buffer,
        })
    }

    async fn set_let_data(
        &mut self,
        _config: &Self::Config,
        led_data: &[models::Color],
    ) -> Result<(), DeviceError> {
        led_data.into_iter().enumerate().for_each(|(index, &led)| {
            let data: [u8; 3] = Rgb::into_raw(led);

            let start_pos = HEADER_SIZE + (index * data.len());

            self.tpm2_data[start_pos..start_pos + data.len()].copy_from_slice(&data);
        });

        Ok(())
    }

    async fn write(&mut self) -> Result<(), DeviceError> {
        self.dev_handle.write(&self.tpm2_data).await?;
        Ok(())
    }
}
