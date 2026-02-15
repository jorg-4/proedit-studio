// Chroma key compute shader
// Extracts a soft matte based on color distance in YCbCr space

struct ChromaKeyUniforms {
    key_color: vec3<f32>,
    tolerance: f32,
    softness: f32,
    _padding: vec3<f32>,
};

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: ChromaKeyUniforms;

fn rgb_to_ycbcr(rgb: vec3<f32>) -> vec3<f32> {
    let y = 0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b;
    let cb = -0.168736 * rgb.r - 0.331264 * rgb.g + 0.5 * rgb.b;
    let cr = 0.5 * rgb.r - 0.418688 * rgb.g - 0.081312 * rgb.b;
    return vec3<f32>(y, cb, cr);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    if (global_id.x >= dims.x || global_id.y >= dims.y) {
        return;
    }

    let pixel = textureLoad(input_texture, vec2<i32>(global_id.xy), 0);
    let ycbcr = rgb_to_ycbcr(pixel.rgb);

    let dcb = ycbcr.y - params.key_color.y;
    let dcr = ycbcr.z - params.key_color.z;
    let dist = sqrt(dcb * dcb + dcr * dcr);

    var alpha: f32;
    if (dist < params.tolerance) {
        alpha = 0.0;
    } else if (dist < params.tolerance + params.softness) {
        alpha = (dist - params.tolerance) / params.softness;
    } else {
        alpha = 1.0;
    }

    textureStore(output_texture, vec2<i32>(global_id.xy), vec4<f32>(pixel.rgb, alpha));
}
