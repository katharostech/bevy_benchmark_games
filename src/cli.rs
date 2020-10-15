use std::{fs::OpenOptions, path::PathBuf};

use argh::FromArgs;
use criterion_stats::{Distribution, Tails};
use eyre::WrapErr;
use human_format::{Formatter, Scales};
use plotters::{coord::Shift, prelude::*};
use thiserror::Error;
use tracing as trc;

use crate::metrics::Metrics;

mod cmd;

/// The list of benchmarks
static BENCHMARKS: &'static [&'static str] = &["asteroids"];

/// The number of columns of graphs we will have for each benchmark
///
/// Currently we will have three graphs per benchmark.
static BENCHMARK_GRAPH_COLS: usize = 3;

/// The height in pixels to allocate for each benchmark graph
static BENCHMARK_GRAPH_HEIGHT: usize = 400;

/// The width in pixels to allocate for each benchmark graph
static BENCHMARK_GRAPH_WIDTH: usize = 600;

/// An error that ndicates that the program should exit with the given code
#[derive(Error, Debug)]
#[error("Program exited {0}")]
struct Exit(i32);

#[derive(FromArgs)]
/// Reach new heights.
struct Args {
    /// whether or not to jump
    #[argh(switch, short = 'H')]
    no_headless: bool,
}
/// Start program logic
fn start() -> eyre::Result<()> {
    let args: Args = trc::debug_span!("Parsing commandline args").in_scope(|| argh::from_env());

    let document_width = BENCHMARK_GRAPH_WIDTH * BENCHMARK_GRAPH_COLS;
    let document_height = BENCHMARK_GRAPH_HEIGHT * BENCHMARKS.len();
    let root_drawing_area = SVGBackend::new(
        "./target/report.svg",
        (document_width as u32, document_height as u32),
    )
    .into_drawing_area();

    root_drawing_area.fill(&WHITE)?;

    let areas = root_drawing_area.split_evenly((BENCHMARKS.len(), 1));

    trc::info!("Starting benchmarks");

    for (&benchmark, drawing_area) in BENCHMARKS.iter().zip(areas) {
        trc::info_span!("Benchmarking {}", benchmark).in_scope(|| -> eyre::Result<()> {
            // Build the benchmark
            cmd::build_example(benchmark, !args.no_headless)?;
            let output = cmd::run_example(benchmark)?;

            // Parse the metrics
            let metrics: Metrics =
                serde_json::from_str(&output).wrap_err("Could not parse metrics")?;
            let iterations = metrics.iterations.clone();

            // Check for previous run metrics
            let previous_metrics_path =
                PathBuf::from(format!("./target/{}_metrics.json", benchmark));
            let previous_metrics: Option<Metrics> = if previous_metrics_path.exists() {
                let file = OpenOptions::new().read(true).open(&previous_metrics_path)?;
                serde_json::from_reader(file)?
            } else {
                None
            };
            let previous_iterations = previous_metrics.map(|x| x.iterations);

            // Write our current metrics out to the previous metrics file for next run
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(previous_metrics_path)?;
            serde_json::to_writer(file, &metrics)?;

            // Create a title area for the chart
            let (title_area, graph_area) = drawing_area.split_vertically(8.percent_height());

            // Draw the title
            title_area.draw_text(
                &format!("\"{}\" Benchmark", benchmark),
                &TextStyle::from(
                    ("Sans", title_area.relative_to_height(1.))
                        .into_font()
                        .color(&BLACK),
                ),
                (10, 5),
            )?;

            // Split the graph area into parts for each of our different graphs
            let graph_areas = graph_area.split_evenly((1, BENCHMARK_GRAPH_COLS));
            let frame_time_area = &graph_areas[0];
            let cpu_cycles_area = &graph_areas[1];
            let cpu_instructions_area = &graph_areas[2];

            // Print the frame averages graph
            let mut frame_avgs: Vec<_> = iterations.iter().map(|x| x.avg_frame_time_us).collect();
            frame_avgs
                .as_mut_slice()
                .sort_unstable_by(|x, y| x.partial_cmp(&y).unwrap());
            let previous_frame_avgs = previous_iterations.clone().map(|x| {
                let mut vec: Vec<_> = x.iter().map(|y| y.avg_frame_time_us).collect();
                vec.as_mut_slice()
                    .sort_unstable_by(|x, y| x.partial_cmp(&y).unwrap());
                vec
            });

            let frame_formatter = &|x: &f64| format!("{:.2} µs", x);

            graph_series(
                "Frame Time Avg.",
                "Frame Time",
                frame_avgs,
                previous_frame_avgs,
                &frame_time_area,
                Some(frame_formatter),
            )?;

            // Print the CPU cycles graph
            let mut formatter = Formatter::new();
            formatter.with_scales(Scales::SI());
            let cpu_formatter = &|x: &f64| formatter.format(*x);

            let mut cpu_cycles: Vec<_> = iterations.iter().map(|x| x.cpu_cycles as f64).collect();
            cpu_cycles
                .as_mut_slice()
                .sort_unstable_by(|x, y| x.partial_cmp(&y).unwrap());
            let previous_cpu_cycles = previous_iterations.clone().map(|x| {
                let mut vec: Vec<_> = x.iter().map(|y| y.cpu_cycles as f64).collect();
                vec.as_mut_slice()
                    .sort_unstable_by(|x, y| x.partial_cmp(&y).unwrap());
                vec
            });

            graph_series(
                "CPU Cycles",
                "Cycles",
                cpu_cycles,
                previous_cpu_cycles,
                &cpu_cycles_area,
                Some(&cpu_formatter),
            )?;

            // Print the CPU instructions graph
            let mut cpu_instructions: Vec<_> = iterations
                .iter()
                .map(|x| x.cpu_instructions as f64)
                .collect();
            cpu_instructions
                .as_mut_slice()
                .sort_unstable_by(|x, y| x.partial_cmp(&y).unwrap());
            let previous_cpu_instructions = previous_iterations.clone().map(|x| {
                let mut vec: Vec<_> = x.iter().map(|y| y.cpu_instructions as f64).collect();
                vec.as_mut_slice()
                    .sort_unstable_by(|x, y| x.partial_cmp(&y).unwrap());
                vec
            });

            graph_series(
                "CPU instructions",
                "Instructions",
                cpu_instructions,
                previous_cpu_instructions,
                &cpu_instructions_area,
                Some(&cpu_formatter),
            )?;

            Ok(())
        })?;
    }

    trc::info!("Benchmark report is in `target/report.svg` and can be opened in a web browser");

    Ok(())
}

