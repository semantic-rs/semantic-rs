#![feature(try_trait, external_doc)]
#![deny(missing_docs)]
#![doc(include = "../README.md")]

#[macro_use]
extern crate strum_macros;

mod builtin_plugins;
mod config;
mod kernel;
mod plugin;
mod utils;

use crate::config::Config;
use crate::kernel::Kernel;
use kernel::KernelError;
use std::env;

fn main() {
    if let Err(err) = run() {
        eprintln!("!! Error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), failure::Error> {
    init_logger();
    dotenv::dotenv().ok();

    log::info!("semantic.rs 🚀");

    let clap_args = clap::App::new("semantic-rs")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(
            clap::Arg::with_name("dry")
                .long("dry")
                .help("Execute semantic-rs in dry-run more (no writes or publishes"),
        )
        .get_matches();

    let is_dry_run = clap_args.is_present("dry");

    let config = Config::from_toml("./releaserc.toml", is_dry_run)?;
    let kernel = Kernel::builder(config).build()?;

    if let Err(err) = kernel.run() {
        macro_rules! log_error_and_die {
            ($err:expr) => {{
                log::error!("{}", $err);
                std::process::exit(1);
            }};
        }

        match err.downcast::<KernelError>() {
            Ok(kernel_error) => match kernel_error {
                KernelError::EarlyExit => (),
                _ => log_error_and_die!(kernel_error),
            },
            Err(other_error) => {
                log_error_and_die!(other_error);
            }
        }
    }

    Ok(())
}

fn init_logger() {
    use std::io::Write;

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    env_logger::Builder::from_default_env()
        .format(|fmt, record| match record.level() {
            log::Level::Info => writeln!(fmt, "{}", record.args()),
            log::Level::Warn => writeln!(fmt, ">> {}", record.args()),
            log::Level::Error => writeln!(fmt, "!! {}", record.args()),
            log::Level::Debug => writeln!(fmt, "DD {}", record.args()),
            log::Level::Trace => writeln!(fmt, "TT {}", record.args()),
        })
        .init();
}
