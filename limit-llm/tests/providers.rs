use limit_llm::ProviderResponseChunk;

#[test]
fn test_reasoning_delta_variant() {
    let chunk = ProviderResponseChunk::ReasoningDelta("test reasoning".to_string());
    assert!(matches!(chunk, ProviderResponseChunk::ReasoningDelta(_)));
}
