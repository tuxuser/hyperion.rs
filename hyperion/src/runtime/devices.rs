//! Definition of the Devices type

use std::time::Instant;

use std::convert::TryFrom;

use futures::{Async, Future, Poll};

use num_traits::Float;
use std::ops::AddAssign;

use crate::color;
use crate::config::*;
use crate::image::*;
use crate::methods;
use crate::runtime::HostHandle;

use super::DeviceInstance;

/// A set of runtime devices
pub struct Devices {
    /// List of device instances
    devices: Vec<DeviceInstance>,
    /// Components handle
    host: HostHandle,
}

impl Devices {
    /// Create a new runtime device host
    ///
    /// # Parameters
    ///
    /// * `config`: configuration handle for devices
    pub fn new(config: ConfigHandle) -> Result<Self, methods::MethodError> {
        let devices = config
            .read()
            .unwrap()
            .devices
            .iter()
            .enumerate()
            .map(|(i, _device)| {
                DeviceInstance::try_from(DeviceConfigHandle::new(config.clone(), i))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            devices,
            host: HostHandle::new(),
        })
    }

    /// Get a reference to the host handle
    pub fn get_host_mut(&mut self) -> &mut HostHandle {
        &mut self.host
    }

    /// Set all LEDs of all devices to a new color immediately
    ///
    /// # Parameters
    ///
    /// * `time`: time of the color update
    /// * `color`: new color to apply immediately to all the LEDs of all devices
    /// * `immediate`: apply change immediately (skipping filtering)
    pub fn set_all_leds(&mut self, time: Instant, color: color::ColorPoint, immediate: bool) {
        for device in self.devices.iter_mut() {
            device.set_all_leds(time, color, immediate);
        }
    }

    /// Update the devices using the given image processor and input image
    ///
    /// # Parameters
    ///
    /// * `time`: time of the color update
    /// * `image_processor`: image processor instance
    /// * `raw_image`: raw RGB image
    /// * `immediate`: apply change immediately (skipping filtering)
    pub fn set_from_image<T: Float + AddAssign + Default + std::fmt::Display>(
        &mut self,
        time: Instant,
        image_processor: &mut Processor<T>,
        raw_image: RawImage,
        immediate: bool,
    ) {
        // Update stored image
        image_processor
            .with_devices(self.host.get_config().read().unwrap().devices.iter())
            .process_image(raw_image);

        // Mutable reference to devices to prevent the closure exclusive access
        let devices = &mut self.devices;
        // Get reference to color config data
        let config = self.host.get_config();
        let correction = &config.read().unwrap().color;

        // Update LEDs with computed colors
        image_processor.update_leds(|(device_idx, led_idx), color| {
            // Should never fail, we only consider valid LEDs
            devices[device_idx]
                .set_led(time, led_idx, correction.process(color), immediate)
                .unwrap();
        });
    }

    /// Reload a given device
    ///
    /// # Parameters
    ///
    /// * `device_index`: index of the device to reload
    /// * `reload_hints`: parts of the device to reload
    pub fn reload_device(
        &mut self,
        device_index: usize,
        reload_hints: ReloadHints,
    ) -> Result<(), crate::methods::MethodError> {
        self.devices[device_index].reload(reload_hints)
    }

    /// Set all LEDs of all devices to a new color immediately
    ///
    /// # Parameters
    ///
    /// * `time`: time of the color update
    /// * `leds`: color data for every device LED
    /// * `immediate`: apply change immediately (skipping filtering)
    pub fn set_leds(&mut self, time: Instant, leds: Vec<color::ColorPoint>, immediate: bool) {
        let mut current_idx = 0;

        for device in self.devices.iter_mut() {
            if current_idx >= leds.len() {
                warn!(
                    "not enough led data (only got {}, check led count)",
                    leds.len()
                );
                break;
            }

            for idx in 0..device.get_data().read().unwrap().leds().len() {
                if current_idx >= leds.len() {
                    break;
                }

                device
                    .set_led(time, idx, leds[current_idx], immediate)
                    .unwrap();

                current_idx += 1;
            }
        }
    }

    /// Get the total LED count for all devices
    pub fn get_led_count(&self) -> usize {
        self.devices.iter().fold(0usize, |s, device| {
            s + device.get_data().read().unwrap().leds().len()
        })
    }
}

impl Future for Devices {
    type Item = ();
    type Error = tokio::timer::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        // Check intervals for devices to write to
        for device in self.devices.iter_mut() {
            while let Async::Ready(()) = device.poll()? {}
        }

        Ok(Async::NotReady)
    }
}
