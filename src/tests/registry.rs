use crate::*;
use ed25519_dalek::SigningKey;

#[test]
fn registry_signatures_are_content_bound() {
    let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
    let trusted_public_key = encode_hex(signing_key.verifying_key().as_bytes());
    let signature = sign_registry(b"catalog", "fixture", &"07".repeat(32)).unwrap();
    verify_registry_signature(b"catalog", &signature, "fixture", &trusted_public_key).unwrap();
    assert!(
        verify_registry_signature(b"tampered", &signature, "fixture", &trusted_public_key).is_err()
    );
    let attacker_key = encode_hex(
        SigningKey::from_bytes(&[8_u8; 32])
            .verifying_key()
            .as_bytes(),
    );
    assert!(verify_registry_signature(b"catalog", &signature, "fixture", &attacker_key).is_err());
    assert!(
        verify_registry_signature(b"catalog", &signature, "attacker", &trusted_public_key).is_err()
    );
}
