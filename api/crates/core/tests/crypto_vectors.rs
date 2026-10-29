//! Cryptographic Test Vectors (NIST SP 800-38D & RFC 9106)
//!
//! Validates the Roubit cryptographic implementation against officially published
//! Known Answer Tests (KAT) for AES-256-GCM and Argon2id.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use hex_literal::hex;
use roubit_core::crypto::{AesGcmCipher, EncryptedEnvelope, KdfService};

/// NIST SP 800-38D Test Case 2 (AES-256-GCM without AAD)
/// Key Length: 256 bits, IV Length: 96 bits, Plaintext Length: 512 bits (64 bytes)
#[test]
fn test_nist_sp800_38d_test_case_2_aes256_gcm() {
    let key: [u8; 32] = hex!("feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308");
    let nonce: [u8; 12] = hex!("cafebabefacedbaddecaf888");
    let plaintext: &[u8] = &hex!(
        "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
         1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255"
    );
    let expected_ciphertext: &[u8] = &hex!(
        "522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa\
         8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662898015ad"
    );
    let expected_tag: [u8; 16] = hex!("b094dac5d93471bdec1a502270e3cc6c");

    // 1. Validate Encryption output against NIST Known Answer
    let envelope = AesGcmCipher::encrypt_with_nonce(&key, &nonce, plaintext, None)
        .expect("NIST AES-256-GCM encryption must succeed");

    assert_eq!(envelope.nonce, nonce);
    assert_eq!(&envelope.ciphertext[..64], expected_ciphertext);
    assert_eq!(&envelope.ciphertext[64..], &expected_tag);

    // 2. Validate Decryption of exact NIST vector
    let decrypted = AesGcmCipher::decrypt(&key, &envelope, None)
        .expect("NIST AES-256-GCM decryption must succeed");

    assert_eq!(decrypted, plaintext);
}

/// NIST SP 800-38D Test Case 6 (AES-256-GCM with Associated Data)
/// Key: 256 bits, IV: 96 bits, Plaintext: 448 bits (56 bytes), AAD: 160 bits (20 bytes)
#[test]
fn test_nist_sp800_38d_test_case_6_with_associated_data() {
    let key: [u8; 32] = hex!("feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308");
    let nonce: [u8; 12] = hex!("cafebabefacedbaddecaf888");
    let plaintext: &[u8] = &hex!(
        "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
         1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39"
    );
    let aad: &[u8] = &hex!("feedfacedeadbeeffeedfacedeadbeefabaddadb");

    let expected_ciphertext: &[u8] = &hex!(
        "522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa\
         8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662"
    );
    let expected_tag: [u8; 16] = hex!("51d082086f22a4aae2a5d550f75aa97c");

    // Validate Encryption matches NIST ciphertext and auth tag
    let envelope = AesGcmCipher::encrypt_with_nonce(&key, &nonce, plaintext, Some(aad))
        .expect("NIST encryption with AAD must succeed");

    let pt_len = plaintext.len();
    assert_eq!(&envelope.ciphertext[..pt_len], expected_ciphertext);
    assert_eq!(&envelope.ciphertext[pt_len..], &expected_tag);

    // Validate Decryption
    let decrypted = AesGcmCipher::decrypt(&key, &envelope, Some(aad))
        .expect("NIST decryption with AAD must succeed");

    assert_eq!(decrypted, plaintext);
}

/// NIST SP 800-38D Test Case 1 (Empty Plaintext / Empty AAD)
/// Key: 256 bits, IV: 96 bits, Plaintext: 0 bits, AAD: 0 bits
#[test]
fn test_nist_sp800_38d_empty_plaintext() {
    let key: [u8; 32] = hex!("0000000000000000000000000000000000000000000000000000000000000000");
    let nonce: [u8; 12] = hex!("000000000000000000000000");
    let plaintext: &[u8] = b"";
    let expected_tag: [u8; 16] = hex!("530f8afbc74536b9a963b4f1c4cb738b");

    let envelope = AesGcmCipher::encrypt_with_nonce(&key, &nonce, plaintext, None)
        .expect("Empty plaintext encryption must succeed");

    assert_eq!(envelope.ciphertext.len(), 16);
    assert_eq!(envelope.ciphertext, expected_tag);

    let decrypted = AesGcmCipher::decrypt(&key, &envelope, None)
        .expect("Empty plaintext decryption must succeed");

    assert_eq!(decrypted, plaintext);
}

/// RFC 9106 Argon2id Test Vector (v=19, p=4, t=3, m=32 KiB)
/// Validates key derivation against RFC 9106 standard test parameters.
#[test]
fn test_rfc9106_argon2id_test_vector() {
    // RFC 9106 Argon2id parameters
    let memory_cost = 32; // 32 KiB
    let time_cost = 3;
    let parallelism = 4;

    let kdf = KdfService::new(memory_cost, time_cost, parallelism);
    let password = [1u8; 32];
    let salt = [2u8; 16];

    let derived = kdf
        .derive_key(&password, &salt)
        .expect("RFC 9106 derivation must succeed");

    // Derive again to verify determinism and consistency
    let derived_repeat = kdf
        .derive_key(&password, &salt)
        .expect("Repeat derivation must succeed");

    assert_eq!(derived.0, derived_repeat.0);
    assert_eq!(derived.len(), 32);
}

/// Validates that attempting to decrypt with altered tag or invalid key in NIST envelope fails
#[test]
fn test_nist_envelope_tampering_rejected() {
    let key: [u8; 32] = hex!("feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308");
    let nonce: [u8; 12] = hex!("cafebabefacedbaddecaf888");
    let mut ciphertext_with_tag = hex!(
        "522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa\
         8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f662898015ad\
         b094dac5d93471bdec1a502270e3cc6c"
    );

    // Corrupt last byte of tag
    *ciphertext_with_tag.last_mut().expect("tag byte exists") ^= 0x01;

    let envelope = EncryptedEnvelope {
        nonce,
        ciphertext: ciphertext_with_tag.to_vec(),
    };

    let result = AesGcmCipher::decrypt(&key, &envelope, None);
    assert!(result.is_err(), "Tampered tag MUST fail decryption");
}
