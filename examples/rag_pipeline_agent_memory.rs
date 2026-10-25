//! # Vectrust RAG Pipeline & AI Agent Memory Example
//!
//! This example demonstrates an end-to-end Retrieval-Augmented Generation (RAG)
//! pipeline and AI Agent Long-Term Memory system using Vectrust embedded engine.
//!
//! ## What this example demonstrates:
//! 1. Chunking knowledge documents into semantic passages with structured JSON metadata.
//! 2. Embedding document chunks into vector space.
//! 3. Ingesting passages into a persistent HNSW vector collection with Cosine similarity.
//! 4. Querying relevant context for a user prompt (RAG retrieval step).
//! 5. Constructing an augmented prompt ready for an LLM (e.g., OpenAI, Anthropic, or local Ollama).
//! 6. Storing and retrieving AI Agent episodic memory with session-isolated metadata pre-filtering.

use serde_json::json;
use tempfile::tempdir;
use vectrust::prelude::*;

/// A deterministic semantic vector embedder that maps text to a normalized dense vector.
/// In production, replace this with an embedding model call:
/// - FastEmbed (local ONNX)
/// - OpenAI `text-embedding-3-small` (1536-dim)
/// - HuggingFace Text Embeddings Inference (TEI)
/// - Ollama `nomic-embed-text`
fn generate_embedding(text: &str, dimension: usize) -> Vec<f32> {
    let mut vec = vec![0.0f32; dimension];
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().filter(|c| c.is_alphanumeric() || c.is_whitespace()).collect();

    // 1. Character trigram hashing for semantic root matching
    if chars.len() >= 3 {
        for window in chars.windows(3) {
            let mut h = 2166136261u32;
            for &c in window {
                h ^= c as u32;
                h = h.wrapping_mul(16777619);
            }
            let idx = (h as usize) % dimension;
            let sign = if (h >> 16) & 1 == 0 { 1.0 } else { -1.0 };
            vec[idx] += sign;
        }
    }

    // 2. Whole word hashing
    for word in lower.split_whitespace() {
        let mut h = 5381u32;
        for b in word.bytes() {
            h = ((h << 5).wrapping_add(h)).wrapping_add(b as u32);
        }
        let idx = (h as usize) % dimension;
        let sign = if (h >> 8) & 1 == 0 { 2.0 } else { -2.0 };
        vec[idx] += sign;
    }

    // L2-normalize to unit length for Cosine similarity
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in &mut vec {
            *val /= norm;
        }
    }
    vec
}

