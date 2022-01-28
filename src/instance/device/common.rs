use std::net::SocketAddr;
use std::time::Instant;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::net::UdpSocket;
use tokio_serial::{self, SerialPortBuilderExt, SerialStream};

use super::{DeviceError, DeviceImpl};
use crate::models::{self, DeviceConfig, NetworkDeviceConfig, Rs232DeviceConfig};

#[async_trait]
pub trait WritingDevice: Send + Sized {
    type Config: DeviceConfig;

    fn new(config: &Self::Config) -> Result<Self, DeviceError>;

    async fn set_let_data(
        &mut self,
        config: &Self::Config,
        led_data: &[models::Color],
    ) -> Result<(), DeviceError>;

    async fn write(&mut self) -> Result<(), DeviceError>;
}

pub struct Rewriter<D: WritingDevice> {
    inner: D,
    config: D::Config,
    last_write_time: Option<Instant>,
    next_write_time: Option<Instant>,
}

impl<D: WritingDevice> Rewriter<D> {
    pub fn new(config: D::Config) -> Result<Self, DeviceError> {
        let inner = D::new(&config)?;

        Ok(Self {
            inner,
            config,
            last_write_time: None,
            next_write_time: None,
        })
    }

    async fn write(&mut self) -> Result<(), DeviceError> {
        self.inner.write().await?;
        self.last_write_time = Some(Instant::now());
        self.next_write_time = None;
        Ok(())
    }

    async fn latching_write(&mut self) -> Result<(), DeviceError> {
        let latch_time = self.config.latch_time();
        if latch_time.is_zero() {
            // No latch time, write immediately
            self.write().await?;
        } else {
            if let Some(lwt) = self.last_write_time {
                // We wrote something already, so schedule a write after the next latch period
                let now = Instant::now();
                let next_write_time = lwt + latch_time;

                if next_write_time < now {
                    // Latch time elapsed already
                    self.write().await?;
                } else {
                    // Not elapsed yet, so schedule it
                    self.next_write_time = Some(next_write_time);
                }
            } else {
                // Never wrote anything, so immediately write
                self.write().await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<D: WritingDevice> DeviceImpl for Rewriter<D> {
    async fn set_led_data(&mut self, led_data: &[models::Color]) -> Result<(), DeviceError> {
        self.inner.set_let_data(&self.config, led_data).await?;
        self.latching_write().await?;
        Ok(())
    }

    async fn update(&mut self) -> Result<(), DeviceError> {
        // Handle latching
        if let Some(next_write_time) = self.next_write_time {
            // A write was pending because of latching
            let now = Instant::now();

            if next_write_time > now {
                // We still have to wait
                tokio::time::sleep_until(next_write_time.into()).await;
            }

            // Elapsed, write immediately
            self.write().await?;
        }

        if let Some(rewrite_time) = self.config.rewrite_time() {
            let now = Instant::now();
            let next_rewrite_time = self
                .last_write_time
                .map(|lwt| lwt + rewrite_time)
                .unwrap_or(now);

            // Wait until the next rewrite cycle if necessary
            if next_rewrite_time > now {
                tokio::time::sleep_until(next_rewrite_time.into()).await;
            }

            // Write to device
            self.latching_write().await?;

            Ok(())
        } else {
            futures::future::pending().await
        }
    }
}

pub struct Rs232Provider {
    serial_port: SerialStream,
}

impl Rs232Provider {
    fn new(port: &str, baudrate: u32) -> Result<Self, DeviceError> {
        let device_name = format!("/dev/{}", port);
        let handle = tokio_serial::new(&device_name, baudrate)
            .open_native_async()
            .map_err(|_| DeviceError::NotSupported("Failed opening serial port"))?;

        Ok(Self {
            serial_port: handle,
        })
    }

    pub fn from_config(config: &dyn Rs232DeviceConfig) -> Result<Self, DeviceError> {
        Self::new(&config.port_name(), config.baudrate())
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), DeviceError> {
        self.serial_port.write(data).await?;
        self.serial_port.flush().await?;

        Ok(())
    }
}

pub struct UdpProvider {
    target_addr: SocketAddr,
    socket: Option<UdpSocket>,
}

impl UdpProvider {
    fn new(address: &str, port: u16) -> Result<Self, DeviceError> {
        let remote_address: SocketAddr = format!("{}:{}", address, port)
            .parse()
            .map_err(|_| DeviceError::NotSupported("Failed parsing address / port"))?;

        Ok(Self {
            target_addr: remote_address,
            socket: None,
        })
    }

    pub fn from_config(config: &dyn NetworkDeviceConfig) -> Result<Self, DeviceError> {
        Self::new(&config.address(), config.port())
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), DeviceError> {
        if self.socket.is_none() {
            self.socket = Some(UdpSocket::bind(self.target_addr).await?);
        }

        self.socket.as_ref().unwrap().send(data).await?;

        Ok(())
    }
}
