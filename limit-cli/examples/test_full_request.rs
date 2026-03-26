use limit_llm::{ContentPart, Message, MessageContent, Role};

fn main() {
    let parts = vec![
        ContentPart::text("What's in this image?"),
        ContentPart::image_base64("image/png", "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="),
    ];
    
    let message = Message {
        role: Role::User,
        content: Some(MessageContent::parts(parts)),
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
    };
    
    println!("=== Full Message JSON ===\n");
    let json = serde_json::to_string_pretty(&message).unwrap();
    println!("{}\n", json);
    
    println!("=== Request Body ===\n");
    let request = serde_json::json!({
        "model": "gpt-4o",
        "messages": vec![message],
        "stream": true,
        "max_tokens": 4096
    });
    println!("{}\n", serde_json::to_string_pretty(&request).unwrap());
}
