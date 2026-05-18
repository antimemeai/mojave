use std::fmt;
use std::path::PathBuf;

use ed25519_dalek::{SigningKey, VerifyingKey};
use signature::Signer;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum SigningAlgorithm {
    Ed25519,
}

#[derive(Clone)]
pub struct SignerKeyId(String);

impl SignerKeyId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Display for SignerKeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for SignerKeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SignerKeyId(\"{}\")", self.0)
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SignerError {
    #[error("signing failed: {0}")]
    SigningFailed(String),
    #[error("public key encoding error: {0}")]
    PublicKeyEncoding(String),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum KeyLoadError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("environment variable not set: {var}")]
    EnvNotSet { var: String },
    #[error("environment variable not UTF-8: {var}")]
    EnvNotUtf8 { var: String },
    #[error("invalid Ed25519 PKCS#8 key: {0}")]
    InvalidEd25519Pkcs8(String),
}

#[non_exhaustive]
pub enum KeyRef {
    InMemoryPkcs8 { key_id: SignerKeyId, der: Vec<u8> },
    FilePath { key_id: SignerKeyId, path: PathBuf },
    Env { key_id: SignerKeyId, var: String },
}

impl KeyRef {
    pub fn key_id(&self) -> &SignerKeyId {
        match self {
            KeyRef::InMemoryPkcs8 { key_id, .. } => key_id,
            KeyRef::FilePath { key_id, .. } => key_id,
            KeyRef::Env { key_id, .. } => key_id,
        }
    }

    pub fn load(self) -> Result<LocalEd25519Signer, KeyLoadError> {
        match self {
            KeyRef::InMemoryPkcs8 { key_id, der } => {
                let signing_key = parse_pkcs8_der(&der)?;
                Ok(LocalEd25519Signer {
                    key_id,
                    signing_key,
                })
            }
            KeyRef::FilePath { key_id, path } => {
                let bytes = std::fs::read(&path)?;
                let signing_key = if is_pem(&bytes) {
                    let pem_str = String::from_utf8(bytes).map_err(|e| {
                        KeyLoadError::InvalidEd25519Pkcs8(format!("PEM not UTF-8: {e}"))
                    })?;
                    parse_pkcs8_pem(&pem_str)?
                } else {
                    parse_pkcs8_der(&bytes)?
                };
                Ok(LocalEd25519Signer {
                    key_id,
                    signing_key,
                })
            }
            KeyRef::Env { key_id, var } => {
                let val = std::env::var(&var).map_err(|e| match e {
                    std::env::VarError::NotPresent => KeyLoadError::EnvNotSet { var: var.clone() },
                    std::env::VarError::NotUnicode(_) => KeyLoadError::EnvNotUtf8 { var },
                })?;
                let signing_key = if val.starts_with("-----BEGIN") {
                    parse_pkcs8_pem(&val)?
                } else {
                    parse_pkcs8_der(val.as_bytes())?
                };
                Ok(LocalEd25519Signer {
                    key_id,
                    signing_key,
                })
            }
        }
    }
}

fn is_pem(bytes: &[u8]) -> bool {
    bytes.windows(11).any(|w| w == b"-----BEGIN ")
}

fn parse_pkcs8_der(der: &[u8]) -> Result<SigningKey, KeyLoadError> {
    use ed25519_dalek::pkcs8::DecodePrivateKey;
    SigningKey::from_pkcs8_der(der).map_err(|e| KeyLoadError::InvalidEd25519Pkcs8(e.to_string()))
}

fn parse_pkcs8_pem(pem: &str) -> Result<SigningKey, KeyLoadError> {
    use ed25519_dalek::pkcs8::DecodePrivateKey;
    SigningKey::from_pkcs8_pem(pem).map_err(|e| KeyLoadError::InvalidEd25519Pkcs8(e.to_string()))
}

pub trait AuditSigner: Send + Sync {
    fn key_id(&self) -> &SignerKeyId;
    fn algorithm(&self) -> SigningAlgorithm;
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SignerError>;
    fn verifying_key_spki_der(&self) -> Result<Vec<u8>, SignerError>;
}

pub struct LocalEd25519Signer {
    key_id: SignerKeyId,
    signing_key: SigningKey,
}

impl fmt::Debug for LocalEd25519Signer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalEd25519Signer")
            .field("key_id", &self.key_id)
            .field("signing_key", &"<redacted>")
            .finish()
    }
}

impl LocalEd25519Signer {
    pub fn from_signing_key(key_id: SignerKeyId, signing_key: SigningKey) -> Self {
        Self {
            key_id,
            signing_key,
        }
    }

