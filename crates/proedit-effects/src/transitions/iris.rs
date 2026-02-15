use crate::transition::Transition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum IrisShape {
    #[default]
    Circle,
    Rectangle,
    Diamond,
}

pub struct Iris {
    pub shape: IrisShape,
}

impl Default for Iris {
    fn default() -> Self {
        Self {
            shape: IrisShape::Circle,
        }
    }
}

impl Transition for Iris {
    fn name(&self) -> &str {
        "Iris"
    }

    fn render(&self, a: &[u8], b: &[u8], w: u32, h: u32, progress: f32) -> Vec<u8> {
        let size = (w * h * 4) as usize;
        let mut out = vec![0u8; size];
        let p = progress.clamp(0.0, 1.0);
        let cx = w as f32 * 0.5;
        let cy = h as f32 * 0.5;
        let max_radius = (cx * cx + cy * cy).sqrt();

        for y in 0..h {
            for x in 0..w {
                let idx = ((y * w + x) * 4) as usize;
                let fx = x as f32 - cx;
                let fy = y as f32 - cy;

                let dist = match self.shape {
                    IrisShape::Circle => (fx * fx + fy * fy).sqrt() / max_radius,
                    IrisShape::Rectangle => (fx.abs() / cx).max(fy.abs() / cy),
                    IrisShape::Diamond => (fx.abs() / cx + fy.abs() / cy) * 0.5,
                };

                let use_b = dist < p;
                for c in 0..4 {
                    out[idx + c] = if use_b {
                        b.get(idx + c).copied().unwrap_or(0)
                    } else {
                        a.get(idx + c).copied().unwrap_or(0)
                    };
                }
            }
        }
        out
    }
}
