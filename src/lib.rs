pub use async_trait;
pub use color_eyre;
pub use tokio_stream;

pub use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

pub mod action;
pub mod filter;
pub mod generator;
pub mod launcher;
pub mod sorter;
pub mod source;
pub mod ui;

pub use crate::action::Action;
pub use crate::filter::Filter;
pub use crate::launcher::Launcher;
pub use crate::sorter::Sorter;

use color_eyre::eyre::Result;

fn init_subscriber_with_level(level: Level) {
    tracing_subscriber::fmt()
        .with_max_level(level) // set recorded log level
        .with_span_events(FmtSpan::ACTIVE) // enable record span timing
        .init();
}

/// Install color_eyre and setup tracing(with tracing-log)
/// ```
/// use ltrait::{Level, setup};
///
/// let _ = setup(Level::TRACE);
/// ```
pub fn setup(log_level: Level) -> Result<()> {
    color_eyre::install()?;

    init_subscriber_with_level(log_level);

    Ok(())
}
