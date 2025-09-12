//! Test for embedding functionality

use tempfile::TempDir;
use vtagent_core::embeddings::{EmbeddingManager, EmbeddingMetadata};

#[test]
fn test_embedding_manager_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let embeddings_dir = temp_dir.path().join("embeddings");
    let retrieval_dir = temp_dir.path().join("retrieval");
    
    let manager = EmbeddingManager::new(embeddings_dir.clone(), retrieval_dir.clone())
        .expect("Failed to create embedding manager");
    
    assert_eq!(manager.embeddings_dir, embeddings_dir);
    assert_eq!(manager.retrieval_dir, retrieval_dir);
}

#[test]
fn test_embedding_generation_and_storage() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let embeddings_dir = temp_dir.path().join("embeddings");
    let retrieval_dir = temp_dir.path().join("retrieval");
    
    let mut manager = EmbeddingManager::new(embeddings_dir, retrieval_dir)
        .expect("Failed to create embedding manager");
    
    let metadata = EmbeddingMetadata {
        source_path: "/test/file.txt".to_string(),
        content_hash: "test_hash".to_string(),
        content_type: "test".to_string(),
        tags: vec![],
        file_size: 100,
        last_modified: 1234567890,
    };
    
    let embedding = manager.generate_embedding("test content", metadata)
        .expect("Failed to generate embedding");
    
    manager.store_embedding(embedding.clone())
        .expect("Failed to store embedding");
    
    let loaded_embedding = manager.load_embedding(&embedding.id)
        .expect("Failed to load embedding")
        .expect("Embedding should exist");
    
    assert_eq!(loaded_embedding.id, embedding.id);
    assert_eq!(loaded_embedding.vector.len(), 128); // Mock vector size
}

#[test]
fn test_cosine_similarity() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let embeddings_dir = temp_dir.path().join("embeddings");
    let retrieval_dir = temp_dir.path().join("retrieval");
    
    let manager = EmbeddingManager::new(embeddings_dir, retrieval_dir)
        .expect("Failed to create embedding manager");
    
    // Test identical vectors (should have similarity of 1.0)
    let vec1 = vec![1.0, 0.0, 0.0];
    let vec2 = vec![1.0, 0.0, 0.0];
    let similarity = manager.cosine_similarity(&vec1, &vec2);
    assert!((similarity - 1.0).abs() < 0.001);
    
    // Test orthogonal vectors (should have similarity of 0.0)
    let vec1 = vec![1.0, 0.0, 0.0];
    let vec2 = vec![0.0, 1.0, 0.0];
    let similarity = manager.cosine_similarity(&vec1, &vec2);
    assert!((similarity - 0.0).abs() < 0.001);
    
    // Test opposite vectors (should have similarity of -1.0)
    let vec1 = vec![1.0, 0.0, 0.0];
    let vec2 = vec![-1.0, 0.0, 0.0];
    let similarity = manager.cosine_similarity(&vec1, &vec2);
    assert!((similarity + 1.0).abs() < 0.001);
}