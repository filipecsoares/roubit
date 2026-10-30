/**
 * Client-Side Web Crypto API & Argon2id WASM Service (RF04)
 *
 * Implements client-side selective encryption for sensitive journal reflections.
 * - Key Derivation: Argon2id via WASM (memory: 64MiB, iterations: 3, parallelism: 4)
 * - Encryption: AES-256-GCM authenticated cipher with 12-byte random IV
 * - Non-Exportable: CryptoKey is retained exclusively in volatile browser RAM
 */

import { argon2id } from 'hash-wasm'

export interface EncryptedPayload {
  saltBase64: string
  nonceBase64: string
  ciphertextBase64: string
}

export function uint8ArrayToBase64(bytes: Uint8Array): string {
  let binary = ''
  const len = bytes.byteLength
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return btoa(binary)
}

export function base64ToUint8Array(base64: string): Uint8Array {
  const binaryString = atob(base64)
  const len = binaryString.length
  const bytes = new Uint8Array(len)
  for (let i = 0; i < len; i++) {
    bytes[i] = binaryString.charCodeAt(i)
  }
  return bytes
}

export class WebCryptoService {
  /**
   * Generates 16 cryptographically secure random bytes for Argon2id salt.
   */
  static generateSalt(): Uint8Array {
    return crypto.getRandomValues(new Uint8Array(16))
  }

  /**
   * Derives a non-exportable 256-bit AES-GCM CryptoKey using Argon2id via WASM.
   */
  static async deriveKey(passphrase: string, salt: Uint8Array): Promise<CryptoKey> {
    if (!passphrase || passphrase.length < 1) {
      throw new Error('Passphrase cannot be empty')
    }
    if (salt.length !== 16) {
      throw new Error('Salt must be exactly 16 bytes')
    }

    const rawKey = await argon2id({
      password: passphrase,
      salt,
      iterations: 3,
      memorySize: 65536, // 64 MiB in KiB
      parallelism: 4,
      hashLength: 32, // 256-bit key
      outputType: 'binary',
    })

    return crypto.subtle.importKey(
      'raw',
      rawKey as unknown as BufferSource,
      { name: 'AES-GCM', length: 256 },
      false, // non-extractable in memory
      ['encrypt', 'decrypt'],
    )
  }

  /**
   * Encrypts plaintext string using AES-256-GCM with a freshly generated 12-byte IV.
   * Returns a Base64-encoded envelope containing salt, nonce, and ciphertext + auth tag.
   */
  static async encrypt(plaintext: string, key: CryptoKey, salt: Uint8Array): Promise<EncryptedPayload> {
    const nonce = crypto.getRandomValues(new Uint8Array(12))
    const encoded = new TextEncoder().encode(plaintext)

    const ciphertextBuffer = await crypto.subtle.encrypt(
      {
        name: 'AES-GCM',
        iv: nonce as unknown as BufferSource,
        tagLength: 128, // 16-byte authentication tag
      },
      key,
      encoded,
    )

    return {
      saltBase64: uint8ArrayToBase64(salt),
      nonceBase64: uint8ArrayToBase64(nonce),
      ciphertextBase64: uint8ArrayToBase64(new Uint8Array(ciphertextBuffer)),
    }
  }

  /**
   * Decrypts Base64-encoded envelope and verifies the 128-bit authentication tag.
   */
  static async decrypt(payload: EncryptedPayload, key: CryptoKey): Promise<string> {
    const nonce = base64ToUint8Array(payload.nonceBase64)
    const ciphertext = base64ToUint8Array(payload.ciphertextBase64)

    if (nonce.length !== 12) {
      throw new Error('Invalid nonce length: expected 12 bytes')
    }
    if (ciphertext.length < 16) {
      throw new Error('Invalid ciphertext length: missing authentication tag')
    }

    const decryptedBuffer = await crypto.subtle.decrypt(
      {
        name: 'AES-GCM',
        iv: nonce as unknown as BufferSource,
        tagLength: 128,
      },
      key,
      ciphertext as unknown as BufferSource,
    )

    return new TextDecoder().decode(decryptedBuffer)
  }
}

export default WebCryptoService
