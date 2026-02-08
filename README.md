# ProEdit Studio

A professional video editor built from scratch in Rust, combining the power of:
- **Premiere Pro** - Timeline-based editing
- **After Effects** - Layer compositing and motion graphics
- **DaVinci Resolve Fusion** - Node-based compositing
- **InVideo** - AI-powered video generation

## Features (Planned)

- GPU-accelerated rendering via wgpu (Metal on macOS)
- 200+ built-in effects (blur, color grading, keying, etc.)
- Frame-accurate editing with rational time representation
- AI-powered auto-rotoscoping, frame interpolation, and upscaling
- Professional codec support via FFmpeg
- Cross-platform (macOS, Windows, Linux)

## Requirements

- Rust 1.85+
- FFmpeg 6.0+ (for media I/O)
- macOS 13+ / Windows 10+ / Linux with Vulkan support

## Building

```bash
# Clone the repository
git clone https://github.com/JorgGoram/proedit-studio.git
cd proedit-studio

# Build in release mode
cargo build --release

# Run with a video file
cargo run --release -- /path/to/video.mp4
```

## Project Structure

```
proedit-studio/
├── crates/
│   ├── proedit-core/      # Foundation types (time, color, geometry)
│   ├── proedit-media/     # FFmpeg integration
│   ├── proedit-gpu/       # wgpu rendering pipeline
│   ├── proedit-timeline/  # Timeline data model
│   ├── proedit-effects/   # GPU effects library
│   ├── proedit-ui/        # egui widgets
│   ├── proedit-audio/     # Audio engine
│   ├── proedit-ai/        # AI features (optional)
│   └── proedit-app/       # Main application
└── tests/
```

## License

MIT OR Apache-2.0
