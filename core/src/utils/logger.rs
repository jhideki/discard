use tracing_subscriber::EnvFilter;
pub fn init_tracing() {
    let filter = EnvFilter::new("discard=info");
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .with_line_number(true)
        .with_file(true)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

pub fn init_tracing_no_filt() {
    let subscriber = tracing_subscriber::fmt().compact().with_file(true).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

pub fn init_signal_file_trace() {
    let filter = EnvFilter::new("discard::core::signal=debug,iroh::net::endpoint=info");
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .with_line_number(true)
        .with_file(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
