# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take security seriously at Toss. If you discover a security vulnerability, please report it responsibly.

### How to Report

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please report security vulnerabilities by emailing:

**security@example.com** (replace with actual security contact)

Or use GitHub's private vulnerability reporting feature:
1. Go to the Security tab of this repository
2. Click "Report a vulnerability"
3. Fill out the form with details

### What to Include

Please include as much of the following information as possible:

- Type of vulnerability (e.g., buffer overflow, SQL injection, XSS)
- Full paths of source file(s) related to the vulnerability
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the vulnerability

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Resolution Target**: Within 90 days (depending on severity)

### What to Expect

1. **Acknowledgment**: We'll confirm receipt of your report
2. **Assessment**: We'll assess the vulnerability and determine its severity
3. **Updates**: We'll keep you informed of our progress
4. **Fix**: We'll develop and test a fix
5. **Disclosure**: We'll coordinate with you on public disclosure timing
6. **Credit**: With your permission, we'll credit you in the security advisory

## Security Measures in Toss

### Cryptographic Primitives

| Purpose | Algorithm | Notes |
|---------|-----------|-------|
| Key Exchange | X25519 | Elliptic curve Diffie-Hellman |
| Encryption | AES-256-GCM | Authenticated encryption |
| Signatures | Ed25519 | Device identity |
| Key Derivation | HKDF-SHA256 | Session key derivation |

### Security Features

- **End-to-End Encryption**: All clipboard data is encrypted before transmission
- **Zero-Knowledge Relay**: Relay servers only see encrypted blobs
- **Forward Secrecy**: Session keys are rotated regularly
- **Secure Key Storage**: Platform secure storage (Keychain, Credential Manager, etc.)
- **Certificate Pinning**: For relay server connections

### Security Best Practices

When using Toss:

1. **Verify Device Pairing**: Always verify the 6-digit code matches on both devices
2. **Use Latest Version**: Keep Toss updated to receive security patches
3. **Secure Your Devices**: Toss is only as secure as the devices it runs on
4. **Review Paired Devices**: Periodically review and remove unused devices

## Security Audits

We welcome security audits and penetration testing. If you're interested in conducting a security audit, please contact us first.

## Acknowledgments

We thank the following security researchers for responsibly disclosing vulnerabilities:

- (None yet - be the first!)
