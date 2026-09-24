//! Latest-request worker. Scientific computation stays off the UI thread.
use crate::math_editor::scene::clip;
pub(super) mod integrals;
mod surfaces;

use sim_math::{
    Expression, Integral, IntegralKind, IntegrationJob, MathError,
    calculus::IntegralExpressionJob,
    geometry::Point,
    statement::{Statement, definition},
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Clone)]
pub(super) struct Request {
    pub sources: Vec<Result<String, String>>,
    pub calculate: Vec<bool>,
    pub lower: Point,
    pub upper: Point,
    pub pixels: [f32; 2],
    pub spatial: bool,
}
#[derive(Clone, Default)]
pub(super) struct RowPlot {
    pub points: Vec<Option<Point>>,
    pub segments: Vec<[Point; 2]>,
    pub scalar: Option<f64>,
    pub diagnostic: Option<String>,
    pub integral: Option<(Integral, [f64; 2])>,
    pub expression: Option<Expression>,
    pub pending: bool,
    pub surface: Vec<Vec<Option<[f64; 3]>>>,
    pub boundary: Option<sim_math::relation::Relation>,
    pub integral_plot: Option<integrals::IntegralPlot>,
}
#[derive(Clone, Default)]
pub(super) struct Plot {
    pub rows: Vec<RowPlot>,
}
struct Task {
    request: Request,
    alive: Arc<AtomicBool>,
}
#[derive(Default)]
struct Mailbox {
    task: Option<Task>,
    result: Option<Plot>,
    stop: bool,
}
pub(super) struct Worker {
    shared: Arc<(Mutex<Mailbox>, Condvar)>,
    alive: Arc<AtomicBool>,
}
impl Worker {
    pub fn new() -> std::io::Result<Self> {
        let shared = Arc::new((Mutex::new(Mailbox::default()), Condvar::new()));
        let cloned = shared.clone();
        std::thread::Builder::new()
            .name("sim-math".into())
            .spawn(move || run(cloned))?;
        Ok(Self {
            shared,
            alive: Arc::new(AtomicBool::new(false)),
        })
    }
    pub fn cancel(&mut self) {
        self.alive.store(false, Ordering::Release);
        let mut m = self.shared.0.lock().unwrap_or_else(|e| e.into_inner());
        m.task = None;
        m.result = None;
    }
    pub fn submit(&mut self, request: Request) {
        self.cancel();
        self.alive = Arc::new(AtomicBool::new(true));
        self.shared.0.lock().unwrap_or_else(|e| e.into_inner()).task = Some(Task {
            request,
            alive: self.alive.clone(),
        });
        self.shared.1.notify_one();
    }
    pub fn poll(&self) -> Option<Plot> {
        self.shared
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .result
            .take()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
        self.shared.0.lock().unwrap_or_else(|e| e.into_inner()).stop = true;
        self.shared.1.notify_one();
    }
}
type Shared = Arc<(Mutex<Mailbox>, Condvar)>;
fn publish(shared: &Shared, alive: &AtomicBool, plot: &Plot) {
    let mut mailbox = shared.0.lock().unwrap_or_else(|e| e.into_inner());
    if alive.load(Ordering::Acquire) && !mailbox.stop {
        mailbox.result = Some(plot.clone());
    }
}
fn run(shared: Shared) {
    let mut cache = CalculationCache::default();
    loop {
        let task = {
            let mut m = shared.0.lock().unwrap_or_else(|e| e.into_inner());
            while m.task.is_none() && !m.stop {
                m = shared.1.wait(m).unwrap_or_else(|e| e.into_inner());
            }
            if m.stop {
                return;
            }
            m.task.take()
        };
        let Some(task) = task else {
            continue;
        };
        let alive = || task.alive.load(Ordering::Acquire);
        cache.prepare(&task.request);
        let (mut plot, mut jobs) = build_plot(&task.request, &alive);
        jobs.retain(|(index, _, bounds, graph_result)| {
            if let Some(result) = cache.get(*index) {
                complete(
                    &mut plot.rows[*index],
                    result,
                    *bounds,
                    *graph_result,
                    &task.request,
                );
                false
            } else {
                true
            }
        });
        publish(&shared, &task.alive, &plot);
        // Each row keeps its own result. Other graphs appear before quadrature.
        while !jobs.is_empty() && alive() {
            let mut remaining = vec![];
            for (index, mut job, bounds, graph_result) in jobs {
                if !alive() {
                    break;
                }
                match job.advance(1) {
                    Ok(None) => remaining.push((index, job, bounds, graph_result)),
                    Ok(Some(result)) => {
                        if alive() {
                            cache.record(index, result);
                        }
                        complete(
                            &mut plot.rows[index],
                            result,
                            bounds,
                            graph_result,
                            &task.request,
                        );
                    }
                    Err(error) => {
                        plot.rows[index].diagnostic = Some(error.to_string());
                        plot.rows[index].pending = false;
                    }
                }
            }
            jobs = remaining;
            publish(&shared, &task.alive, &plot);
        }
    }
}
fn complete(
    row: &mut RowPlot,
    (value, integral): (f64, Option<Integral>),
    bounds: Option<[f64; 2]>,
    graph_result: bool,
    request: &Request,
) {
    row.scalar = Some(value);
    row.integral = integral.zip(bounds);
    row.pending = false;
    if graph_result {
        row.segments.push([
            Point {
                x: request.lower.x,
                y: value,
            },
            Point {
                x: request.upper.x,
                y: value,
            },
        ]);
    }
}
type Jobs = Vec<(usize, Calculation, Option<[f64; 2]>, bool)>;
fn build_plot(request: &Request, alive: &impl Fn() -> bool) -> (Plot, Jobs) {
    let mut plot = Plot {
        rows: vec![RowPlot::default(); request.sources.len()],
    };
    let mut variables = BTreeMap::new();
    let mut definitions = BTreeMap::<String, Vec<(usize, &str)>>::new();
    for (index, source) in request.sources.iter().enumerate() {
        if let Ok(source) = source
            && let Some((name, value)) = definition(source)
        {
            definitions.entry(name).or_default().push((index, value));
        }
    }
    let mut unresolved = vec![];
    for (name, rows) in definitions {
        if rows.len() > 1 {
            for (index, _) in rows {
                plot.rows[index].diagnostic = Some(format!("Duplicate definition of {name}"));
            }
        } else {
            unresolved.push((name, rows[0].0, rows[0].1));
        }
    }
    loop {
        let before = unresolved.len();
        unresolved.retain(|(name, index, source)| {
            if let Ok(value) = Expression::constant_with(source, &variables) {
                variables.insert(name.clone(), value);
                plot.rows[*index].scalar = Some(value);
                false
            } else {
                true
            }
        });
        if unresolved.len() == before || !alive() {
            break;
        }
    }
    for (name, index, _) in unresolved {
        plot.rows[index].diagnostic = Some(format!(
            "Cannot resolve {name}: invalid, missing or cyclic dependency"
        ));
    }
    let mut jobs = vec![];
    for (index, source) in request.sources.iter().enumerate() {
        if !alive() {
            break;
        }
        let row = &mut plot.rows[index];
        let source = match source {
            Ok(s) => s,
            Err(e) => {
                row.diagnostic = Some(e.clone());
                continue;
            }
        };
        if source.is_empty() || definition(source).is_some() {
            continue;
        }
        let result = (|| {
            match Statement::parse(source, &variables)? {
                Statement::Scalar(value) => row.scalar = Some(value),
                Statement::Implicit3d(relation) => {
                    surfaces::implicit(row, &relation, request, alive)?
                }
                Statement::Surface(f) => {
                    if request.spatial {
                        let bounds = clip::Bounds::from_xy(request.lower, request.upper)
                            .ok_or(sim_math::MathError::NumericRange)?;
                        let count = (request.pixels[0].min(request.pixels[1]) * 0.56 / 40.0)
                            .ceil()
                            .max(2.0) as usize;
                        row.surface = sim_math::surface::wireframe(
                            &f,
                            Point {
                                x: bounds.ranges[0][0],
                                y: bounds.ranges[1][0],
                            },
                            Point {
                                x: bounds.ranges[0][1],
                                y: bounds.ranges[1][1],
                            },
                            [count; 2],
                            8,
                            alive,
                        )?;
                        row.diagnostic = Some("Sampled 3D wireframe".into());
                    } else {
                        row.diagnostic = Some("Switch to 3D to view z = f(x,y)".into());
                    }
                }
                Statement::Curve(f) => {
                    row.points = sample(&f, request, false, false, alive);
                    row.expression = Some(f);
                }
                Statement::InverseCurve(f) => row.points = sample(&f, request, true, false, alive),
                Statement::Derivative(f) => {
                    row.points = sample(&f, request, false, true, alive);
                    row.diagnostic = Some("Numerical derivative (not symbolic)".into());
                }
                Statement::Implicit(f) => {
                    row.segments = sim_math::contour::contours(
                        &f,
                        request.lower,
                        request.upper,
                        [
                            (request.pixels[0] / 7.0).ceil() as usize,
                            (request.pixels[1] / 7.0).ceil() as usize,
                        ],
                        alive,
                    )?;
                }
                Statement::Integral {
                    body,
                    lower,
                    upper,
                    graph_result,
                } => {
                    if !graph_result {
                        row.integral_plot = Some(integrals::IntegralPlot::prepare(
                            sim_math::calculus::IntegralVisualization::single(
                                body.clone(),
                                [lower, upper],
                            ),
                            request,
                            alive,
                        )?);
                    }
                    if request.calculate.get(index).copied().unwrap_or(false) {
                        let job = IntegrationJob::new(
                            body,
                            lower,
                            upper,
                            variables.get("a").copied().unwrap_or(0.0),
                            IntegralKind::Signed,
                            1e-8,
                        )?;
                        row.pending = true;
                        jobs.push((
                            index,
                            Calculation::Integral(Box::new(job)),
                            Some([lower, upper]),
                            graph_result,
                        ));
                    }
                }
                Statement::IntegralExpression {
                    expression,
                    graph_result,
                } => {
                    if !graph_result {
                        row.integral_plot = Some(integrals::IntegralPlot::prepare(
                            expression.visualization(),
                            request,
                            alive,
                        )?);
                    }
                    if request.calculate.get(index).copied().unwrap_or(false) {
                        jobs.push((
                            index,
                            Calculation::Scalar(Box::new(expression.into_job(1e-8)?)),
                            None,
                            graph_result,
                        ));
                        row.pending = true;
                    }
                }
            }
            Ok::<_, sim_math::MathError>(())
        })();
        if let Err(error) = result {
            row.diagnostic = Some(error.to_string());
        } else if !row.points.is_empty() && row.diagnostic.is_none() {
            if !row.points.iter().any(Option::is_some) {
                row.diagnostic = Some("No finite real values in this view".into());
            } else if !row
                .points
                .windows(2)
                .any(|p| p[0].is_some() && p[1].is_some())
            {
                row.diagnostic = Some("Cannot safely connect samples in this view".into());
            }
        }
    }
    (plot, jobs)
}

