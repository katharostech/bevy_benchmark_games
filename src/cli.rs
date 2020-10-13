use argh::FromArgs;
use eyre::WrapErr;
use thiserror::Error;
use tracing as trc;

use crate::metrics::Metrics;

mod cmd;

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

    trc::info!("Starting benchmarks");

    trc::info_span!("Benchmarking asteroids").in_scope(|| -> eyre::Result<()> {
        cmd::build_example("asteroids", !args.no_headless)?;
        let output = cmd::run_example("asteroids")?;

        let metrics: Metrics = serde_json::from_str(&output).wrap_err("Could not parse metrics")?;

        dbg!(metrics);

        Ok(())
    })?;

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
