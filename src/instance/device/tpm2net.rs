use crate::models;
use async_trait::async_trait;
use palette::{rgb::Rgb, Pixel};

use super::{common::*, DeviceError};

pub type Tpm2NetDevice = Rewriter<Tpm2NetDeviceImpl>;

pub struct Tpm2NetDeviceImpl {
    max_packet_size: u32,
    byte_count: u32,
    total_packets: u8,
    udp_conn: UdpProvider,
    leds: Vec<models::Color>,
}

#[async_trait]
impl WritingDevice for Tpm2NetDeviceImpl {
    type Config = models::Tpm2Net;

    fn new(config: &Self::Config) -> Result<Self, DeviceError> {
        let tpm2_byte_count = config.hardware_led_count * 3;
        let mut tpm2_total_packets = tpm2_byte_count / config.max_packet;

        if (tpm2_byte_count % config.max_packet) != 0 {
            tpm2_total_packets += 1;
        }

        Ok(Self {
            max_packet_size: config.max_packet,
            byte_count: tpm2_byte_count,
            total_packets: tpm2_total_packets as u8,
            udp_conn: UdpProvider::from_config(config)?,
            leds: vec![Default::default(); config.hardware_led_count as _],
        })
    }

    async fn set_let_data(
        &mut self,
        _config: &Self::Config,
        led_data: &[models::Color],
    ) -> Result<(), DeviceError> {
        self.leds.copy_from_slice(led_data);
        Ok(())
    }

    async fn write(&mut self) -> Result<(), DeviceError> {
        let mut tpm2_buf = vec![0u8; (self.max_packet_size + 7) as usize];
        let mut this_packet_bytes = 0;
        let mut packet_number = 1;

        let raw_data: Vec<u8> = self
            .leds
            .iter()
            .map(|&l| Rgb::into_raw::<[u8; 3]>(l))
            .flatten()
            .collect();

        for (raw_idx, &raw_byte) in raw_data.iter().enumerate() {
            let pkt_offset = raw_idx % self.max_packet_size as usize;

            if pkt_offset == 0 {
                // start of new packet
                this_packet_bytes = {
                    // Is this the last packet ?
                    if (self.byte_count - raw_idx as u32) < self.max_packet_size {
                        self.byte_count % self.max_packet_size // Last packet
                    } else {
                        self.max_packet_size // Earlier packets
                    }
                };

                tpm2_buf[0] = 0x9A;
                tpm2_buf[1] = 0xDA;
                tpm2_buf[2] = ((this_packet_bytes >> 8) & 0xFF) as u8;
                tpm2_buf[3] = (this_packet_bytes & 0xFF) as u8;
                tpm2_buf[4] = (packet_number & 0xFF) as u8;
                tpm2_buf[5] = self.total_packets;

                packet_number += 1;
            }

            tpm2_buf[6 + pkt_offset] = raw_byte;

            // is this the      last byte of last packet || last byte of other packets
            if raw_idx == (self.byte_count - 1) as usize
                || pkt_offset == (self.max_packet_size - 1) as usize
            {
                tpm2_buf[6 + pkt_offset + 1] = 0x36; // Packet end byte
                self.udp_conn
                    .write(&tpm2_buf[..(this_packet_bytes + 7) as usize])
                    .await?;
            }
        }

        Ok(())
    }
}
