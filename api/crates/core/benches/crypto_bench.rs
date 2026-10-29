//! Cryptographic Performance Benchmarks
//!
//! Measures throughput and execution latency for AES-256-GCM authenticated encryption/decryption
//! and Argon2id master key derivation against performance budgets.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use roubit_core::crypto::{AesGcmCipher, KdfService};

fn bench_aes_gcm_encrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("aes_256_gcm_encrypt");
    let key = [42u8; 32];

    for &size in &[1024, 10 * 1024, 100 * 1024] {
        let payload = vec![0xABu8; size];
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size / 1024)),
            &payload,
            |b, data| {
                b.iter(|| {
                    let _ = AesGcmCipher::encrypt(&key, black_box(data), None);
                });
            },
        );
    }
    group.finish();
}

fn bench_aes_gcm_decrypt(c: &mut Criterion) {
    let mut group = c.benchmark_group("aes_256_gcm_decrypt");
    let key = [42u8; 32];
    let size = 10 * 1024; // 10 KB typical journal reflection
    let payload = vec![0xCDu8; size];
    let envelope =
        AesGcmCipher::encrypt(&key, &payload, None).expect("pre-encryption must succeed");

    group.throughput(Throughput::Bytes(size as u64));
    group.bench_function("10KB", |b| {
        b.iter(|| {
            let _ = AesGcmCipher::decrypt(&key, black_box(&envelope), None);
        });
    });
    group.finish();
}

fn bench_argon2id_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("argon2id_kdf");
    group.sample_size(10); // OWASP parameters are memory-hard; sample size 10 is reliable

    let kdf = KdfService::default_recommended();
    let passphrase = b"super-secret-user-passphrase-2026";
    let salt = [77u8; 16];

    group.bench_function("owasp_64mb_3iter_4lanes", |b| {
        b.iter(|| {
            let _ = kdf.derive_key(black_box(passphrase), black_box(&salt));
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_aes_gcm_encrypt,
    bench_aes_gcm_decrypt,
    bench_argon2id_derivation
);
criterion_main!(benches);
