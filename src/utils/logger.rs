use std::{env, io};

use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, EnvFilter};
pub fn init_tracing() {
    let filter = EnvFilter::new("discard=debug");
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .with_file(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

pub fn init_tracing_no_filt() {
    let subscriber = tracing_subscriber::fmt().compact().with_file(true).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

pub fn init_signal_file_trace() {
    let filter = EnvFilter::new("discard::core::signal=debug,tests=debug");
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .with_file(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
