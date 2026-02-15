use crate::transition::Transition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum WipeDirection {
    #[default]
    Left,
    Right,
    Up,
    Down,
}

pub struct Wipe {
    pub direction: WipeDirection,
}

impl Default for Wipe {
    fn default() -> Self {
        Self {
            direction: WipeDirection::Left,
        }
    }
}

impl Transition for Wipe {
    fn name(&self) -> &str {
        "Wipe"
    }

    fn render(&self, a: &[u8], b: &[u8], w: u32, h: u32, progress: f32) -> Vec<u8> {
        let size = (w * h * 4) as usize;
        let mut out = vec![0u8; size];
        let p = progress.clamp(0.0, 1.0);

        for y in 0..h {
            for x in 0..w {
                let idx = ((y * w + x) * 4) as usize;
                let threshold = match self.direction {
                    WipeDirection::Left => x as f32 / w as f32,
                    WipeDirection::Right => 1.0 - x as f32 / w as f32,
                    WipeDirection::Up => y as f32 / h as f32,
                    WipeDirection::Down => 1.0 - y as f32 / h as f32,
                };

                let use_b = threshold < p;
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
