use base64::{engine::general_purpose, Engine as _};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let _base64_img = general_purpose::STANDARD.encode([0x89, 0x50, 0x4E, 0x47]); // dummy

    // Test: Just URL string, no object wrapper
    let request = serde_json::json!({
        "model": "glm-5",
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "Test"
                    },
                    {
                        "type": "image_url",
                        "image_url": "https://example.com/image.png"
                    }
                ]
            }
        ],
        "stream": false
    });

    println!("=== Test: image_url as string ===");
    let response = client
        .post("https://api.z.ai/api/coding/paas/v4/chat/completions")
        .header(
            "Authorization",
            "Bearer 88e670ac50f74705b43579797ab8c632.9UaX3cYn7howivpZ",
        )
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    println!("Status: {}", response.status());
    println!("Response: {}\n", response.text().await?);

    Ok(())
}
