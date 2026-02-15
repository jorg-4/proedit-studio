use crate::transition::Transition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum PushDirection {
    #[default]
    Left,
    Right,
    Up,
    Down,
}

pub struct Push {
    pub direction: PushDirection,
}

impl Default for Push {
    fn default() -> Self {
        Self {
            direction: PushDirection::Left,
        }
    }
}

impl Transition for Push {
    fn name(&self) -> &str {
        "Push"
    }

    fn render(&self, a: &[u8], b: &[u8], w: u32, h: u32, progress: f32) -> Vec<u8> {
        let size = (w * h * 4) as usize;
        let mut out = vec![0u8; size];
        let p = progress.clamp(0.0, 1.0);
        let w_i = w as i32;
        let h_i = h as i32;

        for y in 0..h_i {
            for x in 0..w_i {
                let idx = ((y * w_i + x) * 4) as usize;
                let (src_x_a, src_y_a, src_x_b, src_y_b) = match self.direction {
                    PushDirection::Left => {
                        let offset = (w as f32 * p) as i32;
                        (x + offset, y, x + offset - w_i, y)
                    }
                    PushDirection::Right => {
                        let offset = (w as f32 * p) as i32;
                        (x - offset, y, x - offset + w_i, y)
                    }
                    PushDirection::Up => {
                        let offset = (h as f32 * p) as i32;
                        (x, y + offset, x, y + offset - h_i)
                    }
                    PushDirection::Down => {
                        let offset = (h as f32 * p) as i32;
                        (x, y - offset, x, y - offset + h_i)
                    }
                };

                // Try to sample from A, then from B
                if src_x_a >= 0 && src_x_a < w_i && src_y_a >= 0 && src_y_a < h_i {
                    let src_idx = ((src_y_a * w_i + src_x_a) * 4) as usize;
                    for c in 0..4 {
                        out[idx + c] = a.get(src_idx + c).copied().unwrap_or(0);
                    }
                } else if src_x_b >= 0 && src_x_b < w_i && src_y_b >= 0 && src_y_b < h_i {
                    let src_idx = ((src_y_b * w_i + src_x_b) * 4) as usize;
                    for c in 0..4 {
                        out[idx + c] = b.get(src_idx + c).copied().unwrap_or(0);
                    }
                }
            }
        }
        out
    }
}
