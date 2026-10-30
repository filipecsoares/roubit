//! Automated concurrent load test and P99 latency benchmark (RNF04)
//!
//! Validates:
//! 1. Tail latency: P99 < 10ms under concurrent traffic.
//! 2. Memory safety: Process RSS footprint < 25MB RAM.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::uninlined_format_args,
    clippy::too_many_lines
)]

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use roubit_core::{create_sqlite_pool, run_migrations, AppConfig};
use roubit_server::{build_router, AppState};

fn get_process_rss_mb() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            let mut parts = statm.split_whitespace();
            let _total_pages = parts.next();
            if let Some(resident_pages_str) = parts.next() {
                if let Ok(resident_pages) = resident_pages_str.parse::<u64>() {
                    let page_size = 4096; // 4KB typical Linux page
                    let bytes = resident_pages * page_size;
                    return Some((bytes as f64) / (1024.0 * 1024.0));
                }
            }
        }
    }
    None
}

#[tokio::test]
#[allow(clippy::unwrap_used, clippy::cast_precision_loss)]
async fn test_load_p99_latency_and_memory_footprint() {
    // 1. Initialize in-memory database and server state
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Failed to initialize memory SQLite");
    run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    let config = AppConfig {
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        jwt_secret: "benchmark_secret_key_32_characters_long!".to_string(),
        ..Default::default()
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral TCP port");
    let addr = listener.local_addr().expect("Local address");

    let state = AppState::new(pool, config);
    let app = build_router(state);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("Server exited with error");
    });

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(50)
        .build()
        .expect("Reqwest client");

    // 2. Register test user
    let reg_url = format!("http://{addr}/api/v1/auth/register");
    let reg_body = serde_json::json!({
        "email": "p99_bench@roubit.app",
        "password": "Password123!"
    });
    let reg_res = client
        .post(&reg_url)
        .json(&reg_body)
        .send()
        .await
        .expect("Register failed");
    assert_eq!(reg_res.status(), reqwest::StatusCode::CREATED);
    let auth: roubit_server::AuthResponse = reg_res.json().await.expect("Valid auth response");

    // 3. Create sample habit
    let habit_url = format!("http://{addr}/api/v1/habits");
    let habit_body = serde_json::json!({
        "name": "P99 Hydration Target",
        "habit_type": "binary",
        "target_value": 1.0,
        "frequency_days": 127
    });
    let habit_res = client
        .post(&habit_url)
        .bearer_auth(&auth.token)
        .header("Idempotency-Key", "setup-habit-p99")
        .json(&habit_body)
        .send()
        .await
        .expect("Create habit failed");
    assert_eq!(habit_res.status(), reqwest::StatusCode::CREATED);

    // 4. Concurrent load simulation: 1,000 requests across 20 workers
    let total_requests = 1000usize;
    let concurrency = 20usize;
    let requests_per_worker = total_requests / concurrency;

    let shared_client = Arc::new(client);
    let token = Arc::new(auth.token);
    let endpoint = Arc::new(format!("http://{addr}/api/v1/habits"));

    let mut handles = Vec::with_capacity(concurrency);

    let overall_start = Instant::now();

    for worker_id in 0..concurrency {
        let w_client = Arc::clone(&shared_client);
        let w_token = Arc::clone(&token);
        let w_url = Arc::clone(&endpoint);

        handles.push(tokio::spawn(async move {
            let mut worker_latencies = Vec::with_capacity(requests_per_worker);

            for req_idx in 0..requests_per_worker {
                let start = Instant::now();
                let res = w_client
                    .get(&*w_url)
                    .bearer_auth(&*w_token)
                    .header("Idempotency-Key", format!("load-{worker_id}-{req_idx}"))
                    .send()
                    .await;

                let elapsed = start.elapsed();
                if let Ok(response) = res {
                    if response.status() == reqwest::StatusCode::OK {
                        worker_latencies.push(elapsed);
                    }
                }
            }

            worker_latencies
        }));
    }

    let mut all_latencies = Vec::with_capacity(total_requests);
    for handle in handles {
        let latencies = handle.await.expect("Worker panic");
        all_latencies.extend(latencies);
    }

    let total_duration = overall_start.elapsed();
    assert_eq!(
        all_latencies.len(),
        total_requests,
        "All requests must succeed with 200 OK"
    );

    // 5. Compute percentile latencies
    all_latencies.sort();

    let p50 = all_latencies[total_requests * 50 / 100];
    let p90 = all_latencies[total_requests * 90 / 100];
    let p95 = all_latencies[total_requests * 95 / 100];
    let p99 = all_latencies[total_requests * 99 / 100];
    let max = all_latencies[total_requests - 1];

    let rps = (total_requests as f64) / total_duration.as_secs_f64();

    println!("\n========== ROUBIT LOAD BENCHMARK (RNF04) ==========");
    println!("Total Requests:      {}", total_requests);
    println!("Concurrency (VUs):   {}", concurrency);
    println!("Throughput:          {:.2} req/sec", rps);
    println!("Total Duration:      {:.2?}", total_duration);
    println!("Min Latency:         {:.3?}", all_latencies[0]);
    println!("P50 (Median):        {:.3?}", p50);
    println!("P90 Latency:         {:.3?}", p90);
    println!("P95 Latency:         {:.3?}", p95);
    println!("P99 Latency:         {:.3?}", p99);
    println!("Max Latency:         {:.3?}", max);

    // 6. Memory RSS Check
    if let Some(rss_mb) = get_process_rss_mb() {
        println!(
            "Process RSS Memory:  {:.2} MB (Threshold: < 25.0 MB)",
            rss_mb
        );
        assert!(
            rss_mb < 25.0,
            "RSS memory ({:.2}MB) must be below 25MB",
            rss_mb
        );
    } else {
        println!("Process RSS Memory:  [Unavailable on this platform]");
    }
    println!("===================================================\n");

    // RNF04 Requirement: P99 < 10ms in release mode (allow up to 50ms in unoptimized debug test profile)
    let max_allowed = if cfg!(debug_assertions) {
        Duration::from_millis(50)
    } else {
        Duration::from_millis(10)
    };
    assert!(
        p99 < max_allowed,
        "P99 latency ({p99:.3?}) exceeded threshold ({max_allowed:.3?})"
    );

    // Clean shutdown
    let _ = shutdown_tx.send(());
    tokio::time::timeout(Duration::from_secs(3), server_handle)
        .await
        .expect("Clean shutdown")
        .expect("Server panic");
}
