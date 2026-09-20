//! View-dependent tick density, always anchored at zero rather than the first
//! visible grid line. This is display LOD, never a limit on world coordinates.
#[derive(Clone, Copy)]
pub(super) struct Spacing {
    pub major: f64,
    pub minor: f64,
}
impl Spacing {
    pub fn new(pixels_per_unit: f64, label_width: f32) -> Option<Self> {
        Self::with_density(pixels_per_unit, label_width, 88.0)
    }
    pub fn with_density(pixels_per_unit: f64, label_width: f32, minimum: f32) -> Option<Self> {
        if !pixels_per_unit.is_finite() || pixels_per_unit <= 0.0 {
            return None;
        }
        let desired = f64::from((label_width + 28.0).max(minimum)) / pixels_per_unit;
        let decade = 10.0_f64.powf(desired.log10().floor());
        let factor = desired / decade;
        let major = decade
            * if factor <= 1.0 {
                1.0
            } else if factor <= 2.0 {
                2.0
            } else if factor <= 5.0 {
                5.0
            } else {
                10.0
            };
        (major.is_finite() && major > 0.0 && major / 5.0 > 0.0).then_some(Self {
            major,
            minor: major / 5.0,
        })
    }
}
pub(super) fn ticks(low: f64, high: f64, step: f64) -> Vec<f64> {
    if !low.is_finite() || !high.is_finite() || !step.is_finite() || step <= 0.0 || high < low {
        return vec![];
    }
    let first = (low / step).ceil();
    let last = (high / step).floor();
    if !first.is_finite() || !last.is_finite() || first + 1.0 == first || last + 1.0 == last {
        return vec![];
    }
    if last < first {
        return vec![];
    }
    (0..=(last - first) as usize)
        .filter_map(|i| {
            let v = (first + i as f64) * step;
            v.is_finite()
                .then_some(if v.abs() < step * 1e-8 { 0.0 } else { v })
        })
        .collect()
}
pub(super) fn label(value: f64, step: f64) -> String {
    if value.abs() < step * 1e-8 {
        return "0".into();
    }
    if value.abs() >= 1e6 || value.abs() < 1e-4 {
        let digits = (value.abs().log10().floor() - step.log10().floor()).clamp(0.0, 15.0) as usize;
        format!("{value:.digits$e}")
    } else {
        let digits = (-step.log10().floor()).clamp(0.0, 15.0) as usize;
        let text = format!("{value:.digits$}");
        if text.contains('.') {
            text.trim_end_matches('0').trim_end_matches('.').into()
        } else {
            text
        }
    }
}