pub(super) fn sample(
    f: &Expression,
    r: &Request,
    inverse: bool,
    derivative: bool,
    alive: &impl Fn() -> bool,
) -> Vec<Option<Point>> {
    let (low, high, pixels) = if inverse {
        (r.lower.y, r.upper.y, r.pixels[1])
    } else {
        (r.lower.x, r.upper.x, r.pixels[0])
    };
    let count = (pixels / 1.5).ceil().max(2.0) as usize;
    let mut points = vec![];
    let mut previous = None;
    for i in 0..=count {
        if !alive() {
            break;
        }
        let t = low + (high - low) * i as f64 / count as f64;
        if !t.is_finite() {
            return vec![];
        }
        if let Some(last) = previous {
            let screen = if inverse {
                f.screen_box([0.0, 0.0], [last, t])
            } else {
                f.screen_box([last, t], [0.0, 0.0])
            };
            if screen.is_err() {
                points.push(None);
            }
        }
        let value = if derivative {
            let h = f64::EPSILON.cbrt() * t.abs().max(1.0);
            if f.screen_box([t - h, t + h], [0.0, 0.0]).is_err() {
                None
            } else {
                f.evaluate_at(t + h, 0.0)
                    .ok()
                    .zip(f.evaluate_at(t - h, 0.0).ok())
                    .map(|(a, b)| (a - b) / (2.0 * h))
            }
        } else if inverse {
            f.evaluate_at(0.0, t).ok()
        } else {
            f.evaluate_at(t, 0.0).ok()
        };
        points.push(value.and_then(|v| {
            if inverse {
                Point::new(v, t).ok()
            } else {
                Point::new(t, v).ok()
            }
        }));
        previous = Some(t);
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math_editor::formula;
    fn request(sources: &[&str]) -> Request {
        Request {
            sources: sources.iter().map(|s| Ok((*s).into())).collect(),
            calculate: vec![false; sources.len()],
            lower: Point { x: -2.0, y: -2.0 },
            upper: Point { x: 2.0, y: 2.0 },
            pixels: [100.0, 100.0],
            spatial: false,
        }
    }
    #[test]
    fn dependency_order_duplicates_cycles_and_local_errors() {
        let (plot, _) = build_plot(&request(&["y=b*x", "b=c+1", "c=2", "x+", "3"]), &|| true);
        assert_eq!(plot.rows[1].scalar, Some(3.0));
        assert_eq!(plot.rows[2].scalar, Some(2.0));
        assert!(
            plot.rows[0]
                .points
                .iter()
                .flatten()
                .all(|p| (p.y - 3.0 * p.x).abs() < 1e-12)
        );
        assert!(plot.rows[3].diagnostic.is_some());
        assert_eq!(plot.rows[4].scalar, Some(3.0));
        let (plot, _) = build_plot(&request(&["b=c", "c=b", "a=1", "a=2", "y=a*x"]), &|| true);
        assert!(plot.rows.iter().all(|r| r.diagnostic.is_some()));
    }
    #[test]
    fn numerical_derivative_and_undefined_curve_are_explicit() {
        let (plot, _) = build_plot(&request(&["derivative(x^2)", "sqrt(-x^2-1)"]), &|| true);
        assert!(
            plot.rows[0]
                .points
                .iter()
                .flatten()
                .all(|p| (p.y - 2.0 * p.x).abs() < 1e-8)
        );
        assert!(plot.rows[1].diagnostic.is_some());
    }
    #[test]
    fn obsolete_worker_cannot_publish_over_latest_request() {
        let mut worker = Worker::new().unwrap();
        let mut heavy = request(&["x^2+y^2=3"]);
        heavy.pixels = [2000.0, 2000.0];
        worker.submit(heavy);
        worker.submit(request(&["42"]));
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(plot) = worker.poll() {
                assert_eq!(plot.rows[0].scalar, Some(42.0));
                break;
            }
            assert!(std::time::Instant::now() < until);
            std::thread::yield_now();
        }
        worker.cancel();
        assert!(worker.poll().is_none());
    }
    #[test]
    fn structured_integral_difference_waits_for_calculate_then_graphs_scalar() {
        let formula = formula::Formula::from_text("y=integral(0,1,x^2)-integral(0,1,x)").unwrap();
        assert!(formula.is_integral());
        let mut input = request(&[&formula.source().unwrap(), "y=x"]);
        let (plot, jobs) = build_plot(&input, &|| true);
        assert!(jobs.is_empty());
        assert!(plot.rows[0].diagnostic.is_none());
        assert!(plot.rows[0].scalar.is_none() && plot.rows[0].segments.is_empty());
        let mut worker = Worker::new().unwrap();
        input.calculate[0] = true;
        worker.submit(input);
        let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(plot) = worker.poll()
                && !plot.rows[0].pending
            {
                let row = &plot.rows[0];
                assert!(row.diagnostic.is_none(), "{:?}", row.diagnostic);
                assert!((row.scalar.unwrap() + 1.0 / 6.0).abs() < 1e-8);
                assert_eq!(row.segments.len(), 1);
                assert_eq!(row.segments[0][0].y, row.scalar.unwrap());
                assert!(row.integral.is_none() && row.points.is_empty());
                assert!(!plot.rows[1].points.is_empty());
                break;
            }
            assert!(std::time::Instant::now() < until);
            std::thread::yield_now();
        }
    }
}

