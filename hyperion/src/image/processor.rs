//! Definition of the Processor type

use crate::color;
use crate::runtime::LedInstance;
use std::cmp::min;

use super::RawImage;

/// Image pixel accumulator
#[derive(Default, Clone)]
struct Pixel {
    /// Accumulated color
    color: [f32; 3],
    /// Number of samples
    count: usize,
}

impl Pixel {
    /// Reset this pixels' value and sample count
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Add a new sample to this pixel
    ///
    /// # Parameters
    ///
    /// * `(r, g, b)`: sampled RGB values
    pub fn sample(&mut self, (r, g, b): (u8, u8, u8)) {
        self.color[0] += f32::from(r) / 255.0;
        self.color[1] += f32::from(g) / 255.0;
        self.color[2] += f32::from(b) / 255.0;
        self.count += 1;
    }

    /// Compute the mean of this pixel
    pub fn mean(&self) -> color::ColorPoint {
        color::ColorPoint::from((
            self.color[0] / self.count as f32,
            self.color[1] / self.count as f32,
            self.color[2] / self.count as f32,
        ))
    }
}

/// Raw image data processor
#[derive(Default)]
pub struct Processor {
    /// Width of the LED map
    width: usize,
    /// Height of the LED map
    height: usize,
    /// 2D row-major list of (color_idx, device_idx, led_idx)
    led_map: Vec<Vec<(usize, usize, usize)>>,
    /// Color storage for every known LED
    color_map: Vec<Pixel>,
}

/// Image processor reference with LED details
pub struct ProcessorWithDevices<'p, 'a, I: Iterator<Item = (usize, &'a LedInstance, usize)>> {
    /// Image processor
    processor: &'p mut Processor,
    /// Device LEDs iterator
    leds: I,
}

impl Processor {
    /// Allocates the image processor working memory
    ///
    /// # Parameters
    ///
    /// * `width`: width of the incoming images in pixels
    /// * `height`: height of the incoming images in pixels
    /// * `leds`: LED specification for target devices
    fn alloc<'a>(
        &mut self,
        width: u32,
        height: u32,
        leds: impl Iterator<Item = (usize, &'a LedInstance, usize)>,
    ) {
        let width = width as usize;
        let height = height as usize;

        // Initialize led map data structure
        let mut led_map = Vec::with_capacity(width * height);
        for _ in 0..(width * height) {
            led_map.push(Vec::new());
        }

        // Add leds whose area overlap the current pixel
        let mut count = 0;
        for (index, led) in leds.enumerate() {
            for j in min(
                height - 1,
                (led.1.spec.vscan.min * height as f32).floor() as usize,
            )
                ..min(
                    height,
                    (led.1.spec.vscan.max * height as f32).ceil() as usize,
                )
            {
                // Vertical scan range
                let y_min = j as f32 / height as f32;
                let y_max = (j + 1) as f32 / height as f32;

                for i in min(
                    width - 1,
                    (led.1.spec.hscan.min * width as f32).floor() as usize,
                )
                    ..min(width, (led.1.spec.hscan.max * width as f32).ceil() as usize)
                {
                    // Horizontal scan range
                    let x_min = i as f32 / width as f32;
                    let x_max = (i + 1) as f32 / width as f32;

                    let map_index = j * width + i;

                    if led.1.spec.hscan.min < x_max
                        && led.1.spec.hscan.max >= x_min
                        && led.1.spec.vscan.min < y_max
                        && led.1.spec.vscan.max >= y_min
                    {
                        led_map[map_index].push((index, led.0, led.2));
                    }
                }
            }

            count += 1;
        }

        // Color map for computation
        let color_map = vec![Pixel::default(); count];

        *self = Self {
            width,
            height,
            led_map,
            color_map,
        };
    }

    /// Checks if this image processor is set up to process images of the given size
    ///
    /// # Parameters
    ///
    /// * `width`: width of the image data
    /// * `height`: height of the image data
    ///
    /// # Return value
    ///
    /// `true` if this image process supports this size, `false` otherwise
    fn matches(&self, width: u32, height: u32) -> bool {
        let width = width as usize;
        let height = height as usize;

        self.width == width && self.height == height
    }

    /// Prepares processing of image data using the given LED scan ranges
    ///
    /// Note that if changes occurred in the LED details, the image processor
    /// should be reset first so this method can reallocate internal structures.
    ///
    /// # Parameters
    ///
    /// * `leds`: iterator returning `(device_idx, led_instance, led_idx)` tuples
    pub fn with_devices<'p, 'a, I: Iterator<Item = (usize, &'a LedInstance, usize)>>(
        &'p mut self,
        leds: I,
    ) -> ProcessorWithDevices<'p, 'a, I> {
        ProcessorWithDevices {
            processor: self,
            leds,
        }
    }

    /// Process incoming image data into led colors
    ///
    /// The image processor should already allocated at the right size
    /// using the alloc method.
    ///
    /// # Parameters
    ///
    /// * `raw_image`: raw RGB image
    fn process_image(&mut self, raw_image: RawImage) {
        let (width, height) = raw_image.get_dimensions();

        // Reset all colors
        for color in &mut self.color_map {
            color.reset();
        }

        for j in 0..height {
            for i in 0..width {
                let map_idx = (j * width + i) as usize;
                let rgb = raw_image.get_pixel(i, j);

                for (pixel_idx, _device_idx, _led_idx) in &self.led_map[map_idx] {
                    self.color_map[*pixel_idx].sample(rgb);
                }
            }
        }
    }

    /// Update LEDs with computed colors
    ///
    /// # Parameters
    ///
    /// * `led_setter`: callback to update LED colors
    pub fn update_leds(&self, mut led_setter: impl FnMut((usize, usize), color::ColorPoint)) {
        // Compute mean and assign to led instances
        for pixel in self.led_map.iter() {
            for (pixel_idx, device_idx, led_idx) in pixel.iter() {
                led_setter((*device_idx, *led_idx), self.color_map[*pixel_idx].mean());
            }
        }
    }
}

impl<'p, 'a, I: Iterator<Item = (usize, &'a LedInstance, usize)>> ProcessorWithDevices<'p, 'a, I> {
    /// Process incoming image data into led colors
    ///
    /// # Parameters
    ///
    /// * `raw_image`: raw RGB image
    pub fn process_image(self, raw_image: RawImage) -> &'p mut Processor {
        let (width, height) = raw_image.get_dimensions();

        // Check that this processor has the right size
        if !self.processor.matches(width, height) {
            self.processor.alloc(width, height, self.leds);
        }

        self.processor.process_image(raw_image);
        self.processor
    }
}
