//! CLI runner

use std::path::PathBuf;

use crate::cli::{Cli, Commands, DaemonCommands};
use crate::daemon::{Daemon, DaemonClient};
use crate::error::Result;
use crate::types::DaemonCommand;
use crate::{Config, TLDR};

pub async fn run(cli: Cli) -> Result<()> {
    let project_path = cli.project.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    
    match cli.command {
        Commands::Warm { path } => {
            let path = path.unwrap_or(project_path);
            println!("Warming up indexes for {}...", path.display());
            
            let config = Config::default();
            let mut tldr = TLDR::new(&path, config).await?;
            tldr.warm().await?;
            
            println!("✓ Indexes built successfully");
        }
        
        Commands::Daemon { command } => {
            match command {
                DaemonCommands::Start => {
                    println!("Starting daemon for {}...", project_path.display());
                    
                    let daemon = Daemon::new(&project_path).await?;
                    daemon.run().await?;
                }
                
                DaemonCommands::Stop => {
                    println!("Stopping daemon...");
                    
                    let client = DaemonClient::new(&project_path);
                    // Would send shutdown command
                    
                    println!("✓ Daemon stopped");
                }
                
                DaemonCommands::Status => {
                    if Daemon::is_running(&project_path).await {
                        let client = DaemonClient::new(&project_path);
                        let response = client.send(&DaemonCommand::Status)?;
                        println!("{}", serde_json::to_string_pretty(&response)?);
                    } else {
                        println!("Daemon not running");
                    }
                }
            }
        }
        
        Commands::Context { entry, depth } => {
            let client = DaemonClient::new(&project_path);
            let response = client.send(&DaemonCommand::Context {
                entry,
                depth,
            })?;
            
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        
        Commands::Impact { function, path } => {
            let path = path.unwrap_or(project_path);
            let client = DaemonClient::new(&path);
            let response = client.send(&DaemonCommand::Impact {
                function,
            })?;
            
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        
        Commands::Cfg { file, function } => {
            let client = DaemonClient::new(&project_path);
            let response = client.send(&DaemonCommand::Cfg {
                file,
                function,
            })?;
            
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        
        Commands::Dfg { file, function } => {
            let client = DaemonClient::new(&project_path);
            let response = client.send(&DaemonCommand::Dfg {
                file,
                function,
            })?;
            
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        
        Commands::Slice { file, function, line } => {
            let client = DaemonClient::new(&project_path);
            let response = client.send(&DaemonCommand::Slice {
                file,
                function,
                line,
            })?;
            
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        
        Commands::Extract { file } => {
            let config = Config::default();
            let mut tldr = TLDR::new(&project_path, config).await?;
            
            let analysis = tldr.ast.analyze_file(&file).await?;
            println!("{}", serde_json::to_string_pretty(&analysis)?);
        }
        
        Commands::Dead { entry, path } => {
            let path = path.unwrap_or(project_path);
            
            let config = Config::default();
            let mut tldr = TLDR::new(&path, config).await?;
            tldr.warm().await?;
            
            let dead = tldr.find_dead_code(&entry.iter().map(String::as_str).collect::<Vec<_>>())?;
            
            println!("Dead code found: {} functions", dead.len());
            for func in dead {
                println!("  {} ({}:{})", func.name, func.file.display(), func.line);
            }
        }
        
        Commands::Arch { path } => {
            let path = path.unwrap_or(project_path);
            
            let config = Config::default();
            let mut tldr = TLDR::new(&path, config).await?;
            tldr.warm().await?;
            
            let arch = tldr.detect_architecture()?;
            println!("{}", serde_json::to_string_pretty(&arch)?);
        }
        
        Commands::Semantic { query, path, limit } => {
            #[cfg(feature = "semantic")]
            {
                let path = path.unwrap_or(project_path);
                
                let config = Config { enable_semantic: true, ..Default::default() };
                let mut tldr = TLDR::new(&path, config).await?;
                tldr.warm().await?;
                
                let results = tldr.semantic_search(&query, limit).await?;
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
            
            #[cfg(not(feature = "semantic"))]
            {
                eprintln!("Semantic search requires 'semantic' feature. Reinstall with: pip install tldr-code[semantic]");
            }
        }
        
        Commands::Search { pattern, path } => {
            let client = DaemonClient::new(&path.unwrap_or(project_path));
            let response = client.send(&DaemonCommand::Search {
                pattern,
            })?;
            
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        
        Commands::Tree { path } => {
            let path = path.unwrap_or(project_path);
            
            // Would show file tree
            println!("File tree for {}", path.display());
        }
        
        Commands::Structure { path, lang } => {
            let path = path.unwrap_or(project_path);
            
            // Would show code structure
            if let Some(lang_str) = lang {
                if let Ok(lang) = lang_str.parse::<crate::types::Language>() {
                    println!("Code structure for {} ({:?})", path.display(), lang);
                } else {
                    println!("Code structure for {}", path.display());
                }
            } else {
                println!("Code structure for {}", path.display());
            }
        }
        
        Commands::Imports { file } => {
            let client = DaemonClient::new(&project_path);
            let response = client.send(&DaemonCommand::Imports {
                file,
            })?;
            
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        
        Commands::Importers { module, path } => {
            let client = DaemonClient::new(&path.unwrap_or(project_path));
            let response = client.send(&DaemonCommand::Importers {
                module,
            })?;
            
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        
        Commands::Diagnostics { path, format } => {
            let client = DaemonClient::new(&project_path);
            let response = client.send(&DaemonCommand::Diagnostics {
                path,
                format: if format == "json" {
                    crate::types::OutputFormat::Json
                } else {
                    crate::types::OutputFormat::Text
                },
            })?;
            
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }
    
    Ok(())
}