/// One mathematical request's completed scalars, independent of camera sampling.
/// This is not a history cache: source edits replace it; activation changes
/// invalidate only their own row, since these rows cannot define other values.
#[derive(Default)]
pub(super) struct CalculationCache {
    sources: Vec<Result<String, String>>,
    calculate: Vec<bool>,
    values: Vec<Option<(f64, Option<Integral>)>>,
}
impl CalculationCache {
    pub fn prepare(&mut self, request: &Request) {
        if self.sources != request.sources {
            self.sources = request.sources.clone();
            self.values = vec![None; request.sources.len()];
        } else {
            for (index, value) in self.values.iter_mut().enumerate() {
                if self.calculate.get(index) != request.calculate.get(index) {
                    *value = None;
                }
            }
        }
        self.calculate = request.calculate.clone();
    }
    pub fn get(&self, index: usize) -> Option<(f64, Option<Integral>)> {
        self.values.get(index).copied().flatten()
    }
    pub fn record(&mut self, index: usize, value: (f64, Option<Integral>)) {
        if let Some(slot) = self.values.get_mut(index) {
            *slot = Some(value);
        }
    }
}

pub(super) enum Calculation {
    Integral(Box<IntegrationJob>),
    Scalar(Box<IntegralExpressionJob>),
}
impl Calculation {
    pub fn advance(&mut self, panels: usize) -> Result<Option<(f64, Option<Integral>)>, MathError> {
        match self {
            Self::Integral(job) => Ok(job
                .advance(panels)?
                .map(|result| (result.value, Some(result)))),
            Self::Scalar(job) => Ok(job.advance(panels)?.map(|value| (value, None))),
        }
    }
}
