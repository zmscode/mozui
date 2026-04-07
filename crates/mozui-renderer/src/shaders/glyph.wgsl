// mozui glyph shader — renders text glyphs from atlas texture

struct Uniforms {
    viewport_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct GlyphInstance {
    @location(0) bounds: vec4<f32>,  // x, y, width, height (physical pixels)
    @location(1) uv: vec4<f32>,     // u_min, v_min, u_max, v_max
    @location(2) color: vec4<f32>,  // RGBA text color
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: GlyphInstance,
) -> VertexOutput {
    var out: VertexOutput;

    let x = instance.bounds.x;
    let y = instance.bounds.y;
    let w = instance.bounds.z;
    let h = instance.bounds.w;

    var pos: vec2<f32>;
    var uv: vec2<f32>;
    switch vertex_index {
        case 0u: { pos = vec2(x, y); uv = vec2(instance.uv.x, instance.uv.y); }
        case 1u: { pos = vec2(x, y + h); uv = vec2(instance.uv.x, instance.uv.w); }
        case 2u: { pos = vec2(x + w, y); uv = vec2(instance.uv.z, instance.uv.y); }
        case 3u: { pos = vec2(x + w, y + h); uv = vec2(instance.uv.z, instance.uv.w); }
        default: { pos = vec2(0.0, 0.0); uv = vec2(0.0, 0.0); }
    }

    let ndc = vec2(
        (pos.x / uniforms.viewport_size.x) * 2.0 - 1.0,
        1.0 - (pos.y / uniforms.viewport_size.y) * 2.0,
    );

    out.position = vec4(ndc, 0.0, 1.0);
    out.uv = uv;
    out.color = instance.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(atlas_texture, atlas_sampler, in.uv).r;
    if alpha < 0.01 {
        discard;
    }
    // Premultiplied alpha
    return vec4(in.color.rgb * in.color.a * alpha, in.color.a * alpha);
}
