//! Color transform pipeline — chains of color operations.

use crate::color_space::ColorSpace;
use crate::lut::{Lut1D, Lut3D};
use crate::transfer::TransferFunction;

/// A single color operation in the pipeline.
pub enum ColorOp {
    MatrixTransform([[f32; 3]; 3]),
    TransferToLinear(TransferFunction),
    TransferFromLinear(TransferFunction),
    Lut1D(Lut1D),
    Lut3D(Lut3D),
}

/// A color transform pipeline.
pub struct ColorPipeline {
    pub ops: Vec<ColorOp>,
    pub input_space: ColorSpace,
    pub working_space: ColorSpace,
    pub output_space: ColorSpace,
}

impl ColorPipeline {
    /// Create a new pipeline with the given spaces.
    pub fn new(input: ColorSpace, working: ColorSpace, output: ColorSpace) -> Self {
        Self {
            ops: Vec::new(),
            input_space: input,
            working_space: working,
            output_space: output,
        }
    }

    /// Auto-generate the operation chain for input→working→output.
    pub fn build_ops(&mut self) {
        self.ops.clear();

        // Input → working (via XYZ)
        if self.input_space != self.working_space {
            if !self.input_space.is_linear() {
                self.ops.push(ColorOp::TransferToLinear(transfer_for_space(
                    &self.input_space,
                )));
            }
            // Input RGB → XYZ
            self.ops
                .push(ColorOp::MatrixTransform(self.input_space.to_xyz_matrix()));
            // XYZ → Working RGB
            self.ops.push(ColorOp::MatrixTransform(
                self.working_space.from_xyz_matrix(),
            ));
        }

        // Working → output (via XYZ)
        if self.working_space != self.output_space {
            self.ops
                .push(ColorOp::MatrixTransform(self.working_space.to_xyz_matrix()));
            self.ops.push(ColorOp::MatrixTransform(
                self.output_space.from_xyz_matrix(),
            ));
            if !self.output_space.is_linear() {
                self.ops
                    .push(ColorOp::TransferFromLinear(transfer_for_space(
                        &self.output_space,
                    )));
            }
        }
    }

    /// Process a single pixel through the pipeline.
    pub fn process_pixel(&self, mut rgb: [f32; 3]) -> [f32; 3] {
        for op in &self.ops {
            rgb = match op {
                ColorOp::MatrixTransform(m) => mat3_mul(m, rgb),
                ColorOp::TransferToLinear(tf) => [
                    tf.to_linear(rgb[0]),
                    tf.to_linear(rgb[1]),
                    tf.to_linear(rgb[2]),
                ],
                ColorOp::TransferFromLinear(tf) => [
                    tf.from_linear(rgb[0]),
                    tf.from_linear(rgb[1]),
                    tf.from_linear(rgb[2]),
                ],
                ColorOp::Lut1D(lut) => lut.apply(rgb),
                ColorOp::Lut3D(lut) => lut.apply(rgb),
            };
        }
        rgb
    }

    /// Process a batch of pixels.
    pub fn process_buffer(&self, data: &mut [[f32; 3]]) {
        for pixel in data.iter_mut() {
            *pixel = self.process_pixel(*pixel);
        }
    }
}

fn mat3_mul(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn transfer_for_space(space: &ColorSpace) -> TransferFunction {
    match space {
        ColorSpace::SRGB => TransferFunction::SRGB,
        ColorSpace::Rec709 => TransferFunction::Rec709,
        ColorSpace::Rec2020 => TransferFunction::Rec709, // Rec2020 uses similar OETF
        ColorSpace::DciP3 => TransferFunction::Gamma(2.6),
        ColorSpace::ACEScct => TransferFunction::Linear, // ACEScct has its own log, simplified
        _ => TransferFunction::Linear,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_pipeline() {
        let mut pipe = ColorPipeline::new(ColorSpace::SRGB, ColorSpace::SRGB, ColorSpace::SRGB);
        pipe.build_ops();
        assert!(pipe.ops.is_empty());
        let pixel = [0.5, 0.3, 0.8];
        let result = pipe.process_pixel(pixel);
        assert!((result[0] - pixel[0]).abs() < 0.001);
    }

    #[test]
    fn test_srgb_to_acescg_pipeline() {
        let mut pipe = ColorPipeline::new(ColorSpace::SRGB, ColorSpace::ACEScg, ColorSpace::SRGB);
        pipe.build_ops();
        assert!(!pipe.ops.is_empty());
        let pixel = [0.5, 0.5, 0.5];
        let result = pipe.process_pixel(pixel);
        // Should come back approximately to the original (with roundtrip error)
        assert!((result[0] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_process_buffer() {
        let pipe = ColorPipeline::new(ColorSpace::SRGB, ColorSpace::SRGB, ColorSpace::SRGB);
        let mut data = vec![[0.5, 0.3, 0.8], [1.0, 0.0, 0.0]];
        pipe.process_buffer(&mut data);
        assert!((data[0][0] - 0.5).abs() < 0.001);
    }
}