/// Run the ClI
pub fn run() {
    // Install tracing for logs
    install_tracing();
    // Install color error printing
    color_eyre::install().expect("Could not install error handler");

    // Start the application and capture errors
    match start() {
        // Do nothing for happy runs!
        Ok(()) => (),
        // Hnadle errors
        Err(report) => {
            // If the error is an exit code
            if let Some(e) = report.downcast_ref::<Exit>() {
                let code = e.0;

                // If the code is zero, exit cleanly
                if code == 0 {
                    std::process::exit(0);

                // If the code is non-zero print the error and then exit with that code
                } else {
                    trc::error!("{:?}", report);
                    std::process::exit(e.0);
                }
            // If the error is any other kind of error print it and exit 1
            } else {
                trc::error!("{:?}", report);
                std::process::exit(1);
            }
        }
    }
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, fmt::format::FmtSpan, EnvFilter};

    // Build the tracing layers
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_span_events(FmtSpan::FULL);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    // Add all of the layers to the subscriber and initialize it
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

fn graph_series<'a, T: DrawingBackend + 'static>(
    title: &str,
    x_desc: &str,
    data: Vec<f64>,
    previous_data: Option<Vec<f64>>,
    drawing_area: &DrawingArea<T, Shift>,
    x_label_formatter: Option<&dyn Fn(&f64) -> String>,
) -> eyre::Result<()> {
    let dist = Distribution::from(data.into_boxed_slice());
    let prev_dist = previous_data.map(|x| Distribution::from(x.into_boxed_slice()));

    let x_min = if let Some(prev) = &prev_dist {
        if prev.min() < dist.min() {
            prev.min()
        } else {
            dist.min()
        }
    } else {
        dist.min()
    };
    let x_max = if let Some(prev) = &prev_dist {
        if prev.max() > dist.max() {
            prev.max()
        } else {
            dist.max()
        }
    } else {
        dist.max()
    };

    let mean = dist.mean();

    let mut chart = ChartBuilder::on(drawing_area)
        .caption(title, ("Sans", 20))
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .margin(5)
        .build_cartesian_2d(x_min..x_max, 0f64..1f64)?;

    chart
        .configure_mesh()
        .axis_desc_style(("Sans", 15))
        .y_desc("Probability")
        .x_desc(x_desc)
        .light_line_style(&TRANSPARENT)
        .x_label_formatter(x_label_formatter.unwrap_or(&|x| format!("{}", x)))
        .draw()?;

    let mean_label_x_offset = (dist.max() - dist.min()) / 20.;

    let mut draw_for_dist =
        |dist: &Distribution<f64>, color: &RGBColor, mean, mean_label_pos| -> eyre::Result<()> {
            // Draw the shaded probability indicator
            chart.draw_series(AreaSeries::new(
                dist.to_vec()
                    .iter()
                    .map(|x| (*x, dist.p_value(*x, &Tails::Two))),
                0.,
                &color.mix(0.3),
            ))?;

            // Draw the mean line
            chart.draw_series(LineSeries::new(
                [(mean, 0f64), (mean, dist.p_value(mean, &Tails::Two))]
                    .iter()
                    .map(|x| *x),
                color,
            ))?;

            // Draw mean label
            let drawing_area = chart.plotting_area();
            drawing_area.draw(&Text::new(
                format!(
                    "Avg. {}",
                    if let Some(formatter) = x_label_formatter {
                        formatter(&mean)
                    } else {
                        format!("{}", mean)
                    }
                ),
                (mean + mean_label_x_offset, mean_label_pos),
                TextStyle::from(("Sans", 12).into_font()).color(color),
            ))?;

            Ok(())
        };

    if let Some(prev) = &prev_dist {
        draw_for_dist(&prev, &RED, prev.mean(), 0.5 /* mean label pos */)?;
    }
    draw_for_dist(&dist, &BLUE, mean, 0.7 /* mean label pos */)?;

    // Draw the difference percentage
    if let Some(prev) = &prev_dist {
        let drawing_area = chart.plotting_area();

        let percentage_diff = (dist.mean() - prev.mean()) / prev.mean() * 100.;

        let color = if percentage_diff.abs() < 2. {
            &BLACK
        } else if percentage_diff > 0. {
            &RED
        } else {
            // Dark green
            &RGBColor(0, 170, 0)
        };

        drawing_area.draw(&Text::new(
            format!("{:+.2}%", percentage_diff),
            (
                dist.mean() + (prev.mean() - dist.mean()) + mean_label_x_offset,
                0.6,
            ),
            TextStyle::from(("Sans", 20).into_font()).color(color),
        ))?;
    }

    Ok(())
}
