use std::time::Duration;

use crownui::prelude::{Cx, Spring};
use motion::{Animation, Curve};

use super::state::Pad;

pub fn spring() -> Spring {
    Spring::new(0.38, 0.86)
}

#[derive(Debug, Clone, Copy)]
pub struct SpringValue {
    animation: Animation<f32>,
    aim: Option<f32>,
    pub value: f32,
}

impl SpringValue {
    pub fn new(value: f32) -> Self {
        Self {
            animation: Animation::new(value, value, 0.0, Duration::ZERO, Curve::Spring(spring())),
            aim: None,
            value,
        }
    }

    pub fn target(&self) -> f32 {
        self.aim.unwrap_or_else(|| self.animation.target())
    }

    pub fn aim(&mut self, target: f32) -> bool {
        let moved = self.target() != target;
        if moved {
            self.aim = Some(target);
        }
        moved
    }

    fn step(&mut self, now: Duration) -> bool {
        if let Some(target) = self.aim.take() {
            self.animation = self
                .animation
                .retarget(now, target, Curve::Spring(spring()));
        }
        let sample = self.animation.sample(now);
        self.value = sample.value;
        !sample.finished
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Motion {
    pub reveal: SpringValue,
    scheduled: bool,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            reveal: SpringValue::new(0.0),
            scheduled: false,
        }
    }
}

pub fn animate(cx: &mut Cx, pad: Pad) {
    let schedule = pad
        .motion
        .update(cx, |motion| !std::mem::replace(&mut motion.scheduled, true));
    if schedule {
        cx.request_animation_frame(move |cx, now| frame(cx, pad, now));
    }
}

fn frame(cx: &mut Cx, pad: Pad, now: Duration) {
    let (moving, reveal) = pad.motion.update(cx, |motion| {
        motion.scheduled = false;
        (motion.reveal.step(now), motion.reveal.value)
    });
    pad.reveal.set(cx, reveal);
    if moving {
        animate(cx, pad);
    }
}
