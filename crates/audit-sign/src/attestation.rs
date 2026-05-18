use chrono::Utc;
use coset::{CborSerializable, CoseSign1, CoseSign1Builder, HeaderBuilder, Label};

use crate::signing::{AuditSigner, SignerError, SigningAlgorithm};
use crate::snapshot::ChainHeadSnapshot;

const CONTENT_TYPE_LABEL: i64 = 999;
const CONTENT_TYPE_VALUE: &str = "application/vnd.mojave.audit.chain-head+json";
const SIGNED_AT_LABEL: i64 = 1000;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AttestationBuildError {
    #[error("signing failed: {0}")]
    Signing(#[from] SignerError),
    #[error("canonical encoding failed: {0}")]
    CanonicalEncoding(#[from] audit_chain::canonical::CanonicalEncodingError),
    #[error("CBOR serialization failed: {0}")]
    Cbor(String),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AttestationVerifyError {
    #[error("signature is invalid")]
    SignatureInvalid,
    #[error("missing kid header")]
    MissingKid,
    #[error("content type mismatch")]
    ContentTypeMismatch,
    #[error("payload must be detached (external)")]
    PayloadNotDetached,
    #[error("unprotected headers must be empty")]
    NonEmptyUnprotectedHeader,
    #[error("unknown key id")]
    UnknownKeyId,
    #[error("CBOR deserialization failed: {0}")]
    Cbor(String),
}

fn cose_alg(algo: SigningAlgorithm) -> coset::iana::Algorithm {
    match algo {
        SigningAlgorithm::Ed25519 => coset::iana::Algorithm::EdDSA,
    }
}

pub fn build_detached_attestation(
    signer: &dyn AuditSigner,
    payload: &[u8],
) -> Result<Vec<u8>, AttestationBuildError> {
    let signed_at = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let protected = HeaderBuilder::new()
        .algorithm(cose_alg(signer.algorithm()))
        .key_id(signer.key_id().as_bytes().to_vec())
        .value(
            CONTENT_TYPE_LABEL,
            ciborium::Value::Text(CONTENT_TYPE_VALUE.to_string()),
        )
        .value(SIGNED_AT_LABEL, ciborium::Value::Text(signed_at))
        .build();

    let sign1 = CoseSign1Builder::new()
        .protected(protected)
        .payload(Vec::new())
        .try_create_signature(payload, |tbs| signer.sign(tbs))
        .map_err(AttestationBuildError::Signing)?
        .build();

    sign1
        .to_vec()
        .map_err(|e| AttestationBuildError::Cbor(format!("{e:?}")))
}

pub fn build_tip_attestation(
    signer: &dyn AuditSigner,
    snapshot: &ChainHeadSnapshot,
) -> Result<Vec<u8>, AttestationBuildError> {
    let payload = snapshot.canonical_bytes()?;
    build_detached_attestation(signer, &payload)
}

pub fn verify_detached_attestation(
    cbor_bytes: &[u8],
    payload: &[u8],
    keyring: &std::collections::HashMap<Vec<u8>, ed25519_dalek::VerifyingKey>,
) -> Result<(), AttestationVerifyError> {
    let envelope = CoseSign1::from_slice(cbor_bytes)
        .map_err(|e| AttestationVerifyError::Cbor(format!("{e:?}")))?;

    if envelope.payload.as_ref().is_some_and(|p| !p.is_empty()) {
        return Err(AttestationVerifyError::PayloadNotDetached);
    }

    if !envelope.unprotected.rest.is_empty()
        || !envelope.unprotected.key_id.is_empty()
        || envelope.unprotected.content_type.is_some()
    {
        return Err(AttestationVerifyError::NonEmptyUnprotectedHeader);
    }

    let protected = &envelope.protected.header;

    if protected.key_id.is_empty() {
        return Err(AttestationVerifyError::MissingKid);
    }
    let kid = &protected.key_id;

    let has_ct = protected.rest.iter().any(|(label, val)| {
        *label == Label::Int(CONTENT_TYPE_LABEL)
            && matches!(val, ciborium::Value::Text(s) if s == CONTENT_TYPE_VALUE)
    });
    if !has_ct {
        return Err(AttestationVerifyError::ContentTypeMismatch);
    }

    let vk = keyring
        .get(kid.as_slice())
        .ok_or(AttestationVerifyError::UnknownKeyId)?;

    let tbs = envelope.tbs_data(payload);

    let sig_bytes: &[u8] = &envelope.signature;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| AttestationVerifyError::SignatureInvalid)?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);

    use ed25519_dalek::Verifier;
    vk.verify(&tbs, &sig)
        .map_err(|_| AttestationVerifyError::SignatureInvalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::{LocalEd25519Signer, SignerKeyId};
    use std::collections::HashMap;

    fn test_signer() -> LocalEd25519Signer {
        LocalEd25519Signer::generate(SignerKeyId::new("test-key"))
    }

    fn test_keyring(signer: &LocalEd25519Signer) -> HashMap<Vec<u8>, ed25519_dalek::VerifyingKey> {
        let mut keyring = HashMap::new();
        keyring.insert(signer.key_id().as_bytes().to_vec(), signer.verifying_key());
        keyring
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let signer = test_signer();
        let payload = b"hello audit chain";
        let cbor = build_detached_attestation(&signer, payload).unwrap();
        let keyring = test_keyring(&signer);
        verify_detached_attestation(&cbor, payload, &keyring).unwrap();
    }

    #[test]
    fn tampered_payload_rejected() {
        let signer = test_signer();
        let payload = b"original";
        let cbor = build_detached_attestation(&signer, payload).unwrap();
        let keyring = test_keyring(&signer);
        let result = verify_detached_attestation(&cbor, b"tampered", &keyring);
        assert!(matches!(
            result,
            Err(AttestationVerifyError::SignatureInvalid)
        ));
    }

    #[test]
    fn unknown_key_id_rejected() {
        let signer = test_signer();
        let payload = b"hello";
        let cbor = build_detached_attestation(&signer, payload).unwrap();
        let keyring = HashMap::new();
        let result = verify_detached_attestation(&cbor, payload, &keyring);
        assert!(matches!(result, Err(AttestationVerifyError::UnknownKeyId)));
    }

    #[test]
    fn bad_cbor_rejected() {
        let keyring = HashMap::new();
        let result = verify_detached_attestation(b"not cbor", b"", &keyring);
        assert!(matches!(result, Err(AttestationVerifyError::Cbor(_))));
    }

    #[test]
    fn tip_attestation_round_trip() {
        use audit_chain::entry::{Action, AuditEntryBuilder, Decision, Principal};
        use audit_chain::seal::ChainHead;
        use chrono::{TimeZone, Utc};

        let mut head = ChainHead::new();
        let entry = AuditEntryBuilder::new()
            .seq(0)
            .actor(Principal::System { id: "test".into() })
            .action(Action::Observed)
            .decision(Decision::Observed)
            .at(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap())
            .context(serde_json::json!({}))
            .build()
            .unwrap();
        head.link(entry).unwrap();

        let snapshot = ChainHeadSnapshot::from_chain_head(&head);
        let signer = test_signer();
        let cbor = build_tip_attestation(&signer, &snapshot).unwrap();

        let keyring = test_keyring(&signer);
        let payload = snapshot.canonical_bytes().unwrap();
        verify_detached_attestation(&cbor, &payload, &keyring).unwrap();
    }
}
