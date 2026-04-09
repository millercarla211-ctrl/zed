// ============================================================================
//  Gaussian Blur — WGSL port from C++ Blur.glsl
//  Two-pass separable blur (horizontal + vertical)
// ============================================================================

struct Uniforms {
    resolution: vec2<f32>,  // Texture resolution
    direction:  vec2<f32>,  // Blur direction (1,0) or (0,1)
    radius:     f32,        // Blur radius multiplier
    _padding:   f32,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;
@group(0) @binding(1)
var input_texture: texture_2d<f32>;
@group(0) @binding(2)
var input_sampler: sampler;

// ─── Vertex Shader: Fullscreen Triangle ────────────────────────────
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0)       uv:       vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var out: VertexOutput;
    let pos = positions[idx];
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = vec2<f32>(
        (pos.x + 1.0) * 0.5,
        1.0 - (pos.y + 1.0) * 0.5,
    );
    return out;
}

// ─── 13-tap Gaussian Blur ──────────────────────────────────────────
// Exact port from C++ blur13() function
fn blur13(uv: vec2<f32>, resolution: vec2<f32>, direction: vec2<f32>) -> vec4<f32> {
    var color = vec4<f32>(0.0);
    let off1 = vec2<f32>(1.411764705882353) * direction;
    let off2 = vec2<f32>(3.2941176470588234) * direction;
    let off3 = vec2<f32>(5.176470588235294) * direction;
    
    color += textureSample(input_texture, input_sampler, uv) * 0.1964825501511404;
    color += textureSample(input_texture, input_sampler, uv + (off1 / resolution)) * 0.2969069646728344;
    color += textureSample(input_texture, input_sampler, uv - (off1 / resolution)) * 0.2969069646728344;
    color += textureSample(input_texture, input_sampler, uv + (off2 / resolution)) * 0.09447039785044732;
    color += textureSample(input_texture, input_sampler, uv - (off2 / resolution)) * 0.09447039785044732;
    color += textureSample(input_texture, input_sampler, uv + (off3 / resolution)) * 0.010381362401148057;
    color += textureSample(input_texture, input_sampler, uv - (off3 / resolution)) * 0.010381362401148057;
    
    return color;
}

// ─── Fragment Shader ───────────────────────────────────────────────
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return blur13(in.uv, u.resolution, u.direction * u.radius);
}
