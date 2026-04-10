// mozui rect shader — SDF-based rounded rectangle rendering
//
// Each instance is a rounded rectangle with solid color or gradient fill,
// optional border, rendered with anti-aliased edges via SDF.
// Shadows are rendered as separate rect instances by the CPU.

struct Uniforms {
    viewport_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct RectInstance {
    @location(0) bounds: vec4<f32>,        // x, y, width, height (physical pixels)
    @location(1) color: vec4<f32>,         // RGBA fill color (or gradient stop 0)
    @location(2) corner_radii: vec4<f32>,  // TL, TR, BR, BL
    @location(3) border_width: f32,
    @location(4) border_color: vec4<f32>,
    @location(5) fill_mode: u32,           // 0=solid, 1=linear gradient, 2=radial gradient
    @location(6) gradient_angle: f32,      // radians for linear gradient
    @location(7) gradient_stop1_color: vec4<f32>, // second gradient stop color
    @location(8) gradient_stop0_pos: f32,  // position of first stop [0,1]
    @location(9) gradient_stop1_pos: f32,  // position of second stop [0,1]
    @location(10) shadow_blur: f32,        // SDF blur radius (0 = normal rect)
    @location(11) shadow_inset: f32,      // 1.0 = inset shadow, 0.0 = outer shadow
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,     // Position relative to rect center
    @location(1) half_size: vec2<f32>,     // Half of rect size
    @location(2) color: vec4<f32>,
    @location(3) corner_radii: vec4<f32>,
    @location(4) border_width: f32,
    @location(5) border_color: vec4<f32>,
    @location(6) @interpolate(flat) fill_mode: u32,
    @location(7) gradient_angle: f32,
    @location(8) gradient_stop1_color: vec4<f32>,
    @location(9) gradient_stop0_pos: f32,
    @location(10) gradient_stop1_pos: f32,
    @location(11) uv: vec2<f32>,           // [0,1] normalized position within rect
    @location(12) shadow_blur: f32,
    @location(13) shadow_inset: f32,
};

// Vertex indices for a quad: 0=TL, 1=BL, 2=TR, 3=BR (triangle strip)

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

    // Pad by 1px for anti-aliasing. For shadows, expand by 3*sigma
    // to cover the full Gaussian tail (sigma = shadow_blur).
    let pad = 1.0 + instance.shadow_blur * 3.0;
    let px = x - pad;
    let py = y - pad;
    let pw = w + pad * 2.0;
    let ph = h + pad * 2.0;

    var pos: vec2<f32>;
    var uv: vec2<f32>;
    switch vertex_index {
        case 0u: { pos = vec2(px, py);           uv = vec2(0.0, 0.0); }
        case 1u: { pos = vec2(px, py + ph);      uv = vec2(0.0, 1.0); }
        case 2u: { pos = vec2(px + pw, py);      uv = vec2(1.0, 0.0); }
        case 3u: { pos = vec2(px + pw, py + ph); uv = vec2(1.0, 1.0); }
        default: { pos = vec2(0.0, 0.0);         uv = vec2(0.0, 0.0); }
    }

    // Convert from pixel coordinates to clip space
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
    out.fill_mode = instance.fill_mode;
    out.gradient_angle = instance.gradient_angle;
    out.gradient_stop1_color = instance.gradient_stop1_color;
    out.gradient_stop0_pos = instance.gradient_stop0_pos;
    out.gradient_stop1_pos = instance.gradient_stop1_pos;
    out.uv = uv;
    out.shadow_blur = instance.shadow_blur;
    out.shadow_inset = instance.shadow_inset;

    return out;
}

