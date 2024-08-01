use crate::utils::types::NodeId;
use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*};
pub fn init_tracing(level_filter: LevelFilter) {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(level_filter)
        .init();
}

//TODO: Setup other modules??
