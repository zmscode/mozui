// mozui image shader — renders textured quads with rounded corners and opacity

struct Uniforms {
    viewport_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var image_texture: texture_2d<f32>;
@group(0) @binding(2) var image_sampler: sampler;

struct ImageInstance {
    @location(0) bounds: vec4<f32>,         // x, y, width, height (physical pixels)
    @location(1) uv: vec4<f32>,             // u_min, v_min, u_max, v_max
    @location(2) corner_radii: vec4<f32>,   // TL, TR, BR, BL (physical pixels)
    @location(3) opacity: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) local_pos: vec2<f32>,       // position within rect [0, size]
    @location(2) rect_size: vec2<f32>,       // width, height
    @location(3) corner_radii: vec4<f32>,
    @location(4) opacity: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: ImageInstance,
) -> VertexOutput {
    var out: VertexOutput;

    let x = instance.bounds.x;
    let y = instance.bounds.y;
    let w = instance.bounds.z;
    let h = instance.bounds.w;

    var pos: vec2<f32>;
    var uv: vec2<f32>;
    var local: vec2<f32>;
    switch vertex_index {
        case 0u: { pos = vec2(x, y);         uv = vec2(instance.uv.x, instance.uv.y); local = vec2(0.0, 0.0); }
        case 1u: { pos = vec2(x, y + h);     uv = vec2(instance.uv.x, instance.uv.w); local = vec2(0.0, h); }
        case 2u: { pos = vec2(x + w, y);     uv = vec2(instance.uv.z, instance.uv.y); local = vec2(w, 0.0); }
        case 3u: { pos = vec2(x + w, y + h); uv = vec2(instance.uv.z, instance.uv.w); local = vec2(w, h); }
        default: { pos = vec2(0.0, 0.0); uv = vec2(0.0, 0.0); local = vec2(0.0, 0.0); }
    }

    let ndc = vec2(
        (pos.x / uniforms.viewport_size.x) * 2.0 - 1.0,
        1.0 - (pos.y / uniforms.viewport_size.y) * 2.0,
    );

    out.position = vec4(ndc, 0.0, 1.0);
    out.uv = uv;
    out.local_pos = local;
    out.rect_size = vec2(w, h);
    out.corner_radii = instance.corner_radii;
    out.opacity = instance.opacity;

    return out;
}

// Rounded rectangle SDF — returns negative inside, positive outside
fn rounded_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    // Select corner radius based on quadrant
    var r: f32;
    if p.x > 0.0 {
        if p.y > 0.0 {
            r = radii.z; // bottom-right
        } else {
            r = radii.y; // top-right
        }
    } else {
        if p.y > 0.0 {
            r = radii.w; // bottom-left
        } else {
            r = radii.x; // top-left
        }
    }
    let q = abs(p) - half_size + vec2(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0, 0.0))) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the image texture
    var color = textureSample(image_texture, image_sampler, in.uv);

    // Apply opacity
    let a = color.a * in.opacity;

    // Rounded corner clipping via SDF
    var mask = 1.0;
    let has_radii = any(in.corner_radii != vec4(0.0));
    if has_radii {
        let half_size = in.rect_size * 0.5;
        let centered = in.local_pos - half_size;
        let dist = rounded_rect_sdf(centered, half_size, in.corner_radii);
        mask = smoothstep(0.5, -0.5, dist);
    }

    let final_a = a * mask;
    // Premultiplied alpha output (input texture is straight alpha)
    return vec4(color.rgb * final_a, final_a);
}
