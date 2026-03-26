use reqwest;
use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    let request = serde_json::json!({
        "model": "glm-5",
        "messages": [
            {
                "role": "user",
                "content": "Hello, can you see images?"
            }
        ],
        "stream": false
    });
    
    println!("Testing z.ai API with text-only message...\n");
    
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
