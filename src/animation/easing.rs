pub(crate) fn smoothstep01(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(crate) fn lerp_f32(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}
