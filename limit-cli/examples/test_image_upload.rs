use reqwest;
use serde_json;
use base64::{Engine as _, engine::general_purpose};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // Tiny 1x1 red PNG pixel
    let tiny_png = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00,
        0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F,
        0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x60, 0xE8, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82
    ];
    let base64_img = general_purpose::STANDARD.encode(&tiny_png);
    
    let request = serde_json::json!({
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
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", base64_img)
                        }
                    }
                ]
            }
        ],
        "stream": false
    });
    
    println!("Testing z.ai API with image (OpenAI format)...\n");
    println!("Request: {}\n", serde_json::to_string_pretty(&request).unwrap());
    
    let response = client
        .post("https://api.z.ai/api/coding/paas/v4/chat/completions")
        .header("Authorization", "Bearer 88e670ac50f74705b43579797ab8c632.9UaX3cYn7howivpZ")
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;
    
    println!("Status: {}", response.status());
    let text = response.text().await?;
    println!("Response: {}\n", text);
    
    Ok(())
}
