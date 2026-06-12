# Key Storage and Rotation Policy

## Overview

This document defines the key storage and rotation strategy for Zeus cryptographic proof certificates (.zcert). Certificates are signed with Ed25519 keys to provide cryptographic proof of verification properties.

## Key Types

### Certificate Signing Keys
- **Algorithm**: Ed25519 (ed25519-dalek crate)
- **Purpose**: Signing .zcert certificates for verified code
- **Key Size**: 256-bit private key, 256-bit public key
- **Security Properties**: Resistance to timing attacks, side-channel attacks

### Key Locations

#### Development Environment
- Private keys stored in `~/.zeus/keys/` directory
- Keys encrypted at rest using AES-256-GCM
- Key encryption key derived from user password via Argon2id
- Public keys stored in `~/.zeus/keys/public/` (unencrypted)

#### Production Environment
- Private keys stored in HashiCorp Vault or AWS Secrets Manager
- Keys encrypted at rest by the secrets manager
- Access controlled via IAM policies / Vault policies
- Public keys stored in public certificate registry

## Key Generation

### Initial Key Generation
```bash
# Generate new Ed25519 key pair
zeus key generate --name production_signing_key

# Keys are:
# - Private: encrypted and stored in secrets manager
# - Public: uploaded to certificate registry
```

### Key Properties
- **Entropy**: Generated from cryptographically secure RNG (getrandom crate)
- **Format**: PKCS#8 for storage, raw bytes for signing
- **Backup**: Encrypted backup stored in separate geographic region

## Key Rotation Strategy

### Rotation Triggers
1. **Time-based**: Rotate keys every 90 days
2. **Compromise**: Immediate rotation if key compromise suspected
3. **Algorithm upgrade**: Rotate when cryptographic standards change
4. **Personnel change**: Rotate when key custodian leaves organization

### Rotation Process

#### Pre-Rotation
1. Generate new key pair
2. Store new private key in secrets manager
3. Upload new public key to certificate registry
4. Mark new key as "pending" in key registry

#### Transition Period (7 days)
1. Dual-sign certificates with both old and new keys
2. Verify signature verification works with both keys
3. Monitor for signature verification failures
4. Update client trust stores with new public key

#### Post-Rotation
1. Mark old key as "deprecated" in key registry
2. Stop signing with old key
3. Archive old private key (retain for 1 year)
4. Remove old public key from client trust stores

### Rollback Procedure
If new key fails:
1. Immediately revert to old key for signing
2. Investigate failure cause
3. Generate new replacement key
4. Restart rotation process

## Key Access Control

### Development
- Single user access to personal keys
- Keys encrypted with user password
- No network access to keys

### Production
- Principle of least privilege
- Access via IAM roles / Vault tokens
- Audit logging for all key access
- MFA required for key operations
- Emergency access via break-glass procedure

### Access Levels
1. **Read-Only**: Can retrieve public keys
2. **Sign**: Can use private key for signing (no export)
3. **Admin**: Can generate/rotate keys, manage access

## Key Backup and Recovery

### Backup Strategy
- Primary: Secrets manager (automatic replication)
- Secondary: Encrypted offline backup (air-gapped)
- Tertiary: Key recovery service (shamir secret sharing)

### Recovery Process
1. Verify key compromise status
2. Recover from primary backup if available
3. Use secondary backup if primary unavailable
4. Use shamir shares if both backups unavailable
5. Generate new key pair if all recovery fails

## Key Compromise Response

### Detection
- Monitor certificate signature verification failures
- Monitor for unexpected certificate issuance
- Monitor secrets manager access logs
- Monitor for public key leaks (certificate transparency logs)

### Response Steps
1. **Immediate**: Revoke compromised key
2. **Within 1 hour**: Generate new key pair
3. **Within 4 hours**: Update all systems with new key
4. **Within 24 hours**: Notify all certificate users
5. **Within 7 days**: Complete post-mortem

### Certificate Revocation
- Maintain certificate revocation list (CRL)
- Publish CRL to public registry
- Implement OCSP for real-time revocation checks
- Set maximum certificate validity to 90 days

## Key Lifecycle Management

### Key States
1. **Generating**: Key pair being created
2. **Active**: Key in use for signing
3. **Pending**: Key generated, awaiting activation
4. **Deprecated**: Key phased out, still valid for verification
5. **Revoked**: Key compromised, no longer valid
6. **Archived**: Key retained for historical purposes

### State Transitions
```
Generating → Active → Deprecated → Archived
                ↓
              Revoked
```

## Implementation Requirements

### Compiler Integration
- Load signing key from secrets manager at startup
- Cache key in memory (never write to disk)
- Rotate in-memory key when rotation detected
- Fall back to backup key if primary unavailable

### Cloud API Integration
- Use secrets manager SDK for key access
- Implement key caching with TTL (5 minutes)
- Handle key rotation gracefully
- Log all key access operations

### CLI Integration
- Support multiple key profiles (dev, staging, prod)
- Prompt for password for local key decryption
- Validate key integrity before use
- Support key import/export for backup

## Compliance and Auditing

### Audit Requirements
- Log all key generation events
- Log all key access events
- Log all key rotation events
- Log all certificate signing events
- Retain logs for minimum 1 year

### Compliance Standards
- **SOC 2**: Key access controls and logging
- **ISO 27001**: Key management procedures
- **NIST SP 800-57**: Key length and rotation guidance
- **FIPS 140-2**: Cryptographic module validation

## Security Best Practices

1. **Never** commit private keys to version control
2. **Never** log private keys or their derivatives
3. **Always** encrypt keys at rest
4. **Always** use hardware security modules (HSM) in production
5. **Always** implement principle of least privilege
6. **Always** rotate keys before they are compromised
7. **Always** test key rotation procedures
8. **Always** have a backup and recovery plan

## References

- [NIST SP 800-57 Part 1 Rev. 5](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-57pt1r5.pdf)
- [Ed25519: High-speed high-security signatures](https://ed25519.cr.yp.to/)
- [HashiCorp Vault Documentation](https://www.vaultproject.io/docs)
- [AWS KMS Documentation](https://docs.aws.amazon.com/kms/)

## Version History

- v1.0 (2024-01-15): Initial policy definition