fn main() -> vectrust::Result<()> {
    println!("\x1b[1m\x1b[36m=========================================================================\x1b[0m");
    println!("\x1b[1m\x1b[36m   Vectrust: RAG Pipeline & AI Agent Semantic Memory Architecture        \x1b[0m");
    println!("\x1b[1m\x1b[36m=========================================================================\x1b[0m\n");

    let tmp = tempdir().map_err(|e| VectrustError::IoError(e))?;
    let db_path = tmp.path().join("agent_knowledge_base");

    // Dimensions: 64 for demonstration (use 1536 for OpenAI, 384 for MiniLM, 768 for BGE)
    let embedding_dim = 64;

    // -------------------------------------------------------------------------
    // Phase 1: Initialize Persistent Database & Collection for Knowledge Base
    // -------------------------------------------------------------------------
    println!("📦 \x1b[1m[1/4] Initializing Embedded Vector Storage...\x1b[0m");
    let db = Vectrust::open(&db_path)?;

    let rag_config = CollectionConfig::builder(embedding_dim, Metric::Cosine)
        .index_type(IndexType::Hnsw)
        .hnsw_config(HnswConfig {
            m: 16,
            m0: 32,
            ef_construction: 128,
            ef_search: 64,
        })
        .build();

    let kb_collection = db.create_collection("tech_docs", rag_config)?;
    println!("  Collection 'tech_docs' created (HNSW index, Cosine metric, {} dims)\n", embedding_dim);

    // -------------------------------------------------------------------------
    // Phase 2: Ingest Knowledge Base Document Chunks with Structured Metadata
    // -------------------------------------------------------------------------
    println!("📚 \x1b[1m[2/4] Ingesting Document Chunks into Vector Index...\x1b[0m");

    let document_chunks = vec![
        (
            1u64,
            "Vectrust uses an append-only Write-Ahead Log (WAL) to ensure zero data loss during power failures. Every insert and delete is synchronously or asynchronously logged with CRC32 verification.",
            json!({
                "source": "architecture.md",
                "category": "durability",
                "section": "Storage Engine",
                "access_level": "public",
                "timestamp": "2026-10-25T08:00:00Z"
            })
        ),
        (
            2u64,
            "The Hierarchical Navigable Small World (HNSW) proximity graph routes nearest neighbor queries in O(log N) time by utilizing multi-layer probabilistic skips before greedily descending to layer 0.",
            json!({
                "source": "hnsw.md",
                "category": "indexing",
                "section": "Graph Topology",
                "access_level": "public",
                "timestamp": "2026-10-25T08:10:00Z"
            })
        ),
        (
            3u64,
            "Atomic snapshot compaction in Vectrust writes clean data files into a temporary staging file before atomically replacing the target snapshot and truncating the WAL journal.",
            json!({
                "source": "architecture.md",
                "category": "durability",
                "section": "Compaction",
                "access_level": "internal",
                "timestamp": "2026-10-25T08:20:00Z"
            })
        ),
        (
            4u64,
            "SIMD acceleration in Vectrust unrolls vector distance loops by 8-way parallel accumulators, maximizing throughput on AVX-2, AVX-512, and ARM Neon architectures for Cosine and L2 metrics.",
            json!({
                "source": "simd.md",
                "category": "performance",
                "section": "SIMD Vectorization",
                "access_level": "public",
                "timestamp": "2026-10-25T08:30:00Z"
            })
        ),
        (
            5u64,
            "Deep metadata filtering in Vectrust evaluates structured JSON boolean predicates directly during graph exploration, allowing pre-filtering across nested dot-notation properties.",
            json!({
                "source": "metadata_filtering.md",
                "category": "filtering",
                "section": "Predicate Engine",
                "access_level": "public",
                "timestamp": "2026-10-25T08:40:00Z"
            })
        ),
    ];

    for (id, content, mut metadata) in document_chunks {
        // Embed the textual chunk
        let vector = generate_embedding(content, embedding_dim);

        // Store the original text inside the metadata payload for zero-overhead retrieval
        metadata["text"] = json!(content);

        let point = Point::new(id, vector).with_payload(metadata);
        kb_collection.insert(point)?;
        println!("  • Ingested Chunk ID: {:<2} | Section: {}", id, content.chars().take(55).collect::<String>() + "...");
    }

    println!("  Ingestion complete. Total vectors indexed: {}\n", kb_collection.count());

    // -------------------------------------------------------------------------
    // Phase 3: Execute RAG Retrieval & Prompt Augmentation
    // -------------------------------------------------------------------------
    println!("🔍 \x1b[1m[3/4] Running Semantic Search for User Query (RAG Step)...\x1b[0m");

    let user_query = "How does the write-ahead log ensure data safety and handle crashes during compaction?";
    println!("  User Query: \"\x1b[33m{}\x1b[0m\"\n", user_query);

    // 1. Embed user query into vector space
    let query_vector = generate_embedding(user_query, embedding_dim);

    // 2. Query Vectrust with metadata pre-filtering (e.g. only durability docs accessible to public or internal)
    let rag_query = SearchQuery::new(query_vector, 2)
        .with_filter(Filter::or(vec![
            Filter::eq("category", "durability"),
            Filter::eq("category", "indexing"),
        ]))
        .include_payload(true);

    let retrieved_chunks = kb_collection.search(rag_query)?;

    println!("  Retrieved {} Most Relevant Context Chunks:", retrieved_chunks.len());
    let mut context_snippets = Vec::new();

    for (rank, hit) in retrieved_chunks.iter().enumerate() {
        let payload = hit.payload.as_ref().unwrap();
        let text = payload["text"].as_str().unwrap();
        let source = payload["source"].as_str().unwrap();
        let section = payload["section"].as_str().unwrap();

        println!(
            "    [{}] Score: {:.4} | [{}: {}] -> {}",
            rank + 1,
            hit.score,
            source,
            section,
            text
        );
        context_snippets.push(format!("- [{}: {}] {}", source, section, text));
    }

    // 3. Assemble Augmented Prompt for the LLM
    let augmented_prompt = format!(
        "### System Instructions:\n\
        You are an expert systems assistant. Answer the user question accurately using ONLY the provided context snippets.\n\n\
        ### Context Information:\n\
        {}\n\n\
        ### User Question:\n\
        {}\n\n\
        ### Assistant Response:\n",
        context_snippets.join("\n"),
        user_query
    );

    println!("\n  \x1b[32m=== Synthesized LLM Prompt with Injected Context ===\x1b[0m");
    println!("{}\n", augmented_prompt);

    // -------------------------------------------------------------------------
    // Phase 4: AI Agent Long-Term Memory with Multi-Session Isolation
    // -------------------------------------------------------------------------
    println!("🧠 \x1b[1m[4/4] AI Agent Episodic Memory Simulation...\x1b[0m");

    let agent_config = CollectionConfig::builder(embedding_dim, Metric::Cosine)
        .index_type(IndexType::Hnsw)
        .build();
    let agent_memory = db.create_collection("agent_episodic_memory", agent_config)?;

    // Store memories for different agents and sessions
    let memories = vec![
        (
            "mem_01",
            "User prefers compact JSON output and Rust async tokio examples.",
            "agent_codex",
            "session_alpha",
            "preference",
        ),
        (
            "mem_02",
            "User deployed a Kubernetes cluster in eu-central-1 on AWS.",
            "agent_devops",
            "session_beta",
            "fact",
        ),
        (
            "mem_03",
            "User requested database password reset for the staging environment.",
            "agent_codex",
            "session_alpha",
            "action",
        ),
    ];

    for (id, thought, agent_id, session_id, memory_type) in memories {
        let vec = generate_embedding(thought, embedding_dim);
        let point = Point::new(id, vec).with_payload(json!({
            "thought": thought,
            "agent_id": agent_id,
            "session_id": session_id,
            "type": memory_type,
            "timestamp": "2026-10-25T11:00:00Z"
        }));
        agent_memory.insert(point)?;
    }

    // Query memories: Agent Codex only searches its OWN session's memories
    println!("  Agent 'agent_codex' querying memory for: \"What are the user's code preferences?\"");
    let agent_query_vec = generate_embedding("user coding preferences and output format", embedding_dim);

    let memory_search = SearchQuery::new(agent_query_vec, 1)
        .with_filter(Filter::and(vec![
            Filter::eq("agent_id", "agent_codex"),
            Filter::eq("session_id", "session_alpha"),
        ]))
        .include_payload(true);

    let recalled_memories = agent_memory.search(memory_search)?;

    if let Some(top_mem) = recalled_memories.first() {
        let payload = top_mem.payload.as_ref().unwrap();
        println!("  \x1b[32mRecalled Memory:\x1b[0m \"{}\"", payload["thought"].as_str().unwrap());
        println!("  (Session: {}, Relevance Score: {:.4})", payload["session_id"], top_mem.score);
    }

    // Flush and commit database state
    db.flush_all()?;
    println!("\n\x1b[32m✔ Pipeline successfully executed! All embeddings persisted to disk.\x1b[0m");

    Ok(())
}
