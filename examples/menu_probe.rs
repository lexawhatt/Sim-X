//! Short native menu diagnostic, not a controlled release benchmark.
//!
//! Uses the real application with no external-link opener. The default is the
//! same borderless fullscreen/Vsync mode as the application; `--windowed`
//! instead requests 1920 x 1080 logical pixels. `--seconds N` accepts 1..60 and
//! runs for that duration or at most 10,000 frames, whichever comes first.
//! Do not interact during a sample.

use std::time::{Duration, Instant};

use sim_logic::prelude::*;

const SAMPLE_DURATION: Duration = Duration::from_secs(8);
const MAX_INTERVALS: usize = 9_999;

struct Probe {
    started: Option<Instant>,
    previous: Option<Instant>,
    intervals_ms: Vec<f64>,
    duration: Duration,
    completed: bool,
}

impl Probe {
    fn new(duration: Duration) -> Self {
        Self {
            started: None,
            previous: None,
            intervals_ms: Vec::with_capacity(MAX_INTERVALS),
            duration,
            completed: false,
        }
    }
}

fn sample(
    mut probe: AppResMut<Probe>,
    mut commands: Commands,
    viewport: FrameViewport,
) -> LogicResult {
    if probe.completed {
        return Ok(());
    }
    let now = Instant::now();
    let Some(started) = probe.started else {
        probe.started = Some(now);
        probe.previous = Some(now);
        println!(
            "menu_probe: sampling begins; viewport={}x{} logical; initial application/window setup excluded",
            viewport.logical().width(),
            viewport.logical().height(),
        );
        return Ok(());
    };
    if let Some(previous) = probe.previous
        && probe.intervals_ms.len() < MAX_INTERVALS
    {
        probe
            .intervals_ms
            .push(now.duration_since(previous).as_secs_f64() * 1_000.0);
    }
    probe.previous = Some(now);
    let elapsed = now.duration_since(started);
    let frame_limit_reached = probe.intervals_ms.len() == MAX_INTERVALS;
    if elapsed < probe.duration && !frame_limit_reached {
        return Ok(());
    }

    probe.completed = true;
    probe.intervals_ms.sort_unstable_by(f64::total_cmp);
    let count = probe.intervals_ms.len();
    if count > 0 {
        let percentile = |fraction: f64| {
            let index = ((count - 1) as f64 * fraction).round() as usize;
            probe.intervals_ms[index]
        };
        println!(
            "menu_probe: intervals={count} frame_capped={frame_limit_reached} elapsed_s={:.3} approximate_frame_hz={:.2} wall_interval_ms_p50={:.3} wall_interval_ms_p95={:.3}",
            elapsed.as_secs_f64(),
            count as f64 / elapsed.as_secs_f64(),
            percentile(0.5),
            percentile(0.95),
        );
    }
    println!(
        "menu_probe: Vsync/compositor pacing may cap cadence; includes first-present warmup; intervals are end-to-end frame cadence, not GPU timings"
    );
    commands.request_exit()?;
    Ok(())
}

fn main() -> LogicResult {
    let mut windowed = false;
    let mut duration = SAMPLE_DURATION;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--windowed" => windowed = true,
            "--seconds" => {
                let seconds: u64 = arguments
                    .next()
                    .ok_or("--seconds needs a value in 1..60")?
                    .parse()?;
                if !(1..=60).contains(&seconds) {
                    return Err("--seconds must be in 1..60".into());
                }
                duration = Duration::from_secs(seconds);
            }
            _ => {
                return Err(format!(
                    "unknown argument {argument}; supported: --windowed, --seconds N"
                )
                .into());
            }
        }
    }
    let (mut application, initial) = sim_x::build_application()?;
    application.register_app_resource(Probe::new(duration))?;
    application.add_fallible_frame_system(sample);
    let mut desktop = DesktopConfig::new("Sim;X menu probe", 1920.0, 1080.0)?;
    if !windowed {
        desktop.set_window_mode(WindowMode::BorderlessFullscreen(
            FullscreenMonitor::Automatic,
        ));
    }
    println!(
        "menu_probe: profile={} mode={} source=current_worktree sample_s={} max_intervals={MAX_INTERVALS}",
        if cfg!(debug_assertions) {
            "dev"
        } else {
            "release"
        },
        if windowed {
            "windowed"
        } else {
            "borderless-fullscreen"
        },
        duration.as_secs(),
    );
    let report = application.run_desktop(initial, desktop)?;
    println!(
        "menu_probe: logic_frames={} drawn={} skipped={} device_recoveries={} exit={:?}",
        report.logic_frames(),
        report.drawn_frames(),
        report.skipped_frames(),
        report.device_recoveries(),
        report.exit_reason(),
    );
    if let Some(frame) = report.last_render_frame() {
        println!("menu_probe: last_render={frame:?}");
    }
    Ok(())
}
