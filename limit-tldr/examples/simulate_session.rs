//! Simulate the exact tldr_analyze calls from the "explique o modulo semantic" session
//! to visualize what the LLM receives as tool results.
//!
//! Run with: cargo run --example simulate_session

use limit_tldr::{Config, Language, TLDR};
use serde_json::{json, Value};

fn print_separator(title: &str) {
    println!("\n{}", "=".repeat(80));
    println!("  {}", title);
    println!("{}\n", "=".repeat(80));
}

fn print_json(value: &Value, label: &str) {
    let json_str = serde_json::to_string_pretty(value).unwrap();
    println!(
        "┌─ {} ({} chars, {} bytes)",
        label,
        json_str.chars().count(),
        json_str.len()
    );
    println!("│");
    for line in json_str.lines() {
        println!("│  {}", line);
    }
    println!("└──────────────────────────────────────────");
}

fn estimate_tokens(text: &str) -> usize {
    // Rough estimate: 1 token ≈ 4 chars for English/code
    text.len() / 4
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_path = std::env::current_dir()?.parent().unwrap().to_path_buf();
    println!("Project: {:?}", project_path);

    let config = Config {
        language: Language::Auto,
        max_depth: 3,
        cache_dir: None,
    };

    println!("Creating TLDR instance...");
    let mut tldr = TLDR::new(&project_path, config).await?;
    println!("Warming indexes...");
    tldr.warm().await?;

    let mut total_tool_result_chars = 0usize;
    let mut total_tool_result_tokens = 0usize;
    let mut call_count = 0usize;

    // ================================================================
    // ITERATION 1: Search("semantic") → 1,758 chars
    // ================================================================
    print_separator("ITER 1: Search(query=\"semantic\")");
    let results = tldr.semantic_search("semantic", 10).await?;
    let value = json!({
        "type": "search",
        "query": "semantic",
        "results": results.iter().map(|r| json!({
            "function": r.function,
            "file": r.file.display().to_string(),
            "line": r.line,
            "score": r.score
        })).collect::<Vec<_>>()
    });
    let s = serde_json::to_string(&value)?;
    total_tool_result_chars += s.len();
    total_tool_result_tokens += estimate_tokens(&s);
    call_count += 1;
    print_json(&value, &format!("iter1_search ({})", s.len()));

    // ================================================================
    // ITERATION 2: Source("new") + Source("text_search") + Source("search")
    // ================================================================
    print_separator("ITER 2: 3 parallel Source calls");

    // Source("new") → 901 chars
    let func = tldr.find_function("new").await?.ok_or("not found")?;
    let file_path = project_path.join(&func.file);
    let source = tokio::fs::read_to_string(&file_path).await?;
    let lines: Vec<&str> = source.lines().collect();
    let start = func.line.saturating_sub(1);
    let end = func.end_line.min(lines.len());
    let fn_source = lines[start..end].join("\n");
    let value = json!({
        "type": "source",
        "function": "new",
        "file": func.file.display().to_string(),
        "line": func.line,
        "end_line": func.end_line,
        "source": fn_source
    });
    let s = serde_json::to_string(&value)?;
    total_tool_result_chars += s.len();
    total_tool_result_tokens += estimate_tokens(&s);
    call_count += 1;
    print_json(&value, &format!("iter2_source_new ({})", s.len()));

    // Source("text_search") → 1,761 chars
    let func = tldr
        .find_function("text_search")
        .await?
        .ok_or("not found")?;
    let file_path = project_path.join(&func.file);
    let source = tokio::fs::read_to_string(&file_path).await?;
    let lines: Vec<&str> = source.lines().collect();
    let start = func.line.saturating_sub(1);
    let end = func.end_line.min(lines.len());
    let fn_source = lines[start..end].join("\n");
    let value = json!({
        "type": "source",
        "function": "text_search",
        "file": func.file.display().to_string(),
        "line": func.line,
        "end_line": func.end_line,
        "source": fn_source
    });
    let s = serde_json::to_string(&value)?;
    total_tool_result_chars += s.len();
    total_tool_result_tokens += estimate_tokens(&s);
    call_count += 1;
    print_json(&value, &format!("iter2_source_text_search ({})", s.len()));

    // Source("search") → 2,030 chars
    let func = tldr.find_function("search").await?.ok_or("not found")?;
    let file_path = project_path.join(&func.file);
    let source = tokio::fs::read_to_string(&file_path).await?;
    let lines: Vec<&str> = source.lines().collect();
    let start = func.line.saturating_sub(1);
    let end = func.end_line.min(lines.len());
    let fn_source = lines[start..end].join("\n");
    let value = json!({
        "type": "source",
        "function": "search",
        "file": func.file.display().to_string(),
        "line": func.line,
        "end_line": func.end_line,
        "source": fn_source
    });
    let s = serde_json::to_string(&value)?;
    total_tool_result_chars += s.len();
    total_tool_result_tokens += estimate_tokens(&s);
    call_count += 1;
    print_json(&value, &format!("iter2_source_search ({})", s.len()));

    // ================================================================
    // ITERATION 3: Source("SemanticIndex")
    // ================================================================
    print_separator("ITER 3: Source(function=\"SemanticIndex\")");
    match tldr.find_function("SemanticIndex").await? {
        Some(func) => {
            let file_path = project_path.join(&func.file);
            let source = tokio::fs::read_to_string(&file_path).await?;
            let lines: Vec<&str> = source.lines().collect();
            let start = func.line.saturating_sub(1);
            let end = func.end_line.min(lines.len());
            let fn_source = lines[start..end].join("\n");
            let value = json!({
                "type": "source",
                "function": "SemanticIndex",
                "file": func.file.display().to_string(),
                "line": func.line,
                "end_line": func.end_line,
                "source": fn_source
            });
            let s = serde_json::to_string(&value)?;
            total_tool_result_chars += s.len();
            total_tool_result_tokens += estimate_tokens(&s);
            call_count += 1;
            print_json(&value, &format!("iter3_source_SemanticIndex ({})", s.len()));
        }
        None => println!("  Function 'SemanticIndex' not found (it's a struct, not a function)"),
    }

    // ================================================================
    // ITERATION 4: Search("struct.*Semantic")
    // ================================================================
    print_separator("ITER 4: Search(query=\"struct.*Semantic\")");
    let results = tldr.semantic_search("struct.*Semantic", 10).await?;
    let value = json!({
        "type": "search",
        "query": "struct.*Semantic",
        "results": results.iter().map(|r| json!({
            "function": r.function,
            "file": r.file.display().to_string(),
            "line": r.line,
            "score": r.score
        })).collect::<Vec<_>>()
    });
    let s = serde_json::to_string(&value)?;
    total_tool_result_chars += s.len();
    total_tool_result_tokens += estimate_tokens(&s);
    call_count += 1;
    print_json(
        &value,
        &format!("iter4_search_struct_Semantic ({})", s.len()),
    );

    // ================================================================
    // ITERATION 5: Source("warm")
    // ================================================================
    print_separator("ITER 5: Source(function=\"warm\")");
    let func = tldr.find_function("warm").await?.ok_or("not found")?;
    let file_path = project_path.join(&func.file);
    let source = tokio::fs::read_to_string(&file_path).await?;
    let lines: Vec<&str> = source.lines().collect();
    let start = func.line.saturating_sub(1);
    let end = func.end_line.min(lines.len());
    let fn_source = lines[start..end].join("\n");
    let value = json!({
        "type": "source",
        "function": "warm",
        "file": func.file.display().to_string(),
        "line": func.line,
        "end_line": func.end_line,
        "source": fn_source
    });
    let s = serde_json::to_string(&value)?;
    total_tool_result_chars += s.len();
    total_tool_result_tokens += estimate_tokens(&s);
    call_count += 1;
    print_json(&value, &format!("iter5_source_warm ({})", s.len()));

    // ================================================================
    // ITERATION 6: ??? (no tldr_analyze in log, possibly just processing)
    // ================================================================
    print_separator("ITER 6: No tldr call (LLM processing only)");

    // ================================================================
    // SUMMARY
    // ================================================================
    print_separator("TOKEN COST ANALYSIS");

    println!("Tool calls made: {}", call_count);
    println!("Total tool result chars: {}", total_tool_result_chars);
    println!("Est. tool result tokens: ~{}", total_tool_result_tokens);
    println!();

    println!("Per-iteration input tokens (from log):");
    let iter_tokens = [
        ("Iter 1 (1 Search)", 3976),
        ("Iter 2 (3 Source)", 4485),
        ("Iter 3 (1 Source)", 5644),
        ("Iter 4 (1 Search)", 5682),
        ("Iter 5 (1 Source)", 5728),
        ("Iter 6 (0 calls)", 6210),
        ("Iter 7 (final)", 7167),
    ];

    let mut cumulative = 0;
    for (label, tokens) in &iter_tokens {
        cumulative += tokens;
        println!(
            "  {:20} → {:>5} in  (cumulative: {:>5})",
            label, tokens, cumulative
        );
    }
    println!();
    println!("  TOTAL INPUT TOKENS:      {}", cumulative);
    println!("  TOTAL OUTPUT TOKENS:     937");
    println!();

    // Analyze token breakdown
    println!("TOKEN BREAKDOWN (estimated):");
    println!("  System prompt:           ~600 tokens");
    println!("  19 tool definitions:     ~1,064 tokens (sent EVERY iteration)");
    println!("  Tool def overhead (x7):  ~7,448 tokens");
    println!("  User message:            ~20 tokens (sent once)");
    println!(
        "  Tool results:            ~{} tokens",
        total_tool_result_tokens
    );
    println!("  Assistant msgs:          ~200 tokens");
    println!("  LLM thinking/output:     ~937 tokens");
    println!();

    println!("KEY FINDING:");
    println!("  Tool definitions alone cost 7,448 tokens (19.1% of total)");
    println!(
        "  Tool results cost ~{} tokens (est)",
        total_tool_result_tokens
    );
    println!("  Redundant calls: Source(\"new\") returned SemanticIndex::new()");
    println!(
        "  Source(\"search\") returned SemanticIndex::search() - same file as Source(\"new\")"
    );
    println!("  Source(\"warm\") returned SemanticIndex::warm() - same file again!");
    println!("  Search(\"struct.*Semantic\") returned 0 results (regex not supported)");

    Ok(())
}
