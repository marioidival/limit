use limit_llm::{ContentPart, Message, MessageContent, Role};

fn main() {
    // Test different formats to see which one z.ai accepts

    println!("=== Format 1: OpenAI-style image_url ===");
    let msg1 = Message {
        role: Role::User,
        content: Some(MessageContent::parts(vec![
            ContentPart::text("What's this?"),
            ContentPart::image_base64("image/png", "abc123"),
        ])),
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
    };
    println!("{}\n", serde_json::to_string_pretty(&msg1).unwrap());

    println!("=== Format 2: Simple text (no image) ===");
    let msg2 = Message {
        role: Role::User,
        content: Some(MessageContent::text("Hello")),
        tool_calls: None,
        tool_call_id: None,
        cache_control: None,
    };
    println!("{}\n", serde_json::to_string_pretty(&msg2).unwrap());
}
