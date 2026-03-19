# limit-tldr

**Code Analysis That Actually Fits In Context**

The Core Insight: LLMs can't read your entire codebase. So we extract the structure, trace the dependencies, and give them exactly what they need—at **95% fewer tokens** than raw code.

Stop burning context windows. Start shipping features.

## Features

### 5-Layer Architecture

- **Layer 1 (AST)**: Structure - "What functions exist?"
- **Layer 2 (Call Graph)**: Dependencies - "Who calls what?"
- **Layer 3 (CFG)**: Control Flow - "How complex is this?"
- **Layer 4 (DFG)**: Data Flow - "Where does this value come from?"
- **Layer 5 (PDG)**: Program Dependence - "What affects this line?"

### Multi-Language Support

Same API for 16 languages: Python, TypeScript, JavaScript, Go, Rust, Java, C, C++, Ruby, PHP, C#, Kotlin, Scala, Swift, Lua, Elixir

### Daemon Mode

300x faster than CLI spawns with in-memory indexes.

### Semantic Search

Find code by behavior, not just syntax (optional feature).

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
limit-tldr = { version = "0.1.0" }
```

Or with semantic search:

```toml
[dependencies]
limit-tldr = { version = "0.1.0", features = ["semantic"] }
```

## CLI Usage

```bash
# Install
cargo install limit-tldr

# Index your project (builds all analysis layers)
tldr warm /path/to/project

# Start daemon for fast queries
tldr daemon start --project /path/to/project

# Get LLM-ready context for a function
tldr context process_data --project /path/to/project --depth 2

# Find who calls a function (impact analysis)
tldr impact process_data /path/to/project

# Get control flow graph
tldr cfg src/auth.py login

# Get data flow graph
tldr dfg src/auth.py login

# Get program slice for debugging
tldr slice src/auth.py login 42

# Find dead code
tldr dead /path/to/project --entry main,cli

# Detect architecture layers
tldr arch /path/to/project

# Semantic search
tldr semantic "validate JWT tokens" /path/to/project
```

## API Usage

```rust
use limit_tldr::{TLDR, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create TLDR instance
    let config = Config::default();
    let mut tldr = TLDR::new("./my-project", config).await?;
    
    // Warm up indexes
    tldr.warm().await?;
    
    // Get LLM-ready context
    let context = tldr.get_context("process_data", 2).await?;
    println!("{}", context);
    
    // Impact analysis
    let callers = tldr.get_impact("hash_password")?;
    for caller in callers {
        println!("{} called by {} at {}:{}", 
            "hash_password", caller.function, caller.file.display(), caller.line);
    }
    
    // Find dead code
    let dead = tldr.find_dead_code(&["main", "cli"])?;
    println!("Found {} unreachable functions", dead.len());
    
    Ok(())
}
```

## Architecture

### Token Savings

| Scenario | Raw Tokens | TLDR Tokens | Savings |
|----------|------------|-------------|---------|
| Single file analysis | 9,114 | 7,074 | 22% |
| Function + callees | 21,271 | 175 | **99%** |
| Codebase overview (26 files) | 103,901 | 11,664 | 89% |
| Deep call chain (7 files) | 53,474 | 2,667 | 95% |

### Performance

| Method | Query Time | What Happens |
|--------|------------|--------------|
| CLI spawn | ~30 seconds | Parse entire codebase, build indexes, analyze, return result, exit |
| Daemon query | ~100ms | Read from in-memory index, return result |
| **Speedup** | **300x** | Measured on a 50-file Python project |

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Check all features
cargo check --all-features
```

## License

Apache 2.0