// Signed distance function for a rounded rectangle
fn rounded_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    // Select corner radius based on quadrant
    // radii: TL, TR, BR, BL
    var r: f32;
    if p.x > 0.0 {
        if p.y < 0.0 {
            r = radii.y;  // Top-right
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

// ── Shadow helper ──────────────────────────────────────────────

// Scalar error function approximation (Horner's method, max error ~5e-4).
// Used to compute the Gaussian CDF for smooth shadow falloff.
fn erf_scalar(x: f32) -> f32 {
    let s = sign(x);
    let a = abs(x);
    let t = 1.0 + (0.278393 + (0.230389 + (0.000972 + 0.078108 * a) * a) * a) * a;
    let r = t * t;
    return s * (1.0 - 1.0 / (r * r));
}

/// Compute the fill color based on fill mode and UV coordinates.
fn compute_fill_color(
    solid_color: vec4<f32>,
    fill_mode: u32,
    gradient_angle: f32,
    stop1_color: vec4<f32>,
    stop0_pos: f32,
    stop1_pos: f32,
    uv: vec2<f32>,
) -> vec4<f32> {
    if fill_mode == 1u {
        // Linear gradient
        let cos_a = cos(gradient_angle);
        let sin_a = sin(gradient_angle);
        // Project UV onto gradient direction. UV is [0,1] over rect.
        // Center UV at (0.5, 0.5) for rotation, then project onto direction.
        let centered = uv - vec2(0.5, 0.5);
        let projected = centered.x * cos_a + centered.y * sin_a + 0.5;
        // Interpolate between stops
        let range = max(stop1_pos - stop0_pos, 0.001);
        let t = clamp((projected - stop0_pos) / range, 0.0, 1.0);
        return mix(solid_color, stop1_color, t);
    } else if fill_mode == 2u {
        // Radial gradient (center of rect outward)
        let dist = length(uv - vec2(0.5, 0.5)) * 2.0; // 0 at center, ~1.41 at corners
        let range = max(stop1_pos - stop0_pos, 0.001);
        let t = clamp((dist - stop0_pos) / range, 0.0, 1.0);
        return mix(solid_color, stop1_color, t);
    }
    // Solid fill
    return solid_color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Shadow mode: Gaussian CDF applied to the SDF distance.
    // The SDF gives the distance to the rounded rect boundary.
    // Applying the Gaussian CDF (via erf) produces a smooth, natural falloff.
    if in.shadow_blur > 0.0 {
        let sigma = in.shadow_blur;
        let dist = rounded_rect_sdf(in.local_pos, in.half_size, in.corner_radii);

        if in.shadow_inset > 0.5 {
            // Inset shadow: visible only inside the rect, fading inward from edges.
            // Negate the distance so the Gaussian fades from the inside edge inward.
            let fill = 1.0 - smoothstep(-0.75, 0.75, dist);
            let inner_alpha = 0.5 * (1.0 - erf_scalar(-dist * 0.7071067811865476 / sigma));
            let alpha = in.color.a * inner_alpha * fill;
            if alpha < 0.001 { discard; }
            return vec4(in.color.rgb * alpha, alpha);
        } else {
            // Outer shadow: visible outside the rect, fading outward.
            // Gaussian CDF: 0.5 * (1 - erf(dist / (sigma * sqrt(2))))
            // Inside rect (dist < 0): alpha ≈ 1.0
            // On boundary (dist = 0): alpha = 0.5
            // Outside rect (dist > 0): alpha fades smoothly to 0
            let alpha = in.color.a * 0.5 * (1.0 - erf_scalar(dist * 0.7071067811865476 / sigma));
            if alpha < 0.001 { discard; }
            return vec4(in.color.rgb * alpha, alpha);
        }
    }

    let dist = rounded_rect_sdf(in.local_pos, in.half_size, in.corner_radii);

    // Anti-aliased edge (0.75px smoothstep)
    let aa = 0.75;
    let fill_alpha = 1.0 - smoothstep(-aa, aa, dist);

    // Compute the fill color (solid or gradient)
    let base_color = compute_fill_color(
        in.color,
        in.fill_mode,
        in.gradient_angle,
        in.gradient_stop1_color,
        in.gradient_stop0_pos,
        in.gradient_stop1_pos,
        in.uv,
    );

    if in.border_width > 0.0 {
        // Border: between outer edge and inner edge
        let inner_dist = dist + in.border_width;
        let border_alpha = (1.0 - smoothstep(-aa, aa, dist)) * smoothstep(-aa, aa, inner_dist);
        let fill_inner_alpha = 1.0 - smoothstep(-aa, aa, inner_dist);

        let fill_color = vec4(base_color.rgb * base_color.a, base_color.a) * fill_inner_alpha;
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
        return vec4(base_color.rgb * base_color.a * fill_alpha, base_color.a * fill_alpha);
    }
}
