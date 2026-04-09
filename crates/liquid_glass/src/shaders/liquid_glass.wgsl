// ============================================================================
//  Liquid Glass — WGSL port from C++ BatchRenderer2D.glsl
//
//  Key difference from C++: Instead of vertex attributes (v_MidPoint, 
//  v_QuadNDC2ScreenNDCScale), we use uniforms since wgpu doesn't have 
//  the same vertex attribute system.
// ============================================================================

struct Uniforms {
    // vec4 slot 0
    resolution:    vec2<f32>,   // Window size in pixels
    glass_pos:     vec2<f32>,   // Glass center in pixels

    // vec4 slot 1
    glass_size:    vec2<f32>,   // Glass width,height in pixels
    time:          f32,
    power_factor:  f32,         // Superellipse exponent

    // vec4 slot 2
    a:             f32,         // f(x) parameter a
    b:             f32,         // f(x) parameter b
    c:             f32,         // f(x) parameter c
    d:             f32,         // f(x) parameter d

    // vec4 slot 3
    f_power:       f32,         // Distortion power
    noise:         f32,         // Noise intensity
    glow_weight:   f32,         // Glow multiplier
    glow_edge0:    f32,         // Glow smoothstep inner

    // vec4 slot 4
    glow_edge1:    f32,         // Glow smoothstep outer
    glow_bias:     f32,         // Glow additive bias
    chromatic_aberration: f32,  // Chromatic aberration strength
    aberration_samples:   f32,  // Number of samples for quality
};

@group(0) @binding(0)
var<uniform> u: Uniforms;
@group(0) @binding(1)
var bg_texture:  texture_2d<f32>;
@group(0) @binding(2)
var bg_sampler:  sampler;

// ─── Vertex Shader: Fullscreen Triangle ────────────────────────────
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0)       uv:       vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    // Generate a single triangle covering the entire screen
    // Vertices: (-1,-1), (3,-1), (-1,3) → covers [-1,1] clip space
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var out: VertexOutput;
    let pos = positions[idx];
    out.position = vec4<f32>(pos, 0.0, 1.0);

    // Clip space [-1,1] → UV [0,1], Y flipped for texture convention
    out.uv = vec2<f32>(
        (pos.x + 1.0) * 0.5,
        1.0 - (pos.y + 1.0) * 0.5,
    );
    return out;
}

// ─── Constants ─────────────────────────────────────────────────────
const M_E: f32 = 2.718281828459045;

// ─── Superellipse SDF ──────────────────────────────────────────────
// Exact port from C++ GLSL sdSuperellipse
fn sd_superellipse(p: vec2<f32>, n: f32, r: f32) -> f32 {
    let p_abs = abs(p);
    let numerator = pow(p_abs.x, n) + pow(p_abs.y, n) - pow(r, n);
    let den_x = pow(p_abs.x, 2.0 * n - 2.0);
    let den_y = pow(p_abs.y, 2.0 * n - 2.0);
    let denominator = n * sqrt(den_x + den_y) + 0.00001;
    return numerator / denominator;
}

// ─── f(x) distortion function ──────────────────────────────────────
// Exact port from C++ GLSL f(float x)
fn f_dist(x: f32) -> f32 {
    return 1.0 - u.b * pow(u.c * M_E, -u.d * x - u.a);
}

