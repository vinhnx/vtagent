use vtagent_core::core::agent::types::MessageType;
use vtagent_core::core::agent::{CompactionConfig, CompactionEngine};
use vtagent_core::gemini::Content;

#[tokio::test]
async fn compaction_engine_compacts_messages() {
    let mut config = CompactionConfig::default();
    config.max_uncompressed_messages = 1;
    let engine = CompactionEngine::with_config(config);

    let msg = Content::user_text("hello world");
    engine
        .add_message(&msg, MessageType::UserMessage)
        .await
        .unwrap();
    engine
        .add_message(&msg, MessageType::UserMessage)
        .await
        .unwrap();

    assert!(engine.should_compact().await.unwrap());
    let result = engine.compact_messages_intelligently().await.unwrap();
    assert_eq!(result.messages_compacted, 1);
    let stats = engine.get_statistics().await.unwrap();
    assert_eq!(stats.total_messages, 1);
}
