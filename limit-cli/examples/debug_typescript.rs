use limit_cli::syntax::SyntaxHighlighter;

fn main() {
    let highlighter = SyntaxHighlighter::new().unwrap();
    let syntax = highlighter.detect_language("typescript");
    println!("TypeScript syntax name: {}", syntax.name);
    println!("TypeScript syntax extensions: {:?}", syntax.file_extensions);
}