    pub fn generate(key_id: SignerKeyId) -> Self {
        let signing_key = SigningKey::generate(&mut rand_core::OsRng);
        Self {
            key_id,
            signing_key,
        }
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn to_pkcs8_der(&self) -> Result<Vec<u8>, SignerError> {
        use ed25519_dalek::pkcs8::EncodePrivateKey;
        self.signing_key
            .to_pkcs8_der()
            .map(|der| der.as_bytes().to_vec())
            .map_err(|e| SignerError::PublicKeyEncoding(e.to_string()))
    }
}

impl AuditSigner for LocalEd25519Signer {
    fn key_id(&self) -> &SignerKeyId {
        &self.key_id
    }

    fn algorithm(&self) -> SigningAlgorithm {
        SigningAlgorithm::Ed25519
    }

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SignerError> {
        let sig = self
            .signing_key
            .try_sign(message)
            .map_err(|e| SignerError::SigningFailed(e.to_string()))?;
        Ok(sig.to_bytes().to_vec())
    }

    fn verifying_key_spki_der(&self) -> Result<Vec<u8>, SignerError> {
        use ed25519_dalek::pkcs8::EncodePublicKey;
        self.signing_key
            .verifying_key()
            .to_public_key_der()
            .map(|der| der.as_ref().to_vec())
            .map_err(|e| SignerError::PublicKeyEncoding(e.to_string()))
    }
}

pub fn verifying_key_from_spki_der(spki_der: &[u8]) -> Result<VerifyingKey, KeyLoadError> {
    use ed25519_dalek::pkcs8::DecodePublicKey;
    VerifyingKey::from_public_key_der(spki_der)
        .map_err(|e| KeyLoadError::InvalidEd25519Pkcs8(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier;

    #[test]
    fn signer_key_id_round_trips() {
        let kid = SignerKeyId::new("my-key");
        assert_eq!(kid.as_str(), "my-key");
        assert_eq!(kid.as_bytes(), b"my-key");
        assert_eq!(format!("{kid}"), "my-key");
    }

    #[test]
    fn generate_and_sign() {
        let signer = LocalEd25519Signer::generate(SignerKeyId::new("test"));
        let msg = b"hello world";
        let sig_bytes = signer.sign(msg).unwrap();
        assert_eq!(sig_bytes.len(), 64);

        let sig = ed25519_dalek::Signature::from_bytes(sig_bytes.as_slice().try_into().unwrap());
        signer.verifying_key().verify(msg, &sig).unwrap();
    }

    #[test]
    fn spki_der_round_trip() {
        let signer = LocalEd25519Signer::generate(SignerKeyId::new("test"));
        let spki = signer.verifying_key_spki_der().unwrap();
        let recovered = verifying_key_from_spki_der(&spki).unwrap();
        assert_eq!(recovered, signer.verifying_key());
    }

    #[test]
    fn pkcs8_der_round_trip() {
        let signer = LocalEd25519Signer::generate(SignerKeyId::new("test"));
        let der = signer.to_pkcs8_der().unwrap();
        let key_ref = KeyRef::InMemoryPkcs8 {
            key_id: SignerKeyId::new("test"),
            der,
        };
        let loaded = key_ref.load().unwrap();
        assert_eq!(loaded.verifying_key(), signer.verifying_key());
    }

    #[test]
    fn file_path_loads_pem() {
        use ed25519_dalek::pkcs8::EncodePrivateKey;
        use std::io::Write;

        let signer = LocalEd25519Signer::generate(SignerKeyId::new("test"));
        let pem = signer
            .signing_key
            .to_pkcs8_pem(ed25519_dalek::pkcs8::spki::der::pem::LineEnding::LF)
            .unwrap();

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "{}", pem.as_str()).unwrap();

        let key_ref = KeyRef::FilePath {
            key_id: SignerKeyId::new("test"),
            path: tmp.path().to_path_buf(),
        };
        let loaded = key_ref.load().unwrap();
        assert_eq!(loaded.verifying_key(), signer.verifying_key());
    }

    #[test]
    fn file_path_rejects_garbage() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut tmp, b"not a key").unwrap();

        let key_ref = KeyRef::FilePath {
            key_id: SignerKeyId::new("test"),
            path: tmp.path().to_path_buf(),
        };
        assert!(matches!(
            key_ref.load(),
            Err(KeyLoadError::InvalidEd25519Pkcs8(_))
        ));
    }

    #[test]
    fn spki_der_rejects_garbage() {
        assert!(matches!(
            verifying_key_from_spki_der(b"garbage"),
            Err(KeyLoadError::InvalidEd25519Pkcs8(_))
        ));
    }

    #[test]
    fn algorithm_is_ed25519() {
        let signer = LocalEd25519Signer::generate(SignerKeyId::new("test"));
        assert!(matches!(signer.algorithm(), SigningAlgorithm::Ed25519));
    }

    #[test]
    fn debug_redacts_key() {
        let signer = LocalEd25519Signer::generate(SignerKeyId::new("test"));
        let debug = format!("{signer:?}");
        assert!(debug.contains("<redacted>"));
    }
}
