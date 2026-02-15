use crate::transition::Transition;

pub struct DipToWhite;

impl Transition for DipToWhite {
    fn name(&self) -> &str {
        "Dip to White"
    }

    fn render(&self, a: &[u8], b: &[u8], w: u32, h: u32, progress: f32) -> Vec<u8> {
        let size = (w * h * 4) as usize;
        let mut out = vec![0u8; size];
        let p = progress.clamp(0.0, 1.0);

        if p < 0.5 {
            // Fade A to white
            let fade = p * 2.0;
            for (i, (out_px, a_px)) in out.iter_mut().zip(a.iter()).enumerate() {
                *out_px = if i % 4 == 3 {
                    255
                } else {
                    let src = *a_px as f32;
                    (src + (255.0 - src) * fade) as u8
                };
            }
        } else {
            // Fade white to B
            let fade = (p - 0.5) * 2.0;
            for (i, (out_px, b_px)) in out.iter_mut().zip(b.iter()).enumerate() {
                *out_px = if i % 4 == 3 {
                    255
                } else {
                    let dst = *b_px as f32;
                    (255.0 + (dst - 255.0) * fade) as u8
                };
            }
        }
        out
    }
}
