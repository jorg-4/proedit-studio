//! Lock-free single-producer single-consumer ring buffer for real-time audio.
//!
//! Designed for the audio callback thread (consumer) and the mixer
//! thread (producer). No mutexes — uses atomic fences only.

use std::sync::atomic::{AtomicUsize, Ordering};

/// A SPSC ring buffer for f32 audio samples.
pub struct RingBuffer {
    buffer: Box<[f32]>,
    capacity: usize,
    read_pos: AtomicUsize,
    write_pos: AtomicUsize,
}

// SAFETY: The ring buffer is designed for SPSC use. The read_pos and write_pos
// are accessed via atomics, and the buffer segments accessed by reader and
// writer never overlap (guaranteed by available_read/available_write).
unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

impl RingBuffer {
    /// Create a new ring buffer with the given capacity (in samples).
    pub fn new(capacity: usize) -> Self {
        // Add 1 to distinguish full from empty
        let actual_cap = capacity + 1;
        Self {
            buffer: vec![0.0f32; actual_cap].into_boxed_slice(),
            capacity: actual_cap,
            read_pos: AtomicUsize::new(0),
            write_pos: AtomicUsize::new(0),
        }
    }

    /// Number of samples available for reading.
    pub fn available_read(&self) -> usize {
        let w = self.write_pos.load(Ordering::Acquire);
        let r = self.read_pos.load(Ordering::Acquire);
        if w >= r {
            w - r
        } else {
            self.capacity - r + w
        }
    }

    /// Number of samples that can be written.
    pub fn available_write(&self) -> usize {
        self.capacity - 1 - self.available_read()
    }

    /// Write samples into the buffer. Returns number actually written.
    pub fn write(&self, data: &[f32]) -> usize {
        let available = self.available_write();
        let count = data.len().min(available);
        if count == 0 {
            return 0;
        }

        let w = self.write_pos.load(Ordering::Relaxed);

        // Write in up to two segments (wrap-around)
        let first_chunk = (self.capacity - w).min(count);
        let second_chunk = count - first_chunk;

        // SAFETY: We only write to positions between write_pos and read_pos
        // (the "free" region), and the reader only reads from the other region.
        let buf_ptr = self.buffer.as_ptr() as *mut f32;
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), buf_ptr.add(w), first_chunk);
            if second_chunk > 0 {
                std::ptr::copy_nonoverlapping(data[first_chunk..].as_ptr(), buf_ptr, second_chunk);
            }
        }

        let new_w = (w + count) % self.capacity;
        self.write_pos.store(new_w, Ordering::Release);

        count
    }

    /// Read samples from the buffer. Returns number actually read.
    pub fn read(&self, output: &mut [f32]) -> usize {
        let available = self.available_read();
        let count = output.len().min(available);
        if count == 0 {
            return 0;
        }

        let r = self.read_pos.load(Ordering::Relaxed);

        let first_chunk = (self.capacity - r).min(count);
        let second_chunk = count - first_chunk;

        unsafe {
            let buf_ptr = self.buffer.as_ptr();
            std::ptr::copy_nonoverlapping(buf_ptr.add(r), output.as_mut_ptr(), first_chunk);
            if second_chunk > 0 {
                std::ptr::copy_nonoverlapping(
                    buf_ptr,
                    output[first_chunk..].as_mut_ptr(),
                    second_chunk,
                );
            }
        }

        let new_r = (r + count) % self.capacity;
        self.read_pos.store(new_r, Ordering::Release);

        count
    }

    /// Clear the buffer.
    pub fn clear(&self) {
        self.read_pos
            .store(self.write_pos.load(Ordering::Acquire), Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_write_read() {
        let rb = RingBuffer::new(1024);
        let data: Vec<f32> = (0..100).map(|i| i as f32).collect();
        assert_eq!(rb.write(&data), 100);
        assert_eq!(rb.available_read(), 100);

        let mut output = vec![0.0f32; 100];
        assert_eq!(rb.read(&mut output), 100);
        assert_eq!(output, data);
        assert_eq!(rb.available_read(), 0);
    }

    #[test]
    fn test_wrap_around() {
        let rb = RingBuffer::new(16);

        // Fill most of the buffer
        let data: Vec<f32> = (0..12).map(|i| i as f32).collect();
        assert_eq!(rb.write(&data), 12);

        // Read some
        let mut out = vec![0.0f32; 8];
        assert_eq!(rb.read(&mut out), 8);

        // Write more (should wrap around)
        let data2: Vec<f32> = (100..112).map(|i| i as f32).collect();
        assert_eq!(rb.write(&data2), 12);

        // Read everything
        let mut out2 = vec![0.0f32; 16];
        let read = rb.read(&mut out2);
        assert_eq!(read, 16);
        // First 4 from original write (indices 8-11), then 12 from second write
        assert_eq!(out2[0], 8.0);
        assert_eq!(out2[4], 100.0);
    }

    #[test]
    fn test_overflow_protection() {
        let rb = RingBuffer::new(8);
        let data: Vec<f32> = (0..20).map(|i| i as f32).collect();
        // Can only write 8 (capacity)
        let written = rb.write(&data);
        assert_eq!(written, 8);
    }

    #[test]
    fn test_empty_read() {
        let rb = RingBuffer::new(16);
        let mut out = vec![0.0f32; 8];
        assert_eq!(rb.read(&mut out), 0);
    }

    #[test]
    fn test_clear() {
        let rb = RingBuffer::new(16);
        let data = vec![1.0f32; 10];
        rb.write(&data);
        assert_eq!(rb.available_read(), 10);
        rb.clear();
        assert_eq!(rb.available_read(), 0);
    }
}
