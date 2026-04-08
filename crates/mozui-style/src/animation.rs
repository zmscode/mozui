use crate::Color;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

// ── Lerp trait ────────────────────────────────────────────────────

/// Types that can be linearly interpolated between two values.
pub trait Lerp: Clone + PartialEq {
    fn lerp(&self, target: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        self + (target - self) * t
    }
}

impl Lerp for f64 {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        self + (target - self) * t as f64
    }
}

impl Lerp for Color {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        Color::new(
            self.r + (target.r - self.r) * t,
            self.g + (target.g - self.g) * t,
            self.b + (target.b - self.b) * t,
            self.a + (target.a - self.a) * t,
        )
    }
}

// ── Transition ────────────────────────────────────────────────────

/// Describes how a value should animate between states.
#[derive(Clone)]
pub struct Transition {
    pub duration: Duration,
    easing: EasingFn,
}

#[derive(Clone)]
enum EasingFn {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    Custom(f32, f32, f32, f32),
}

impl Transition {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            easing: EasingFn::EaseInOut,
        }
    }

    pub fn linear(mut self) -> Self {
        self.easing = EasingFn::Linear;
        self
    }

    pub fn ease(mut self) -> Self {
        self.easing = EasingFn::Ease;
        self
    }

    pub fn ease_in(mut self) -> Self {
        self.easing = EasingFn::EaseIn;
        self
    }

    pub fn ease_out(mut self) -> Self {
        self.easing = EasingFn::EaseOut;
        self
    }

    pub fn ease_in_out(mut self) -> Self {
        self.easing = EasingFn::EaseInOut;
        self
    }

    pub fn custom_bezier(mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        self.easing = EasingFn::Custom(x1, y1, x2, y2);
        self
    }

    /// Evaluate the easing function at progress `t` in [0, 1].
    pub fn ease_t(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self.easing {
            EasingFn::Linear => t,
            EasingFn::Ease => cubic_bezier(0.25, 0.1, 0.25, 1.0)(t),
            EasingFn::EaseIn => cubic_bezier(0.42, 0.0, 1.0, 1.0)(t),
            EasingFn::EaseOut => cubic_bezier(0.0, 0.0, 0.58, 1.0)(t),
            EasingFn::EaseInOut => cubic_bezier(0.42, 0.0, 0.58, 1.0)(t),
            EasingFn::Custom(x1, y1, x2, y2) => cubic_bezier(x1, y1, x2, y2)(t),
        }
    }
}

// ── AnimatedValue ─────────────────────────────────────────────────

/// A value that smoothly transitions when its target changes.
///
/// Call `set()` to change the target — the `get()` value will smoothly
/// animate from the old value to the new one over the transition duration.
pub struct AnimatedValue<T: Lerp> {
    from: T,
    to: T,
    current: T,
    transition: Transition,
    start_time: Option<Instant>,
    done: bool,
}

impl<T: Lerp> AnimatedValue<T> {
    pub fn new(initial: T, transition: Transition) -> Self {
        Self {
            from: initial.clone(),
            to: initial.clone(),
            current: initial,
            transition,
            start_time: None,
            done: true,
        }
    }

    /// Set a new target value, starting the animation from the current value.
    /// If the target is the same as the current target, this is a no-op.
    pub fn set(&mut self, target: T) {
        if self.to == target {
            return;
        }
        self.from = self.current.clone();
        self.to = target;
        self.start_time = Some(Instant::now());
        self.done = false;
    }

    /// Set the value immediately without animating.
    pub fn set_immediate(&mut self, value: T) {
        self.from = value.clone();
        self.to = value.clone();
        self.current = value;
        self.start_time = None;
        self.done = true;
    }

    /// Get the current interpolated value, advancing the animation.
    pub fn get(&mut self) -> &T {
        if self.done {
            return &self.current;
        }

        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            let duration = self.transition.duration;

            if elapsed >= duration {
                self.current = self.to.clone();
                self.done = true;
                self.start_time = None;
            } else {
                let raw_t = elapsed.as_secs_f32() / duration.as_secs_f32();
                let eased_t = self.transition.ease_t(raw_t);
                self.current = self.from.lerp(&self.to, eased_t);
            }
        }

        &self.current
    }

    /// Returns true if the animation is still in progress.
    pub fn is_animating(&self) -> bool {
        !self.done
    }

    /// Returns the target value (what we're animating toward).
    pub fn target(&self) -> &T {
        &self.to
    }
}

// ── Spring ────────────────────────────────────────────────────────

/// A spring-based animation for more natural interactive feel.
/// Uses a damped harmonic oscillator model.
pub struct Spring {
    value: f32,
    velocity: f32,
    target: f32,
    stiffness: f32,
    damping: f32,
    mass: f32,
    threshold: f32,
    done: bool,
}

impl Spring {
    pub fn new(initial: f32) -> Self {
        Self {
            value: initial,
            velocity: 0.0,
            target: initial,
            stiffness: 170.0,
            damping: 26.0,
            mass: 1.0,
            threshold: 0.01,
            done: true,
        }
    }

