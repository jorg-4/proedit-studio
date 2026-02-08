// Simple blit shader - displays a texture as a full-screen quad

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Generate a full-screen quad with 6 vertices (2 triangles)
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),  // Bottom-left
        vec2<f32>(1.0, -1.0),   // Bottom-right
        vec2<f32>(-1.0, 1.0),   // Top-left
        vec2<f32>(-1.0, 1.0),   // Top-left
        vec2<f32>(1.0, -1.0),   // Bottom-right
        vec2<f32>(1.0, 1.0),    // Top-right
    );

    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),    // Bottom-left (flip Y for texture)
        vec2<f32>(1.0, 1.0),    // Bottom-right
        vec2<f32>(0.0, 0.0),    // Top-left
        vec2<f32>(0.0, 0.0),    // Top-left
        vec2<f32>(1.0, 1.0),    // Bottom-right
        vec2<f32>(1.0, 0.0),    // Top-right
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var s_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_texture, s_sampler, input.uv);
}
