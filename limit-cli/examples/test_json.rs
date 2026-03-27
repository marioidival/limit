use limit_llm::{ContentPart, MessageContent};

fn main() {
    let parts = vec![
        ContentPart::text("Hello"),
        ContentPart::image_base64("image/png", "abc123"),
    ];

    println!("=== Testing ContentPart serialization ===\n");

    for (i, part) in parts.iter().enumerate() {
        let json = serde_json::to_string_pretty(part).unwrap();
        println!("Part {}:\n{}\n", i, json);
    }

    println!("=== Full MessageContent ===\n");
    let content = MessageContent::parts(parts);
    let json = serde_json::to_string_pretty(&content).unwrap();
    println!("{}\n", json);
}
