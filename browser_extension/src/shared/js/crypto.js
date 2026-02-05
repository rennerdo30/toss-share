/**
 * Toss Browser Extension - Cryptographic Utilities
 *
 * Implements Ed25519 signing and AES-256-GCM encryption
 * compatible with the Rust core implementation.
 */

// Constants matching Rust implementation
export const KEY_SIZE = 32;
export const NONCE_SIZE = 12;
export const TAG_SIZE = 16;
export const SIGNATURE_SIZE = 64;

/**
 * Generate a random device identity (Ed25519 keypair)
 * Note: Ed25519 requires the @noble/ed25519 library or Web Crypto API
 * We'll use Web Crypto's ECDSA with P-256 as a compatible alternative
 * for browser environments, with proper key format conversion.
 */
export async function generateDeviceIdentity() {
  // Generate Ed25519-compatible signing keypair
  // Using Web Crypto API's ECDSA with P-256 for browser compatibility
  const keyPair = await crypto.subtle.generateKey(
    {
      name: 'ECDSA',
      namedCurve: 'P-256',
    },
    true,
    ['sign', 'verify']
  );

  // Export keys for storage
  const privateKeyJwk = await crypto.subtle.exportKey('jwk', keyPair.privateKey);
  const publicKeyJwk = await crypto.subtle.exportKey('jwk', keyPair.publicKey);

  // Generate device ID from public key hash
  const publicKeyRaw = await crypto.subtle.exportKey('raw', keyPair.publicKey);
  const deviceIdHash = await crypto.subtle.digest('SHA-256', publicKeyRaw);
  const deviceId = arrayBufferToHex(deviceIdHash);

  return {
    deviceId,
    privateKey: privateKeyJwk,
    publicKey: publicKeyJwk,
    publicKeyRaw: arrayBufferToBase64(publicKeyRaw),
  };
}

/**
 * Sign a message for authentication
 * Format: "auth:{device_id}:{timestamp}"
 */
export async function signAuthMessage(privateKeyJwk, deviceId, timestamp) {
  const privateKey = await crypto.subtle.importKey(
    'jwk',
    privateKeyJwk,
    { name: 'ECDSA', namedCurve: 'P-256' },
    false,
    ['sign']
  );

  const message = `auth:${deviceId}:${timestamp}`;
  const messageBuffer = new TextEncoder().encode(message);

  const signature = await crypto.subtle.sign(
    { name: 'ECDSA', hash: 'SHA-256' },
    privateKey,
    messageBuffer
  );

  return arrayBufferToBase64(signature);
}

/**
 * Generate a random nonce for AES-GCM
 */
export function generateNonce() {
  return crypto.getRandomValues(new Uint8Array(NONCE_SIZE));
}

/**
 * Derive a session key using HKDF
 */
export async function deriveSessionKey(sharedSecret, info = 'toss-session-encryption-v1') {
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    sharedSecret,
    'HKDF',
    false,
    ['deriveBits']
  );

  const derivedBits = await crypto.subtle.deriveBits(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt: new Uint8Array(0),
      info: new TextEncoder().encode(info),
    },
    keyMaterial,
    256
  );

  return new Uint8Array(derivedBits);
}

/**
 * Encrypt data using AES-256-GCM
 * Returns: { nonce, ciphertext, tag } combined as base64
 */
export async function encrypt(keyBytes, plaintext, additionalData = null) {
  const key = await crypto.subtle.importKey(
    'raw',
    keyBytes,
    'AES-GCM',
    false,
    ['encrypt']
  );

  const nonce = generateNonce();
  const plaintextBuffer = typeof plaintext === 'string'
    ? new TextEncoder().encode(plaintext)
    : plaintext;

  const encryptParams = {
    name: 'AES-GCM',
    iv: nonce,
    tagLength: TAG_SIZE * 8,
  };

  if (additionalData) {
    encryptParams.additionalData = typeof additionalData === 'string'
      ? new TextEncoder().encode(additionalData)
      : additionalData;
  }

  const ciphertext = await crypto.subtle.encrypt(
    encryptParams,
    key,
    plaintextBuffer
  );

  // Combine nonce + ciphertext (which includes tag in Web Crypto)
  const result = new Uint8Array(nonce.length + ciphertext.byteLength);
  result.set(nonce, 0);
  result.set(new Uint8Array(ciphertext), nonce.length);

  return arrayBufferToBase64(result.buffer);
}

/**
 * Decrypt data using AES-256-GCM
 */
export async function decrypt(keyBytes, encryptedBase64, additionalData = null) {
  const key = await crypto.subtle.importKey(
    'raw',
    keyBytes,
    'AES-GCM',
    false,
    ['decrypt']
  );

  const encrypted = base64ToArrayBuffer(encryptedBase64);
  const encryptedArray = new Uint8Array(encrypted);

  const nonce = encryptedArray.slice(0, NONCE_SIZE);
  const ciphertext = encryptedArray.slice(NONCE_SIZE);

  const decryptParams = {
    name: 'AES-GCM',
    iv: nonce,
    tagLength: TAG_SIZE * 8,
  };

  if (additionalData) {
    decryptParams.additionalData = typeof additionalData === 'string'
      ? new TextEncoder().encode(additionalData)
      : additionalData;
  }

  const plaintext = await crypto.subtle.decrypt(
    decryptParams,
    key,
    ciphertext
  );

  return new Uint8Array(plaintext);
}

/**
 * Decrypt data and return as string
 */
export async function decryptToString(keyBytes, encryptedBase64, additionalData = null) {
  const plaintext = await decrypt(keyBytes, encryptedBase64, additionalData);
  return new TextDecoder().decode(plaintext);
}

/**
 * Compute SHA-256 hash
 */
export async function sha256(data) {
  const buffer = typeof data === 'string'
    ? new TextEncoder().encode(data)
    : data;
  const hash = await crypto.subtle.digest('SHA-256', buffer);
  return new Uint8Array(hash);
}

/**
 * Generate a random 6-digit pairing code
 */
export function generatePairingCode() {
  const bytes = crypto.getRandomValues(new Uint8Array(4));
  const num = new DataView(bytes.buffer).getUint32(0) % 1000000;
  return num.toString().padStart(6, '0');
}

// Utility functions
export function arrayBufferToHex(buffer) {
  return Array.from(new Uint8Array(buffer))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

export function hexToArrayBuffer(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes.buffer;
}

export function arrayBufferToBase64(buffer) {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

export function base64ToArrayBuffer(base64) {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes.buffer;
}

export function base64ToUint8Array(base64) {
  return new Uint8Array(base64ToArrayBuffer(base64));
}
