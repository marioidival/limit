#!/usr/bin/env rust
//! TLDR CLI binary

use clap::Parser;
use limit_tldr::cli::Cli;
use limit_tldr::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    // Run CLI
    limit_tldr::cli::run(cli).await
}
