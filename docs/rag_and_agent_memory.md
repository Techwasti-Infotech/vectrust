# 🤖 RAG Pipelines & AI Agent Memory with Vectrust

This guide demonstrates how to use **Vectrust** as an in-process, embedded vector store for **Retrieval-Augmented Generation (RAG)**, **AI Agent long-term episodic memory**, and **local LLM knowledge retrieval**.

---

## ⚡ Why Use an Embedded Vector Database for AI?

| Feature | Standalone Vector DB (Cloud / Daemon) | Vectrust (Embedded In-Process) |
| :--- | :--- | :--- |
| **Query Latency** | 15 – 60 ms (HTTP/gRPC network hop) | **< 0.1 ms** (Direct memory pointer) |
| **Infrastructure** | Requires Docker, network ports, auth | **Zero setup** (Pure Rust library crate) |
| **Privacy & Security**| Vectors sent over network | **100% on-device** (Air-gapped safe) |
| **Memory Footprint**| Hundreds of MBs / GBs daemon | **Minimal** (Allocates only what you index) |
| **Storage Model** | External storage cluster | **Single directory** (`wal.log` + `snapshot.bin`) |

---

## 🏗️ Architectural Flow

```text
1. Document Ingestion Phase:
   [Raw Markdown / Docs] 
           │
           ▼
   [Text Chunking] ──────────┐
           │                 │ Raw Text & Metadata
           ▼                 ▼
   [Embedding Model] ───► [Vectrust Collection]
   (OpenAI / FastEmbed)   • HNSW Proximity Graph
                          • Vector (float32 array)
                          • Payload JSON: { text, source, session_id }

2. Query & Augmentation Phase:
   [User Question]
           │
           ▼
   [Query Embedding] ───► [Vectrust Fast Search]
                          • Sub-millisecond ANN
                          • Metadata Pre-Filtering
                                  │
                                  ▼
                          [Top-K Context Chunks]
                                  │
                                  ▼
   [LLM Prompt Assembly] ◄────────┘
           │
           ▼
   [LLM Generation (Ollama / OpenAI / Claude)]
           │
           ▼
   [Grounded Accurate Answer]
```

---

## 🚀 Complete RAG Implementation Example

Here is how to build a RAG pipeline in pure Rust:

```rust
use serde_json::json;
use vectrust::prelude::*;

fn main() -> vectrust::Result<()> {
    // 1. Initialize persistent storage for the knowledge base
    let db = Vectrust::open("./data/agent_kb")?;

    // 2. Configure 1536-dimensional collection (e.g. OpenAI text-embedding-3-small)
    let config = CollectionConfig::builder(1536, Metric::Cosine)
        .index_type(IndexType::Hnsw)
        .build();

    let kb = db.get_or_create_collection("manuals", config)?;

    // 3. Ingest document chunks with raw text in payload
    let chunk_text = "Vectrust provides crash-resilient persistence using an append-only WAL and CRC32 verification.";
    let chunk_embedding = vec![0.02f32; 1536]; // Replace with real embedding model

    kb.insert(
        Point::new("chunk-001", chunk_embedding).with_payload(json!({
            "text": chunk_text,
            "source": "architecture.md",
            "section": "Durability",
            "access": "public"
        })),
    )?;

    // 4. Retrieve context for a user prompt
    let query_vec = vec![0.02f32; 1536]; // Embed user prompt
    let query = SearchQuery::new(query_vec, 3)
        .with_filter(Filter::eq("access", "public"))
        .include_payload(true);

    let hits = kb.search(query)?;

    // 5. Build augmented prompt
    let mut context_snippets = Vec::new();
    for hit in hits {
        let text = hit.payload.unwrap()["text"].as_str().unwrap().to_string();
        context_snippets.push(text);
    }

    let augmented_prompt = format!(
        "Use the following context to answer the question:\n\n{}\n\nQuestion: How does persistence work?",
        context_snippets.join("\n---\n")
    );

    println!("{}", augmented_prompt);
    Ok(())
}
```

---

## 🧠 AI Agent Long-Term Episodic Memory

Autonomous AI agents (e.g. coding assistants, research agents, conversational bots) need long-term memory across sessions.

With Vectrust, you can isolate memories per agent and session using **metadata pre-filtering**:

```rust
use serde_json::json;
use vectrust::prelude::*;

fn store_agent_memory(
    collection: &Collection,
    memory_id: &str,
    embedding: Vec<f32>,
    thought: &str,
    agent_id: &str,
    session_id: &str,
) -> vectrust::Result<()> {
    collection.insert(
        Point::new(memory_id, embedding).with_payload(json!({
            "thought": thought,
            "agent_id": agent_id,
            "session_id": session_id,
            "timestamp": "2026-10-25T11:00:00Z"
        })),
    )
}

fn recall_agent_memories(
    collection: &Collection,
    query_vec: Vec<f32>,
    agent_id: &str,
    session_id: &str,
    k: usize,
) -> vectrust::Result<Vec<String>> {
    let query = SearchQuery::new(query_vec, k)
        .with_filter(Filter::and(vec![
            Filter::eq("agent_id", agent_id),
            Filter::eq("session_id", session_id),
        ]))
        .include_payload(true);

    let results = collection.search(query)?;
    Ok(results
        .into_iter()
        .filter_map(|r| r.payload)
        .filter_map(|p| p["thought"].as_str().map(String::from))
        .collect())
}
```

---

## 🔌 Recommended Embedding Crates

To generate embeddings in Rust, you can use:

1. **`fastembed`** (Local ONNX models — zero API keys):
   ```toml
   [dependencies]
   fastembed = "4.0"
   ```
   Runs `BGE-small-en`, `All-MiniLM-L6-v2`, or `nomic-embed-text` locally via ONNX Runtime.

2. **`async-openai`** (Cloud or self-hosted API):
   ```toml
   [dependencies]
   async-openai = "0.27"
   ```
   Connects to OpenAI `text-embedding-3-small` or local Ollama `/v1/embeddings` endpoints.

---

## 🏃 Running the Included Example

A runnable end-to-end RAG and agent memory example is included:

```bash
cargo run --example rag_pipeline_agent_memory
```
