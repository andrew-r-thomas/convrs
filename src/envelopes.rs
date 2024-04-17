pub enum FadeDirection {
    FadeIn,
    FadeOut,
}

pub fn linear_envelope(i: usize, size: usize, fade: FadeDirection) -> f32 {
    let o = match fade {
        FadeDirection::FadeIn => (i / size) as f32,
        FadeDirection::FadeOut => 1.0 - (i / size) as f32,
    };
    o
}
