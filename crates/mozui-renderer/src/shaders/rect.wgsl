// mozui rect shader — SDF-based rounded rectangle rendering
//
// Each instance is a rounded rectangle with solid color fill,
// optional border, rendered with anti-aliased edges via SDF.

struct Uniforms {
    viewport_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct RectInstance {
    @location(0) bounds: vec4<f32>,        // x, y, width, height (physical pixels)
    @location(1) color: vec4<f32>,         // RGBA fill color
    @location(2) corner_radii: vec4<f32>,  // TL, TR, BR, BL
    @location(3) border_width: f32,
    @location(4) border_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,     // Position relative to rect center
    @location(1) half_size: vec2<f32>,     // Half of rect size
    @location(2) color: vec4<f32>,
    @location(3) corner_radii: vec4<f32>,
    @location(4) border_width: f32,
    @location(5) border_color: vec4<f32>,
};

// Vertex indices for a quad: 0=TL, 1=TR, 2=BL, 3=BR
// Triangle strip: 0, 2, 1, 3

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: RectInstance,
) -> VertexOutput {
    var out: VertexOutput;

    // Expand instance to quad corners
    let x = instance.bounds.x;
    let y = instance.bounds.y;
    let w = instance.bounds.z;
    let h = instance.bounds.w;

    // Pad by 1px for anti-aliasing
    let pad = 1.0;
    let px = x - pad;
    let py = y - pad;
    let pw = w + pad * 2.0;
    let ph = h + pad * 2.0;

    var pos: vec2<f32>;
    switch vertex_index {
        case 0u: { pos = vec2(px, py); }           // Top-left
        case 1u: { pos = vec2(px, py + ph); }      // Bottom-left
        case 2u: { pos = vec2(px + pw, py); }      // Top-right
        case 3u: { pos = vec2(px + pw, py + ph); }  // Bottom-right
        default: { pos = vec2(0.0, 0.0); }
    }

    // Convert from pixel coordinates to clip space
    // Pixel (0,0) = top-left, Clip (-1,-1) = bottom-left
    let ndc = vec2(
        (pos.x / uniforms.viewport_size.x) * 2.0 - 1.0,
        1.0 - (pos.y / uniforms.viewport_size.y) * 2.0,
    );

    out.position = vec4(ndc, 0.0, 1.0);

    // Local position relative to rect center
    let center = vec2(x + w * 0.5, y + h * 0.5);
    out.local_pos = pos - center;
    out.half_size = vec2(w * 0.5, h * 0.5);

    out.color = instance.color;
    out.corner_radii = instance.corner_radii;
    out.border_width = instance.border_width;
    out.border_color = instance.border_color;

    return out;
}

// Signed distance function for a rounded rectangle
fn rounded_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    // Select corner radius based on quadrant
    // radii: TL, TR, BR, BL
    var r: f32;
    if p.x > 0.0 {
        if p.y < 0.0 {
            r = radii.y;  // Top-right (y < 0 = top in our local coords since center-relative)
        } else {
            r = radii.z;  // Bottom-right
        }
    } else {
        if p.y < 0.0 {
            r = radii.x;  // Top-left
        } else {
            r = radii.w;  // Bottom-left
        }
    }

    let q = abs(p) - half_size + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = rounded_rect_sdf(in.local_pos, in.half_size, in.corner_radii);

    // Anti-aliased edge (0.5px smoothstep)
    let aa = 0.75;
    let fill_alpha = 1.0 - smoothstep(-aa, aa, dist);

    if in.border_width > 0.0 {
        // Border: between outer edge and inner edge
        let inner_dist = dist + in.border_width;
        let border_alpha = (1.0 - smoothstep(-aa, aa, dist)) * smoothstep(-aa, aa, inner_dist);
        let fill_inner_alpha = 1.0 - smoothstep(-aa, aa, inner_dist);

        let fill_color = vec4(in.color.rgb * in.color.a, in.color.a) * fill_inner_alpha;
        let border_color_pm = vec4(in.border_color.rgb * in.border_color.a, in.border_color.a) * border_alpha;

        let combined = fill_color + border_color_pm;
        if combined.a < 0.001 {
            discard;
        }
        return combined;
    } else {
        // No border — just fill
        if fill_alpha < 0.001 {
            discard;
        }
        // Premultiplied alpha
        return vec4(in.color.rgb * in.color.a * fill_alpha, in.color.a * fill_alpha);
    }
}
