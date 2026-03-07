use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| {
            // Default: show only project crates at debug level, suppress noisy deps
            EnvFilter::try_new("debug,limit_llm=trace,limit_agent=debug,limit_cli=debug,reqwest=warn,hyper=warn,tower=warn,h2=warn,rustyline=warn")
        })
        .unwrap();

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true))
        .init();
}
