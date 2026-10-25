use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use rand::Rng;
use vectrust::prelude::*;

#[test]
fn test_concurrent_readers_and_writers() -> vectrust::Result<()> {
    let dim = 8;
    let db = Vectrust::in_memory();
    let config = CollectionConfig::builder(dim, Metric::Cosine)
        .index_type(IndexType::Hnsw)
        .build();
    let collection = Arc::new(db.create_collection("concurrent_test", config)?);

    // Seed collection with initial points
    for i in 0..50 {
        collection.insert(Point::new(i as u64, vec![0.5; dim]))?;
    }

    let running = Arc::new(AtomicBool::new(true));
    let mut handles = Vec::new();

    // Spawn 3 concurrent writer threads
    for thread_id in 0..3 {
        let col = Arc::clone(&collection);
        let run = Arc::clone(&running);
        handles.push(thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut counter = 1000 + thread_id * 1000;
            while run.load(Ordering::Relaxed) {
                let vec: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
                let _ = col.insert(Point::new(counter as u64, vec));
                counter += 1;
                thread::sleep(Duration::from_millis(2));
            }
        }));
    }

    // Spawn 4 concurrent reader threads
    for _ in 0..4 {
        let col = Arc::clone(&collection);
        let run = Arc::clone(&running);
        handles.push(thread::spawn(move || {
            let mut rng = rand::thread_rng();
            while run.load(Ordering::Relaxed) {
                let q_vec: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
                let query = SearchQuery::new(q_vec, 5);
                let res = col.search(query);
                assert!(res.is_ok());
                thread::sleep(Duration::from_millis(1));
            }
        }));
    }

    // Let the workload run for 200 milliseconds
    thread::sleep(Duration::from_millis(200));
    running.store(false, Ordering::Relaxed);

    for handle in handles {
        handle.join().expect("Worker thread panicked!");
    }

    // Collection count must have grown beyond initial 50
    assert!(collection.count() > 50);

    // Test parallel batch search
    let queries: Vec<SearchQuery> = (0..20)
        .map(|_| SearchQuery::new(vec![0.1; dim], 3))
        .collect();

    let batch_results = collection.search_batch(queries)?;
    assert_eq!(batch_results.len(), 20);
    for res in batch_results {
        assert!(!res.is_empty());
    }

    Ok(())
}
