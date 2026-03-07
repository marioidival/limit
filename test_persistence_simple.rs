use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestMessage {
    role: String,
    content: String,
}

fn main() {
    let msg = TestMessage {
        role: "User".to_string(),
        content: "Hello".to_string(),
    };
    
    let serialized = serialize(&msg).expect("Serialization failed");
    println!("Serialized: {:?}", serialized);
    
    let deserialized: TestMessage = deserialize(&serialized).expect("Deserialization failed");
    println!("Deserialized: {:?}", deserialized);
}
