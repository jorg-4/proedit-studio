// Color space transform shader
// Applies a 3x3 matrix transform and transfer function

struct ColorTransformUniforms {
    matrix: mat3x3<f32>,
    transfer_fn: u32,    // 0=Linear, 1=sRGB, 2=Rec709, 3=PQ, 4=HLG
    direction: u32,      // 0=to_linear, 1=from_linear
    _padding: vec2<u32>,
};

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: ColorTransformUniforms;

fn srgb_to_linear(v: f32) -> f32 {
    if (v <= 0.04045) {
        return v / 12.92;
    }
    return pow((v + 0.055) / 1.055, 2.4);
}

fn linear_to_srgb(v: f32) -> f32 {
    if (v <= 0.0031308) {
        return v * 12.92;
    }
    return 1.055 * pow(v, 1.0 / 2.4) - 0.055;
}

fn apply_transfer_to_linear(v: f32) -> f32 {
    switch (params.transfer_fn) {
        case 0u: { return v; }
        case 1u: { return srgb_to_linear(v); }
        case 2u: {
            if (v < 0.081) { return v / 4.5; }
            return pow((v + 0.099) / 1.099, 1.0 / 0.45);
        }
        default: { return srgb_to_linear(v); }
    }
}

fn apply_transfer_from_linear(v: f32) -> f32 {
    switch (params.transfer_fn) {
        case 0u: { return v; }
        case 1u: { return linear_to_srgb(v); }
        case 2u: {
            if (v < 0.018) { return v * 4.5; }
            return 1.099 * pow(v, 0.45) - 0.099;
        }
        default: { return linear_to_srgb(v); }
    }
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    if (global_id.x >= dims.x || global_id.y >= dims.y) {
        return;
    }

    let pixel = textureLoad(input_texture, vec2<i32>(global_id.xy), 0);
    var rgb = pixel.rgb;

    if (params.direction == 0u) {
        rgb = vec3<f32>(
            apply_transfer_to_linear(rgb.r),
            apply_transfer_to_linear(rgb.g),
            apply_transfer_to_linear(rgb.b),
        );
    }

    rgb = params.matrix * rgb;

    if (params.direction == 1u) {
        rgb = vec3<f32>(
            apply_transfer_from_linear(rgb.r),
            apply_transfer_from_linear(rgb.g),
            apply_transfer_from_linear(rgb.b),
        );
    }

    textureStore(output_texture, vec2<i32>(global_id.xy), vec4<f32>(rgb, pixel.a));
}
