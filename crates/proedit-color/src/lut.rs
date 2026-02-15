//! .cube LUT file parsing and application.

use crate::error::ColorError;

/// 1D Look-Up Table.
#[derive(Debug, Clone)]
pub struct Lut1D {
    pub size: usize,
    pub data: Vec<[f32; 3]>,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
}

impl Lut1D {
    /// Parse a .cube file containing a 1D LUT.
    pub fn from_cube(content: &str) -> Result<Self, ColorError> {
        let mut size = 0usize;
        let mut data = Vec::new();
        let mut domain_min = [0.0f32; 3];
        let mut domain_max = [1.0f32; 3];

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("TITLE") {
                continue;
            }
            if let Some(rest) = line.strip_prefix("LUT_1D_SIZE") {
                size = rest
                    .trim()
                    .parse()
                    .map_err(|e| ColorError::Parse(format!("bad LUT_1D_SIZE: {}", e)))?;
                continue;
            }
            if let Some(rest) = line.strip_prefix("DOMAIN_MIN") {
                let vals: Vec<f32> = rest
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if vals.len() == 3 {
                    domain_min = [vals[0], vals[1], vals[2]];
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix("DOMAIN_MAX") {
                let vals: Vec<f32> = rest
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if vals.len() == 3 {
                    domain_max = [vals[0], vals[1], vals[2]];
                }
                continue;
            }
            if line.starts_with("LUT_3D_SIZE") {
                return Err(ColorError::InvalidLut("expected 1D LUT, got 3D".into()));
            }

            // Data row
            let vals: Vec<f32> = line
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if vals.len() == 3 {
                data.push([vals[0], vals[1], vals[2]]);
            }
        }

        if size == 0 {
            size = data.len();
        }
        if data.len() != size {
            return Err(ColorError::DimensionMismatch {
                expected: size,
                got: data.len(),
            });
        }
        if size < 2 {
            return Err(ColorError::InvalidLut(
                "LUT must have at least 2 entries".into(),
            ));
        }

        Ok(Self {
            size,
            data,
            domain_min,
            domain_max,
        })
    }

    /// Apply the 1D LUT to an RGB triplet using linear interpolation.
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        for c in 0..3 {
            let range = self.domain_max[c] - self.domain_min[c];
            let t = if range.abs() < 1e-10 {
                0.0
            } else {
                ((rgb[c] - self.domain_min[c]) / range).clamp(0.0, 1.0)
            };
            let idx_f = t * (self.size - 1) as f32;
            let idx_lo = (idx_f as usize).min(self.size - 2);
            let idx_hi = idx_lo + 1;
            let frac = idx_f - idx_lo as f32;
            out[c] = self.data[idx_lo][c] * (1.0 - frac) + self.data[idx_hi][c] * frac;
        }
        out
    }
}

/// 3D Look-Up Table.
#[derive(Debug, Clone)]
pub struct Lut3D {
    pub size: usize,
    pub data: Vec<[f32; 3]>,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
}

