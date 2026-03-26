use reqwest;
use serde_json;
use base64::{Engine as _, engine::general_purpose};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // Test 1: Image as URL instead of base64
    let request1 = serde_json::json!({
        "model": "glm-5",
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "What's in this image?"
                    },
                    {
                        "type": "image_url",
                        "image_url": "https://via.placeholder.com/150"
                    }
                ]
            }
        ],
        "stream": false
    });
    
    println!("=== Test 1: Image URL (string, not object) ===");
    let response = client
        .post("https://api.z.ai/api/coding/paas/v4/chat/completions")
        .header("Authorization", "Bearer 88e670ac50f74705b43579797ab8c632.9UaX3cYn7howivpZ")
        .header("Content-Type", "application/json")
        .json(&request1)
        .send()
        .await?;
    
    println!("Status: {}", response.status());
    let text = response.text().await?;
    println!("Response: {}\n", text);
    
    Ok(())
}
