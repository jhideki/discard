use std::{env, io};

use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, EnvFilter};
pub fn init_tracing() {
    let filter = EnvFilter::new("discard=debug");
    let subscriber = tracing_subscriber::fmt().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
