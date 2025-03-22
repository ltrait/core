pub use async_trait;
pub use color_eyre;
pub use tokio_stream;

#[cfg(feature = "log")]
pub use tracing::Level;

pub mod action;
pub mod filter;
pub mod generator;
pub mod launcher;
pub mod sorter;
pub mod source;
pub mod ui;

pub use crate::action::Action;
pub use crate::filter::Filter;
pub use crate::generator::Generator;
pub use crate::launcher::Launcher;
pub use crate::sorter::Sorter;
pub use crate::source::Source;
pub use crate::ui::UI;

use color_eyre::eyre::{OptionExt, Result};

#[cfg(feature = "log")]
fn init_subscriber_with_level(level: Level) -> Result<()> {
    use tracing_subscriber::fmt::format::FmtSpan;

    fn db_dir() -> Option<std::path::PathBuf> {
        let path = dirs::cache_dir().map(|p| p.join("ltrait/log"));

        if let Some(parent) = path.as_ref().and_then(|p| p.parent()) {
            std::fs::create_dir_all(parent).ok()?;
        }

        path
    }

    let file_appender = tracing_appender::rolling::hourly(
        db_dir().ok_or_eyre("failed to get log dir")?,
        "core.log",
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_max_level(level) // set recorded log level
        .with_span_events(FmtSpan::ACTIVE) // enable record span timing
        .init();

    Ok(())
}

/// Install color_eyre and setup tracing(with tracing-log)
/// ```
/// use ltrait::{Level, setup};
///
/// let _ = setup(Level::TRACE);
/// ```
pub fn setup(log_level: Level) -> Result<()> {
    color_eyre::install()?;

    #[cfg(feature = "log")]
    init_subscriber_with_level(log_level)
}
