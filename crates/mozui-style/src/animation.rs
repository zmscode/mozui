/// Create a CSS-style cubic bezier easing function.
///
/// Returns a closure that maps `t` in \[0, 1\] to the eased value.
///
/// ```rust,ignore
/// let ease_in_out = cubic_bezier(0.42, 0.0, 0.58, 1.0);
/// assert!((ease_in_out(0.0) - 0.0).abs() < 0.001);
/// assert!((ease_in_out(1.0) - 1.0).abs() < 0.001);
/// ```
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> impl Fn(f32) -> f32 {
    move |t: f32| {
        let t = t.clamp(0.0, 1.0);

        // Newton-Raphson to find the parameter `s` such that bezier_x(s) == t.
        let mut s = t; // initial guess
        for _ in 0..8 {
            let x = bezier(s, x1, x2) - t;
            let dx = bezier_derivative(s, x1, x2);
            if dx.abs() < 1e-6 {
                break;
            }
            s -= x / dx;
            s = s.clamp(0.0, 1.0);
        }

        bezier(s, y1, y2)
    }
}

/// Evaluate the cubic bezier curve at parameter `s` for one axis.
/// The curve is defined by P0=(0,0), P1=(c1,_), P2=(c2,_), P3=(1,1).
fn bezier(s: f32, c1: f32, c2: f32) -> f32 {
    let inv = 1.0 - s;
    3.0 * inv * inv * s * c1 + 3.0 * inv * s * s * c2 + s * s * s
}

/// Derivative of the bezier curve at parameter `s`.
fn bezier_derivative(s: f32, c1: f32, c2: f32) -> f32 {
    let inv = 1.0 - s;
    3.0 * inv * inv * c1 + 6.0 * inv * s * (c2 - c1) + 3.0 * s * s * (1.0 - c2)
}

// Common easing presets

/// CSS `ease` — equivalent to `cubic_bezier(0.25, 0.1, 0.25, 1.0)`.
pub fn ease() -> impl Fn(f32) -> f32 {
    cubic_bezier(0.25, 0.1, 0.25, 1.0)
}

/// CSS `ease-in` — equivalent to `cubic_bezier(0.42, 0.0, 1.0, 1.0)`.
pub fn ease_in() -> impl Fn(f32) -> f32 {
    cubic_bezier(0.42, 0.0, 1.0, 1.0)
}

/// CSS `ease-out` — equivalent to `cubic_bezier(0.0, 0.0, 0.58, 1.0)`.
pub fn ease_out() -> impl Fn(f32) -> f32 {
    cubic_bezier(0.0, 0.0, 0.58, 1.0)
}

/// CSS `ease-in-out` — equivalent to `cubic_bezier(0.42, 0.0, 0.58, 1.0)`.
pub fn ease_in_out() -> impl Fn(f32) -> f32 {
    cubic_bezier(0.42, 0.0, 0.58, 1.0)
}

/// Linear easing (identity).
pub fn linear() -> impl Fn(f32) -> f32 {
    |t: f32| t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_endpoints() {
        let ease = cubic_bezier(0.25, 0.1, 0.25, 1.0);
        assert!((ease(0.0)).abs() < 0.01);
        assert!((ease(1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn linear_is_identity() {
        let f = linear();
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            assert!((f(t) - t).abs() < 0.001);
        }
    }
}
