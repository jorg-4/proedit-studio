use crate::transition::Transition;

pub struct CrossDissolve;

impl Transition for CrossDissolve {
    fn name(&self) -> &str {
        "Cross Dissolve"
    }

    fn render(&self, a: &[u8], b: &[u8], w: u32, h: u32, progress: f32) -> Vec<u8> {
        let size = (w * h * 4) as usize;
        let mut out = vec![0u8; size];
        let p = progress.clamp(0.0, 1.0);
        let ip = 1.0 - p;

        for ((out_px, a_px), b_px) in out.iter_mut().zip(a.iter()).zip(b.iter()) {
            *out_px = (*a_px as f32 * ip + *b_px as f32 * p) as u8;
        }
        out
    }
}