// ─── Hash-based Pseudo-random ──────────────────────────────────────
// Exact port from C++ GLSL rand(vec2 co)
fn rand(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// ─── Glow Function ─────────────────────────────────────────────────
// Exact port from C++ GLSL Glow()
// Uses v_TexCoord which is the screen UV in our case
fn glow_func(uv: vec2<f32>) -> f32 {
    return sin(atan2(uv.y * 2.0 - 1.0, uv.x * 2.0 - 1.0) - 0.5);
}

// ─── Fragment Shader: The Complete Liquid Glass Effect ──────────────
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // ════════════════════════════════════════════════════════════════
    // STEP 1: Transform screen UV to glass-local coordinates
    // ════════════════════════════════════════════════════════════════
    // This is equivalent to the C++ code that uses v_MidPoint and 
    // v_QuadNDC2ScreenNDCScale, but we compute it from uniforms.
    
    // Convert glass position/size to UV space
    let glass_center_uv = u.glass_pos / u.resolution;
    let glass_half_uv = u.glass_size / (2.0 * u.resolution);
    
    // Transform screen UV to glass-local [-1,1] space
    // This is the "forward transform" - screen → glass-local
    let glass_local = (in.uv - glass_center_uv) / glass_half_uv;
    
    // ════════════════════════════════════════════════════════════════
    // STEP 2: SDF - Check if we're inside or outside the glass
    // ════════════════════════════════════════════════════════════════
    
    let d = sd_superellipse(glass_local, u.power_factor, 1.0);
    
    // Outside the glass → show background directly
    if (d > 0.0) {
        return textureSample(bg_texture, bg_sampler, in.uv);
    }
    
    // dist = distance INSIDE the glass (positive value)
    let dist = -d;
    
    // ════════════════════════════════════════════════════════════════
    // STEP 3: Apply distortion function f(dist)
    // ════════════════════════════════════════════════════════════════
    // This creates the refraction effect by warping coordinates
    
    let refraction_amount = pow(f_dist(dist), u.f_power);
    let sample_p = glass_local * refraction_amount;
    
    // ════════════════════════════════════════════════════════════════
    // STEP 4: Transform distorted glass-local back to screen UV
    // ════════════════════════════════════════════════════════════════
    // This is the "inverse transform" - glass-local → screen
    // This is the CRITICAL step that was missing before!
    
    let sample_uv = sample_p * glass_half_uv + glass_center_uv;
    
    // Clamp to valid texture range
    let safe_uv = clamp(sample_uv, vec2<f32>(0.001), vec2<f32>(0.999));
    
    // ════════════════════════════════════════════════════════════════
    // STEP 5: Sample with Chromatic Aberration (Apple Liquid Glass)
    // ════════════════════════════════════════════════════════════════
    // Chromatic aberration separates RGB colors to create a prismatic effect
    // Effect is stronger at edges for more dramatic look
    
    var color: vec4<f32>;
    
    if (u.chromatic_aberration > 0.0001) {
        // Calculate edge factor - stronger aberration near edges
        // dist is the distance inside the glass (0 at edge, higher at center)
        let edge_factor = 1.0 - smoothstep(0.0, 0.3, dist);  // Strong at edges, weak at center
        let aberration_strength = u.chromatic_aberration * edge_factor * 3.0;  // 3x multiplier for solid effect
        
        // Calculate aberration direction (radial from glass center)
        let aberration_dir = normalize(sample_p);
        let samples = i32(u.aberration_samples);
        
        var r_accum = 0.0;
        var g_accum = 0.0;
        var b_accum = 0.0;
        
        // Sample multiple times with offset for each color channel
        for (var i = 0; i < samples; i++) {
            let t = f32(i) / f32(max(samples - 1, 1));
            let offset = (t - 0.5) * aberration_strength;
            
            // Red channel - shift outward (most)
            let r_uv = safe_uv + aberration_dir * offset * 2.0;
            r_accum += textureSample(bg_texture, bg_sampler, clamp(r_uv, vec2<f32>(0.001), vec2<f32>(0.999))).r;
            
            // Green channel - slight shift (middle)
            let g_uv = safe_uv + aberration_dir * offset * 0.8;
            g_accum += textureSample(bg_texture, bg_sampler, clamp(g_uv, vec2<f32>(0.001), vec2<f32>(0.999))).g;
            
            // Blue channel - shift inward (least)
            let b_uv = safe_uv - aberration_dir * offset * 1.5;
            b_accum += textureSample(bg_texture, bg_sampler, clamp(b_uv, vec2<f32>(0.001), vec2<f32>(0.999))).b;
        }
        
        color = vec4<f32>(
            r_accum / f32(samples),
            g_accum / f32(samples),
            b_accum / f32(samples),
            1.0
        );
    } else {
        // No chromatic aberration - simple sample
        color = textureSample(bg_texture, bg_sampler, safe_uv);
    }
    
    // ════════════════════════════════════════════════════════════════
    // STEP 6: Add noise
    // ════════════════════════════════════════════════════════════════
    
    let noise_val = (rand(in.position.xy * 0.001) - 0.5) * u.noise;
    color += vec4<f32>(vec3<f32>(noise_val), 0.0);
    
    // ════════════════════════════════════════════════════════════════
    // STEP 7: Apply glow (rim lighting)
    // ════════════════════════════════════════════════════════════════
    
    let glow_val = glow_func(in.uv);
    let glow_mask = smoothstep(u.glow_edge0, u.glow_edge1, dist);
    let mul = glow_val * u.glow_weight * glow_mask + 1.0 + u.glow_bias;
    
    color = vec4<f32>(color.rgb * vec3<f32>(mul), 1.0);
    
    return color;
}
