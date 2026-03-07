use bincode::{deserialize, serialize};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Message {
    role: Role,
    content: String,
}

fn main() {
    let msg = Message {
        role: Role::User,
        content: "Hello".to_string(),
    };
    
    let serialized = serialize(&msg).expect("Serialization failed");
    println!("Serialized length: {}", serialized.len());
    println!("Serialized bytes: {:?}", serialized);
    
    let deserialized: Message = deserialize(&serialized).expect("Deserialization failed");
    println!("Deserialized: {:?}", deserialized);
    
    assert_eq!(deserialized.role, Role::User);
    assert_eq!(deserialized.content, "Hello");
}
