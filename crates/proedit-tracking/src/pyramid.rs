//! Image pyramid utilities for multi-scale tracking.

/// A grayscale image stored as f32 values [0, 1].
#[derive(Debug, Clone)]
pub struct GrayImage {
    pub data: Vec<f32>,
    pub width: u32,
    pub height: u32,
}

impl GrayImage {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            data: vec![0.0; (width * height) as usize],
            width,
            height,
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> f32 {
        let x = x.clamp(0, self.width as i32 - 1) as u32;
        let y = y.clamp(0, self.height as i32 - 1) as u32;
        self.data[(y * self.width + x) as usize]
    }

    #[inline]
    pub fn set(&mut self, x: u32, y: u32, val: f32) {
        if x < self.width && y < self.height {
            self.data[(y * self.width + x) as usize] = val;
        }
    }
}

/// Multi-scale image pyramid.
pub struct ImagePyramid {
    pub levels: Vec<GrayImage>,
}

impl ImagePyramid {
    pub fn build(gray: &GrayImage, num_levels: u32) -> Self {
        let mut levels = vec![gray.clone()];
        for _ in 1..num_levels {
            let prev = levels.last().unwrap();
            let nw = prev.width.div_ceil(2);
            let nh = prev.height.div_ceil(2);
            let mut level = GrayImage::new(nw, nh);
            for y in 0..nh {
                for x in 0..nw {
                    let sx = (x * 2) as i32;
                    let sy = (y * 2) as i32;
                    let avg = (prev.get(sx, sy)
                        + prev.get(sx + 1, sy)
                        + prev.get(sx, sy + 1)
                        + prev.get(sx + 1, sy + 1))
                        * 0.25;
                    level.set(x, y, avg);
                }
            }
            levels.push(level);
        }
        Self { levels }
    }
}

/// Convert RGBA u8 frame data to a grayscale image.
pub fn rgb_to_gray(rgba: &[u8], w: u32, h: u32) -> GrayImage {
    let size = (w * h) as usize;
    let mut gray = GrayImage::new(w, h);
    for i in 0..size {
        let idx = i * 4;
        if idx + 2 < rgba.len() {
            gray.data[i] = (0.299 * rgba[idx] as f32
                + 0.587 * rgba[idx + 1] as f32
                + 0.114 * rgba[idx + 2] as f32)
                / 255.0;
        }
    }
    gray
}

/// Compute spatial gradients (Ix, Iy) using central differences.
pub fn compute_gradients(img: &GrayImage) -> (Vec<f32>, Vec<f32>) {
    let size = (img.width * img.height) as usize;
    let mut ix = vec![0.0f32; size];
    let mut iy = vec![0.0f32; size];
    for y in 1..(img.height as i32 - 1) {
        for x in 1..(img.width as i32 - 1) {
            let idx = (y as u32 * img.width + x as u32) as usize;
            ix[idx] = (img.get(x + 1, y) - img.get(x - 1, y)) * 0.5;
            iy[idx] = (img.get(x, y + 1) - img.get(x, y - 1)) * 0.5;
        }
    }
    (ix, iy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gray_image() {
        let mut img = GrayImage::new(4, 4);
        img.set(2, 3, 0.75);
        assert!((img.get(2, 3) - 0.75).abs() < 0.001);
        let _ = img.get(-1, -1);
        let _ = img.get(100, 100);
    }

    #[test]
    fn test_rgb_to_gray() {
        let rgba = [255, 255, 255, 255];
        let gray = rgb_to_gray(&rgba, 1, 1);
        assert!((gray.data[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_pyramid_build() {
        let img = GrayImage::new(64, 64);
        let pyr = ImagePyramid::build(&img, 3);
        assert_eq!(pyr.levels.len(), 3);
        assert_eq!(pyr.levels[1].width, 32);
        assert_eq!(pyr.levels[2].width, 16);
    }

    #[test]
    fn test_gradients() {
        let mut img = GrayImage::new(8, 8);
        for y in 0..8u32 {
            for x in 0..8u32 {
                img.set(x, y, x as f32 / 7.0);
            }
        }
        let (ix, _iy) = compute_gradients(&img);
        assert!(ix[(4 * 8 + 4) as usize] > 0.0);
    }
}
