//! Analytic spring integration; retargeting preserves position and velocity.
#[derive(Clone, Copy, Debug)]
pub struct SpringParameters {
    pub mass: f32,
    pub stiffness: f32,
    pub damping: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub position: f32,
    pub velocity: f32,
}

impl Spring {
    pub fn new(position: f32) -> Self {
        Self {
            position,
            velocity: 0.,
        }
    }

    pub fn step(&mut self, target: f32, dt: f32, omega: f32) {
        let dt = dt.max(0.);
        let displacement = self.position - target;
        let c = self.velocity + omega * displacement;
        let decay = (-omega * dt).exp();
        self.position = target + (displacement + c * dt) * decay;
        self.velocity = (self.velocity - omega * c * dt) * decay;
    }

    pub fn settled(&self, target: f32) -> bool {
        (self.position - target).abs() < 0.05 && self.velocity.abs() < 0.1
    }

    /// Solve m*x'' + c*x' + k*(x-target) = 0 for a constant target.
    /// Handles under-, over- and critically damped springs without frame steps.
    pub fn step_physical(&mut self, target: f32, dt: f32, parameters: SpringParameters) {
        let dt = dt.max(0.);
        if dt == 0. {
            return;
        }
        let decay_rate = parameters.damping / (2. * parameters.mass);
        let omega_squared = parameters.stiffness / parameters.mass;
        let discriminant = omega_squared - decay_rate * decay_rate;
        let x = self.position - target;
        let v = self.velocity;
        let (position, velocity) = if discriminant.abs() < 0.0001 {
            let c = v + decay_rate * x;
            let decay = (-decay_rate * dt).exp();
            ((x + c * dt) * decay, (v - decay_rate * c * dt) * decay)
        } else if discriminant > 0. {
            let frequency = discriminant.sqrt();
            let (sin, cos) = (frequency * dt).sin_cos();
            let b = (v + decay_rate * x) / frequency;
            let oscillation = x * cos + b * sin;
            let decay = (-decay_rate * dt).exp();
            (
                oscillation * decay,
                (frequency * (-x * sin + b * cos) - decay_rate * oscillation) * decay,
            )
        } else {
            let root = (-discriminant).sqrt();
            let r1 = -decay_rate + root;
            let r2 = -decay_rate - root;
            let a = (v - r2 * x) / (r1 - r2);
            let b = x - a;
            let e1 = (r1 * dt).exp();
            let e2 = (r2 * dt).exp();
            (a * e1 + b * e2, a * r1 * e1 + b * r2 * e2)
        };
        self.position = target + position;
        self.velocity = velocity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_springs_are_frame_rate_independent_in_all_damping_regimes() {
        for damping in [7., 18., 20., 50.] {
            let parameters = SpringParameters {
                mass: 1.,
                stiffness: 100.,
                damping,
            };
            let mut coarse = Spring::new(0.);
            let mut fine = coarse;
            for _ in 0..30 {
                coarse.step_physical(100., 1. / 30., parameters);
            }
            for _ in 0..144 {
                fine.step_physical(100., 1. / 144., parameters);
            }
            assert!((coarse.position - fine.position).abs() < 0.003);
            assert!((coarse.velocity - fine.velocity).abs() < 0.003);
            let before = (fine.position, fine.velocity);
            fine.step_physical(-100., 0., parameters);
            assert!((fine.position - before.0).abs() < 0.0001);
            assert!((fine.velocity - before.1).abs() < 0.0001);
        }
    }
    #[test]
    fn frame_rate_independent_and_retarget_continuous() {
        let mut a = Spring::new(0.);
        let mut b = a;
        for _ in 0..60 {
            a.step(100., 1. / 60., 14.);
        }
        for _ in 0..120 {
            b.step(100., 1. / 120., 14.);
        }
        assert!((a.position - b.position).abs() < 0.001);
        let old = a.position;
        a.step(-100., 0., 14.);
        assert_eq!(a.position, old);
        a.step(-100., 1., 14.);
        assert!(a.settled(-100.));
    }
}