impl Lut3D {
    /// Parse a .cube file containing a 3D LUT.
    pub fn from_cube(content: &str) -> Result<Self, ColorError> {
        let mut size = 0usize;
        let mut data = Vec::new();
        let mut domain_min = [0.0f32; 3];
        let mut domain_max = [1.0f32; 3];

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("TITLE") {
                continue;
            }
            if let Some(rest) = line.strip_prefix("LUT_3D_SIZE") {
                size = rest
                    .trim()
                    .parse()
                    .map_err(|e| ColorError::Parse(format!("bad LUT_3D_SIZE: {}", e)))?;
                continue;
            }
            if let Some(rest) = line.strip_prefix("DOMAIN_MIN") {
                let vals: Vec<f32> = rest
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if vals.len() == 3 {
                    domain_min = [vals[0], vals[1], vals[2]];
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix("DOMAIN_MAX") {
                let vals: Vec<f32> = rest
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if vals.len() == 3 {
                    domain_max = [vals[0], vals[1], vals[2]];
                }
                continue;
            }
            if line.starts_with("LUT_1D_SIZE") {
                return Err(ColorError::InvalidLut("expected 3D LUT, got 1D".into()));
            }

            let vals: Vec<f32> = line
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if vals.len() == 3 {
                data.push([vals[0], vals[1], vals[2]]);
            }
        }

        if size == 0 {
            return Err(ColorError::InvalidLut("missing LUT_3D_SIZE".into()));
        }
        let expected = size * size * size;
        if data.len() != expected {
            return Err(ColorError::DimensionMismatch {
                expected,
                got: data.len(),
            });
        }

        Ok(Self {
            size,
            data,
            domain_min,
            domain_max,
        })
    }

    /// Apply the 3D LUT to an RGB triplet using trilinear interpolation.
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let s = self.size;
        let n = (s - 1) as f32;

        let mut coords = [0.0f32; 3];
        for c in 0..3 {
            let range = self.domain_max[c] - self.domain_min[c];
            let t = if range.abs() < 1e-10 {
                0.0
            } else {
                ((rgb[c] - self.domain_min[c]) / range).clamp(0.0, 1.0)
            };
            coords[c] = t * n;
        }

        let r0 = (coords[0] as usize).min(s - 2);
        let g0 = (coords[1] as usize).min(s - 2);
        let b0 = (coords[2] as usize).min(s - 2);
        let r1 = r0 + 1;
        let g1 = g0 + 1;
        let b1 = b0 + 1;
        let fr = coords[0] - r0 as f32;
        let fg = coords[1] - g0 as f32;
        let fb = coords[2] - b0 as f32;

        let idx = |r: usize, g: usize, b: usize| -> usize { r + g * s + b * s * s };

        let c000 = self.data[idx(r0, g0, b0)];
        let c100 = self.data[idx(r1, g0, b0)];
        let c010 = self.data[idx(r0, g1, b0)];
        let c110 = self.data[idx(r1, g1, b0)];
        let c001 = self.data[idx(r0, g0, b1)];
        let c101 = self.data[idx(r1, g0, b1)];
        let c011 = self.data[idx(r0, g1, b1)];
        let c111 = self.data[idx(r1, g1, b1)];

        let mut out = [0.0f32; 3];
        for c in 0..3 {
            let c00 = c000[c] * (1.0 - fr) + c100[c] * fr;
            let c10 = c010[c] * (1.0 - fr) + c110[c] * fr;
            let c01 = c001[c] * (1.0 - fr) + c101[c] * fr;
            let c11 = c011[c] * (1.0 - fr) + c111[c] * fr;
            let c0 = c00 * (1.0 - fg) + c10 * fg;
            let c1 = c01 * (1.0 - fg) + c11 * fg;
            out[c] = c0 * (1.0 - fb) + c1 * fb;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1d_identity_lut() {
        let cube = "LUT_1D_SIZE 3\n0.0 0.0 0.0\n0.5 0.5 0.5\n1.0 1.0 1.0\n";
        let lut = Lut1D::from_cube(cube).unwrap();
        let result = lut.apply([0.5, 0.5, 0.5]);
        assert!((result[0] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_1d_lut_interpolation() {
        let cube = "LUT_1D_SIZE 2\n0.0 0.0 0.0\n1.0 1.0 1.0\n";
        let lut = Lut1D::from_cube(cube).unwrap();
        let result = lut.apply([0.25, 0.5, 0.75]);
        assert!((result[0] - 0.25).abs() < 0.01);
        assert!((result[1] - 0.5).abs() < 0.01);
        assert!((result[2] - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_3d_identity_lut() {
        // 2x2x2 identity LUT
        let cube = "LUT_3D_SIZE 2\n\
            0.0 0.0 0.0\n1.0 0.0 0.0\n\
            0.0 1.0 0.0\n1.0 1.0 0.0\n\
            0.0 0.0 1.0\n1.0 0.0 1.0\n\
            0.0 1.0 1.0\n1.0 1.0 1.0\n";
        let lut = Lut3D::from_cube(cube).unwrap();
        let result = lut.apply([0.5, 0.5, 0.5]);
        assert!((result[0] - 0.5).abs() < 0.01);
        assert!((result[1] - 0.5).abs() < 0.01);
        assert!((result[2] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_1d_wrong_size() {
        let cube = "LUT_1D_SIZE 5\n0.0 0.0 0.0\n1.0 1.0 1.0\n";
        assert!(Lut1D::from_cube(cube).is_err());
    }

    #[test]
    fn test_3d_wrong_type() {
        let cube = "LUT_1D_SIZE 3\n0.0 0.0 0.0\n0.5 0.5 0.5\n1.0 1.0 1.0\n";
        assert!(Lut3D::from_cube(cube).is_err());
    }
}