    /// Presets for common spring configurations.
    pub fn gentle(mut self) -> Self {
        self.stiffness = 120.0;
        self.damping = 14.0;
        self
    }

    pub fn wobbly(mut self) -> Self {
        self.stiffness = 180.0;
        self.damping = 12.0;
        self
    }

    pub fn stiff(mut self) -> Self {
        self.stiffness = 210.0;
        self.damping = 20.0;
        self
    }

    pub fn with_stiffness(mut self, stiffness: f32) -> Self {
        self.stiffness = stiffness;
        self
    }

    pub fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping;
        self
    }

    /// Set a new target for the spring.
    pub fn set(&mut self, target: f32) {
        self.target = target;
        self.done = false;
    }

    /// Advance the spring by `dt` seconds. Returns the current value.
    pub fn tick(&mut self, dt: f32) -> f32 {
        if self.done {
            return self.value;
        }

        // Damped harmonic oscillator: F = -k*x - d*v
        let displacement = self.value - self.target;
        let spring_force = -self.stiffness * displacement;
        let damping_force = -self.damping * self.velocity;
        let acceleration = (spring_force + damping_force) / self.mass;

        self.velocity += acceleration * dt;
        self.value += self.velocity * dt;

        // Settle when close enough and slow enough
        if displacement.abs() < self.threshold && self.velocity.abs() < self.threshold {
            self.value = self.target;
            self.velocity = 0.0;
            self.done = true;
        }

        self.value
    }

    pub fn get(&self) -> f32 {
        self.value
    }

    pub fn is_animating(&self) -> bool {
        !self.done
    }

    pub fn target(&self) -> f32 {
        self.target
    }
}

// ── Animated Handle ───────────────────────────────────────────────

/// A shared handle to an animated value, with interior mutability.
///
/// Cloning this handle creates another reference to the same animation.
/// Use `get()` to read the current interpolated value, and `set()` to
/// change the target (starting a smooth transition).
#[derive(Clone)]
pub struct Animated<T: Lerp> {
    inner: Rc<RefCell<AnimatedValue<T>>>,
    /// Shared flag — set to `true` when any animation is in progress.
    active_flag: Rc<Cell<bool>>,
}

impl<T: Lerp> Animated<T> {
    pub fn new(initial: T, transition: Transition, active_flag: Rc<Cell<bool>>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(AnimatedValue::new(initial, transition))),
            active_flag,
        }
    }

    /// Get the current interpolated value, advancing the animation by elapsed time.
    pub fn get(&self) -> T {
        let mut inner = self.inner.borrow_mut();
        let val = inner.get().clone();
        if inner.is_animating() {
            self.active_flag.set(true);
        }
        val
    }

    /// Set a new target value, starting a smooth transition from current.
    pub fn set(&self, target: T) {
        self.inner.borrow_mut().set(target);
        self.active_flag.set(true);
    }

    /// Set the value immediately without animating.
    pub fn set_immediate(&self, value: T) {
        self.inner.borrow_mut().set_immediate(value);
    }

    /// Returns true if the animation is still in progress.
    pub fn is_animating(&self) -> bool {
        self.inner.borrow().is_animating()
    }

    /// Returns the target value.
    pub fn target(&self) -> T {
        self.inner.borrow().target().clone()
    }
}

/// A shared handle to a spring animation.
#[derive(Clone)]
pub struct SpringHandle {
    inner: Rc<RefCell<Spring>>,
    last_tick: Rc<Cell<Option<Instant>>>,
    active_flag: Rc<Cell<bool>>,
}

impl SpringHandle {
    pub fn new(initial: f32, active_flag: Rc<Cell<bool>>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Spring::new(initial))),
            last_tick: Rc::new(Cell::new(None)),
            active_flag,
        }
    }

    /// Get the current spring value, advancing by elapsed time since last call.
    pub fn get(&self) -> f32 {
        let now = Instant::now();
        let dt = match self.last_tick.get() {
            Some(last) => now.duration_since(last).as_secs_f32().min(0.05),
            None => 0.0,
        };
        self.last_tick.set(Some(now));

        let mut inner = self.inner.borrow_mut();
        if !inner.is_animating() {
            return inner.get();
        }
        let val = inner.tick(dt);
        if inner.is_animating() {
            self.active_flag.set(true);
        }
        val
    }

    /// Set a new target for the spring.
    pub fn set(&self, target: f32) {
        self.inner.borrow_mut().set(target);
        self.active_flag.set(true);
    }

    /// Returns true if the spring is still moving.
    pub fn is_animating(&self) -> bool {
        self.inner.borrow().is_animating()
    }

    /// Configure the spring with a builder method.
    pub fn configure(&self, f: impl FnOnce(Spring) -> Spring) {
        let mut inner = self.inner.borrow_mut();
        let spring = std::mem::replace(&mut *inner, Spring::new(0.0));
        *inner = f(spring);
    }
}

// ── Cubic Bezier ──────────────────────────────────────────────────

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
