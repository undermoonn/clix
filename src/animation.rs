use std::time::Instant;

pub const SPEED_MULTIPLIER: f32 = 1.5;

#[derive(Clone, Copy, Debug)]
pub struct ExponentialAnimation {
    value: f32,
    from: f32,
    target: f32,
    speed: f32,
    started_at: Option<Instant>,
}

pub fn scale_seconds(seconds: f32) -> f32 {
    seconds * SPEED_MULTIPLIER
}

pub fn scaled_elapsed_seconds(started_at: Instant, now: Instant) -> f32 {
    scale_seconds(now.duration_since(started_at).as_secs_f32())
}

pub fn linear_progress(started_at: Instant, now: Instant, duration_seconds: f32) -> f32 {
    if duration_seconds <= 0.0 {
        1.0
    } else {
        (scaled_elapsed_seconds(started_at, now) / duration_seconds).clamp(0.0, 1.0)
    }
}

impl ExponentialAnimation {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            from: value,
            target: value,
            speed: 0.0,
            started_at: None,
        }
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn value_at(&self, now: Instant) -> f32 {
        match self.started_at {
            Some(started_at) => {
                let elapsed = scale_seconds(now.duration_since(started_at).as_secs_f32());
                self.target + (self.from - self.target) * (-self.speed * elapsed).exp()
            }
            None => self.value,
        }
    }

    pub fn set_immediate(&mut self, value: f32) {
        self.value = value;
        self.from = value;
        self.target = value;
        self.speed = 0.0;
        self.started_at = None;
    }

    pub fn restart(&mut self, from: f32, target: f32, speed: f32, now: Instant) {
        self.value = from;
        self.from = from;
        self.target = target;
        self.speed = speed;
        self.started_at = Some(now);
    }

    pub fn animate_to(&mut self, target: f32, speed: f32, now: Instant, epsilon: f32) {
        let current = self.value_at(now);
        self.value = current;

        if (current - target).abs() <= epsilon {
            self.set_immediate(target);
            return;
        }

        if self.started_at.is_some()
            && (self.target - target).abs() <= f32::EPSILON
            && (self.speed - speed).abs() <= f32::EPSILON
        {
            return;
        }

        self.from = current;
        self.target = target;
        self.speed = speed;
        self.started_at = Some(now);
    }

    pub fn update(&mut self, now: Instant, epsilon: f32) -> bool {
        let current = self.value_at(now);
        self.value = current;

        if (self.target - current).abs() <= epsilon {
            self.set_immediate(self.target);
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{linear_progress, scale_seconds, ExponentialAnimation};

    #[test]
    fn linear_progress_uses_scaled_elapsed_time() {
        let started_at = Instant::now();
        let now = started_at + Duration::from_millis(150);

        assert_eq!(linear_progress(started_at, now, 0.3), 0.5 * scale_seconds(1.0));
    }

    #[test]
    fn exponential_animation_matches_expected_curve() {
        let started_at = Instant::now();
        let mut animation = ExponentialAnimation::new(0.0);
        animation.restart(0.0, 1.0, 10.0, started_at);

        let now = started_at + Duration::from_secs_f32(1.0 / 60.0);
        assert!(animation.update(now, 0.000_001));

        let expected = 1.0_f32 - (-10.0_f32 / 60.0_f32).exp();
        assert!((animation.value() - expected).abs() < 1e-6);
    }
}