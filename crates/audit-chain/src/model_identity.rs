#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ModelIdentity {
    pub name: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantization: Option<String>,
    pub hash_method: ModelHashMethod,
    pub hash: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModelHashMethod {
    WeightFile,
    StructuredDescriptor,
}

impl ModelIdentity {
    pub fn is_zero_hash(&self) -> bool {
        self.hash == [0u8; 32]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_model() -> ModelIdentity {
        ModelIdentity {
            name: "test-model".into(),
            provider: "test-provider".into(),
            version: Some("v1.0".into()),
            quantization: Some("fp16".into()),
            hash_method: ModelHashMethod::StructuredDescriptor,
            hash: [42u8; 32],
        }
    }

    #[test]
    fn serde_round_trip() {
        let model = sample_model();
        let json = serde_json::to_string(&model).unwrap();
        let back: ModelIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(back, model);
    }

    #[test]
    fn weight_file_hash_method_serde() {
        let model = ModelIdentity {
            hash_method: ModelHashMethod::WeightFile,
            ..sample_model()
        };
        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("WeightFile"));
        let back: ModelIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(back.hash_method, ModelHashMethod::WeightFile);
    }

    #[test]
    fn structured_descriptor_hash_method_serde() {
        let model = sample_model();
        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("StructuredDescriptor"));
    }

    #[test]
    fn zero_hash_detected() {
        let model = ModelIdentity {
            hash: [0u8; 32],
            ..sample_model()
        };
        assert!(model.is_zero_hash());
    }

    #[test]
    fn nonzero_hash_not_detected_as_zero() {
        let model = sample_model();
        assert!(!model.is_zero_hash());
    }

    #[test]
    fn optional_fields_omitted_when_none() {
        let model = ModelIdentity {
            version: None,
            quantization: None,
            ..sample_model()
        };
        let json = serde_json::to_string(&model).unwrap();
        assert!(!json.contains("version"));
        assert!(!json.contains("quantization"));
    }

    #[test]
    fn canonical_encoding_deterministic() {
        let m1 = sample_model();
        let m2 = sample_model();
        let b1 = crate::canonical::encode(&m1).unwrap();
        let b2 = crate::canonical::encode(&m2).unwrap();
        assert_eq!(b1, b2);
    }
}
