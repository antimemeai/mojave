# Genesis Sentinel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the all-zeros genesis sentinel in the audit chain with the model's identity hash, so every entry transitively depends on the evaluated model.

**Architecture:** `SealedAuditEntry` becomes a two-variant enum (Genesis | Chained). `ChainHead::new()` requires a `ModelIdentity` and produces the genesis entry immediately. The model hash occupies the same 32-byte slot where parent_hash lives for chained entries — every subsequent entry transitively depends on it through the hash chain. One chain per model.

**Tech Stack:** Rust (audit-chain, audit-events, audit-sign, audit-recover, audit-emit, mojave-cli crates), Python (pipeline scripts), SHA-256 via `sha2` crate, serde with internally-tagged enum.

**Spec:** `docs/specs/2026-05-25-genesis-sentinel-design.md`

---

### Task 1: TCK — Genesis Sentinel Scenarios

**Files:**
- Modify: `tck/audit-chain/features/chain_integrity.feature`

- [ ] **Step 1: Replace the Gherkin feature file with genesis-aware scenarios**

```gherkin
Feature: Hash chain integrity for sealed audit entries
  SHA-256 hash chain linking sealed audit entries via parent_hash → entry_hash.
  ChainHead requires ModelIdentity to produce a genesis entry.
  ChainVerifier detects tampering and structural violations.

  Background:
    Given a ModelIdentity with hash_method "StructuredDescriptor" and hash [42; 32]

  # --- Genesis construction ---

  Scenario: Genesis entry is created by ChainHead::new
    When I create a new ChainHead with the ModelIdentity
    Then the returned genesis entry has seq 0
    And the genesis entry has event "chain.genesis"
    And the genesis entry has no parent_hash
    And the genesis entry contains the ModelIdentity
    And the entry_hash is 32 bytes

  Scenario: Genesis hash uses model hash as sentinel
    When I create a new ChainHead with the ModelIdentity
    Then the genesis entry_hash equals SHA-256(domain_tag || canonical(base) || model_hash)

  Scenario: Different model hash produces different genesis hash
    Given a second ModelIdentity with hash [43; 32]
    When I create two ChainHeads with different ModelIdentities
    Then the two genesis entry_hashes differ

  Scenario: Zero model hash is rejected
    Given a ModelIdentity with hash [0; 32]
    When I attempt to create a ChainHead
    Then it fails with ZeroModelHash error

  # --- Chained entries ---

  Scenario: Second entry chains to genesis
    When I create a new ChainHead with the ModelIdentity
    And I link one AuditEntry
    Then the chained entry has seq 1
    And the chained entry's parent_hash equals the genesis entry's entry_hash

  Scenario: Chained entry has required parent_hash
    When I create a new ChainHead with the ModelIdentity
    And I link one AuditEntry
    Then the chained entry's parent_hash is not None

  # --- Verification ---

  Scenario: Clean chain verifies clean
    When I create a chain with genesis and 3 chained entries
    And I verify the chain
    Then the result is clean

  Scenario: Empty chain produces MissingGenesis finding
    When I verify an empty chain
    Then the result has a MissingGenesis finding

  Scenario: Chained entry at index zero detected
    When I create a chain and replace genesis with a forged Chained entry
    And I verify the chain
    Then the result has a ChainedAtIndexZero finding

  Scenario: Genesis hash mismatch detected
    When I create a chain and tamper with the genesis model_identity hash
    And I verify the chain
    Then the result has a GenesisHashMismatch finding

  Scenario: Duplicate genesis detected
    When I create a chain and insert a second genesis at index 2
    And I verify the chain
    Then the result has a DuplicateGenesis finding at index 2

  Scenario: Tampered entry body detected as entry hash mismatch
    When I create a chain with genesis and 3 chained entries
    And I tamper with the detail of the chained entry at index 2
    And I verify the chain
    Then the result has an EntryHashMismatch finding at seq 2

  Scenario: Tampered parent hash detected
    When I create a chain with genesis and 3 chained entries
    And I overwrite the parent_hash of the chained entry at index 2 with zeros
    And I verify the chain
    Then the result has a ParentHashMismatch finding at seq 2

  Scenario: Sequence discontinuity detected
    When I create a chain with genesis and 3 chained entries
    And I set the seq of the chained entry at index 2 to 99
    And I verify the chain
    Then the result has a SeqDiscontinuity finding at index 2

  # --- Determinism ---

  Scenario: Resumed chain continues from prior head
    Given a ChainHead resumed with a known entry_hash and next_seq 10
    When I link one AuditEntry
    Then the sealed entry has seq 10
    And the sealed entry's parent_hash equals the known entry_hash

  Scenario: entry_hash is deterministic
    Given two identical AuditEntry values
    When I create two ChainHeads with the same ModelIdentity and timestamp
    And I compute the genesis entry_hash for both
    Then the hashes are identical

  Scenario: Single bit flip in detail changes entry_hash
    Given two AuditEntry values differing by one bit in detail
    When I link both to identical chain heads
    Then the entry_hashes differ

  Scenario: Model identity accessor returns model from genesis
    When I create a chain with genesis and 2 chained entries
    Then ChainVerifier::model_identity returns the original ModelIdentity
```

- [ ] **Step 2: Commit**

```bash
git add tck/audit-chain/features/chain_integrity.feature
git commit -m "tck(audit-chain): genesis sentinel scenarios (red)"
```

---

### Task 2: ModelIdentity Type

**Files:**
- Create: `crates/audit-chain/src/model_identity.rs`
- Modify: `crates/audit-chain/src/lib.rs`

- [ ] **Step 1: Write the test file with ModelIdentity tests**

Add this at the bottom of a new file `crates/audit-chain/src/model_identity.rs`:

```rust
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
```

- [ ] **Step 2: Export the module from lib.rs**

In `crates/audit-chain/src/lib.rs`, add:

```rust
pub mod model_identity;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p audit-chain --lib model_identity`

Expected: All 7 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/audit-chain/src/model_identity.rs crates/audit-chain/src/lib.rs
git commit -m "feat(audit-chain): add ModelIdentity type"
```

---

### Task 3: SealedAuditEntry Enum + ChainHead + Hash Functions

This is the core transformation. `SealedAuditEntry` becomes an enum, `ChainHead::new()` requires `ModelIdentity`, hash functions split into genesis/chained variants.

**Files:**
- Modify: `crates/audit-chain/src/seal.rs` (complete rewrite)
- Modify: `crates/audit-chain/src/verify.rs` (mechanical accessor updates only — full rewrite in Task 4)

- [ ] **Step 1: Rewrite seal.rs with the new types, hash functions, and tests**

Replace the entire contents of `crates/audit-chain/src/seal.rs` with:

```rust
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::canonical::{self, CanonicalEncodingError};
use crate::entry::{AuditEntry, AuditEntryBuilder, Principal};
use crate::model_identity::ModelIdentity;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum SealedAuditEntry {
    Genesis {
        base: AuditEntry,
        model_identity: ModelIdentity,
        entry_hash: [u8; 32],
    },
    Chained {
        base: AuditEntry,
        parent_hash: [u8; 32],
        entry_hash: [u8; 32],
    },
}

impl SealedAuditEntry {
    pub fn base(&self) -> &AuditEntry {
        match self {
            Self::Genesis { base, .. } | Self::Chained { base, .. } => base,
        }
    }

    pub fn entry_hash(&self) -> [u8; 32] {
        match self {
            Self::Genesis { entry_hash, .. } | Self::Chained { entry_hash, .. } => *entry_hash,
        }
    }

    pub fn seq(&self) -> u64 {
        self.base().seq
    }

    pub fn is_genesis(&self) -> bool {
        matches!(self, Self::Genesis { .. })
    }

    pub fn parent_hash(&self) -> Option<[u8; 32]> {
        match self {
            Self::Genesis { .. } => None,
            Self::Chained { parent_hash, .. } => Some(*parent_hash),
        }
    }

    pub fn model_identity(&self) -> Option<&ModelIdentity> {
        match self {
            Self::Genesis {
                model_identity, ..
            } => Some(model_identity),
            Self::Chained { .. } => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SealError {
    #[error(transparent)]
    CanonicalEncoding(#[from] CanonicalEncodingError),
    #[error("sequence number exhausted (u64::MAX reached)")]
    SeqExhausted,
    #[error("model identity hash is all zeros")]
    ZeroModelHash,
    #[error("genesis entry construction failed")]
    GenesisConstruction,
}

#[derive(Debug)]
pub struct ChainHead {
    last_entry_hash: Option<[u8; 32]>,
    next_seq: u64,
}

impl ChainHead {
    pub fn new(
        model: ModelIdentity,
        at: DateTime<Utc>,
    ) -> Result<(Self, SealedAuditEntry), SealError> {
        if model.is_zero_hash() {
            return Err(SealError::ZeroModelHash);
        }

        let model_hash = model.hash;

        let detail =
            serde_json::to_value(&model).map_err(|_| SealError::GenesisConstruction)?;

        let genesis_base = AuditEntryBuilder::new()
            .seq(0)
            .actor(Principal {
                kind: "System".into(),
                id: "chain.init".into(),
            })
            .event("chain.genesis")
            .authorization("Allowed")
            .outcome("Succeeded")
            .at(at)
            .detail(detail)
            .build()
            .map_err(|_| SealError::GenesisConstruction)?;

        let entry_hash = compute_genesis_hash(&genesis_base, model_hash)?;

        let genesis = SealedAuditEntry::Genesis {
            base: genesis_base,
            model_identity: model,
            entry_hash,
        };

        let head = Self {
            last_entry_hash: Some(entry_hash),
            next_seq: 1,
        };

        Ok((head, genesis))
    }

    pub fn resume(last_entry_hash: [u8; 32], next_seq: u64) -> Self {
        Self {
            last_entry_hash: Some(last_entry_hash),
            next_seq,
        }
    }

    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    pub fn last_entry_hash(&self) -> Option<[u8; 32]> {
        self.last_entry_hash
    }

    pub fn link(&mut self, mut base: AuditEntry) -> Result<SealedAuditEntry, SealError> {
        if self.next_seq == u64::MAX {
            return Err(SealError::SeqExhausted);
        }

        base.seq = self.next_seq;
        let parent_hash = self
            .last_entry_hash
            .ok_or(SealError::GenesisConstruction)?;
        let entry_hash = compute_chained_hash(&base, parent_hash)?;

        self.last_entry_hash = Some(entry_hash);
        self.next_seq += 1;

        Ok(SealedAuditEntry::Chained {
            base,
            parent_hash,
            entry_hash,
        })
    }
}

const DOMAIN_TAG: &[u8] = b"mojave-audit-v1\x00";

pub(crate) fn compute_genesis_hash(
    base: &AuditEntry,
    model_hash: [u8; 32],
) -> Result<[u8; 32], CanonicalEncodingError> {
    let canonical = canonical::encode(base)?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_TAG);
    hasher.update(&canonical);
    hasher.update(model_hash);
    Ok(hasher.finalize().into())
}

pub(crate) fn compute_chained_hash(
    base: &AuditEntry,
    parent_hash: [u8; 32],
) -> Result<[u8; 32], CanonicalEncodingError> {
    let canonical = canonical::encode(base)?;
    let mut hasher = Sha256::new();
    hasher.update(DOMAIN_TAG);
    hasher.update(&canonical);
    hasher.update(parent_hash);
    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_identity::ModelHashMethod;
    use chrono::TimeZone;

    fn sample_model() -> ModelIdentity {
        ModelIdentity {
            name: "test-model".into(),
            provider: "test-provider".into(),
            version: None,
            quantization: None,
            hash_method: ModelHashMethod::StructuredDescriptor,
            hash: [42u8; 32],
        }
    }

    fn fixed_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn sample_entry() -> AuditEntry {
        AuditEntryBuilder::new()
            .seq(0)
            .actor(Principal {
                kind: "System".into(),
                id: "test".into(),
            })
            .event("eval.started")
            .authorization("Allowed")
            .outcome("Succeeded")
            .at(fixed_time())
            .detail(serde_json::json!({"trial": 1}))
            .build()
            .unwrap()
    }

    #[test]
    fn genesis_entry_created_by_new() {
        let (head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert!(genesis.is_genesis());
        assert_eq!(genesis.seq(), 0);
        assert_eq!(head.next_seq(), 1);
        assert!(head.last_entry_hash().is_some());
    }

    #[test]
    fn genesis_has_model_identity() {
        let model = sample_model();
        let (_, genesis) = ChainHead::new(model.clone(), fixed_time()).unwrap();
        let mi = genesis.model_identity().unwrap();
        assert_eq!(mi.name, "test-model");
        assert_eq!(mi.hash, model.hash);
    }

    #[test]
    fn genesis_has_no_parent_hash() {
        let (_, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert!(genesis.parent_hash().is_none());
    }

    #[test]
    fn genesis_event_is_chain_genesis() {
        let (_, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert_eq!(genesis.base().event, "chain.genesis");
    }

    #[test]
    fn genesis_hash_uses_model_hash() {
        let (_, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let recomputed =
            compute_genesis_hash(genesis.base(), sample_model().hash).unwrap();
        assert_eq!(recomputed, genesis.entry_hash());
    }

    #[test]
    fn different_model_hash_changes_genesis_hash() {
        let model_a = sample_model();
        let mut model_b = sample_model();
        model_b.hash = [43u8; 32];
        let (_, genesis_a) = ChainHead::new(model_a, fixed_time()).unwrap();
        let (_, genesis_b) = ChainHead::new(model_b, fixed_time()).unwrap();
        assert_ne!(genesis_a.entry_hash(), genesis_b.entry_hash());
    }

    #[test]
    fn zero_model_hash_rejected() {
        let model = ModelIdentity {
            hash: [0u8; 32],
            ..sample_model()
        };
        let result = ChainHead::new(model, fixed_time());
        assert!(matches!(result, Err(SealError::ZeroModelHash)));
    }

    #[test]
    fn second_entry_chains_to_genesis() {
        let (mut head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let second = head.link(sample_entry()).unwrap();
        assert!(!second.is_genesis());
        assert_eq!(second.seq(), 1);
        assert_eq!(second.parent_hash(), Some(genesis.entry_hash()));
    }

    #[test]
    fn chained_entry_has_parent_hash() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let chained = head.link(sample_entry()).unwrap();
        assert!(chained.parent_hash().is_some());
    }

    #[test]
    fn seq_advances_monotonically() {
        let (mut head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert_eq!(genesis.seq(), 0);
        for i in 1..5u64 {
            let sealed = head.link(sample_entry()).unwrap();
            assert_eq!(sealed.seq(), i);
        }
    }

    #[test]
    fn resumed_chain_continues() {
        let known_hash = [99u8; 32];
        let mut head = ChainHead::resume(known_hash, 10);
        let sealed = head.link(sample_entry()).unwrap();
        assert_eq!(sealed.seq(), 10);
        assert_eq!(sealed.parent_hash(), Some(known_hash));
    }

    #[test]
    fn entry_hash_is_deterministic() {
        let (_, g1) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let (_, g2) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert_eq!(g1.entry_hash(), g2.entry_hash());
    }

    #[test]
    fn different_parent_hash_changes_entry_hash() {
        let entry = sample_entry();
        let h1 = compute_chained_hash(&entry, [1u8; 32]).unwrap();
        let h2 = compute_chained_hash(&entry, [2u8; 32]).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn single_bit_detail_change_changes_hash() {
        let mut e1 = sample_entry();
        let mut e2 = sample_entry();
        e1.detail = serde_json::json!({"trial": 1});
        e2.detail = serde_json::json!({"trial": 2});
        let h1 = compute_chained_hash(&e1, [1u8; 32]).unwrap();
        let h2 = compute_chained_hash(&e2, [1u8; 32]).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn sealed_entry_serde_round_trip_genesis() {
        let (_, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let json = serde_json::to_string(&genesis).unwrap();
        assert!(json.contains(r#""type":"Genesis"#));
        let back: SealedAuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entry_hash(), genesis.entry_hash());
        assert!(back.is_genesis());
    }

    #[test]
    fn sealed_entry_serde_round_trip_chained() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let chained = head.link(sample_entry()).unwrap();
        let json = serde_json::to_string(&chained).unwrap();
        assert!(json.contains(r#""type":"Chained"#));
        let back: SealedAuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entry_hash(), chained.entry_hash());
        assert_eq!(back.parent_hash(), chained.parent_hash());
    }

    #[test]
    fn domain_tag_is_included_in_genesis_hash() {
        let (_, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let canonical = crate::canonical::encode(genesis.base()).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(b"mojave-audit-v1\x00");
        hasher.update(&canonical);
        hasher.update(sample_model().hash);
        let expected: [u8; 32] = hasher.finalize().into();
        assert_eq!(genesis.entry_hash(), expected);
    }

    #[test]
    fn next_seq_accessor() {
        let (head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert_eq!(head.next_seq(), 1);
    }

    #[test]
    fn last_entry_hash_accessor() {
        let (head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        assert_eq!(head.last_entry_hash(), Some(genesis.entry_hash()));
    }

    #[test]
    fn model_identity_returns_none_for_chained() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let chained = head.link(sample_entry()).unwrap();
        assert!(chained.model_identity().is_none());
    }
}
```

- [ ] **Step 2: Mechanically update verify.rs to compile with new accessor pattern**

In `crates/audit-chain/src/verify.rs`, apply these mechanical changes so the crate compiles. The full verifier rewrite happens in Task 4.

Change the import (line 2):
```rust
use crate::seal::{compute_genesis_hash, compute_chained_hash, SealedAuditEntry};
```

Replace the `verify` function body (lines 64-107) to use accessor methods and handle the enum. This is a temporary bridge — Task 4 rewrites it properly:

```rust
    pub fn verify(entries: &[SealedAuditEntry]) -> ChainFindings {
        let mut findings = Vec::new();

        for (i, entry) in entries.iter().enumerate() {
            if i == 0 && entry.parent_hash().is_some() {
                findings.push(ChainFinding::NonGenesisAtIndexZero);
            }

            let expected_seq = if i == 0 {
                entries.first().map_or(0, |e| e.seq())
            } else {
                entries[i - 1].seq() + 1
            };
            if i > 0 && entry.seq() != expected_seq {
                findings.push(ChainFinding::SeqDiscontinuity {
                    index: i,
                    expected: expected_seq,
                    actual: entry.seq(),
                });
            }

            if i > 0 {
                let expected_parent = entries[i - 1].entry_hash();
                if entry.parent_hash() != Some(expected_parent) {
                    findings.push(ChainFinding::ParentHashMismatch {
                        index: i,
                        seq: entry.seq(),
                    });
                }
            }

            let recomputed = match entry {
                SealedAuditEntry::Genesis {
                    base,
                    model_identity,
                    ..
                } => compute_genesis_hash(base, model_identity.hash),
                SealedAuditEntry::Chained {
                    base, parent_hash, ..
                } => compute_chained_hash(base, *parent_hash),
            };
            if let Ok(hash) = recomputed {
                if hash != entry.entry_hash() {
                    findings.push(ChainFinding::EntryHashMismatch {
                        index: i,
                        seq: entry.seq(),
                    });
                }
            }
        }

        ChainFindings { findings }
    }
```

Update `recompute_entry_hash` (lines 109-113):

```rust
    pub fn recompute_entry_hash(
        entry: &SealedAuditEntry,
    ) -> Result<[u8; 32], CanonicalEncodingError> {
        match entry {
            SealedAuditEntry::Genesis {
                base,
                model_identity,
                ..
            } => compute_genesis_hash(base, model_identity.hash),
            SealedAuditEntry::Chained {
                base, parent_hash, ..
            } => compute_chained_hash(base, *parent_hash),
        }
    }
```

Update the verify.rs tests — replace the `build_chain` helper and update all tests to use the new API:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{AuditEntryBuilder, Principal};
    use crate::model_identity::{ModelHashMethod, ModelIdentity};
    use crate::seal::ChainHead;
    use chrono::{TimeZone, Utc};

    fn sample_model() -> ModelIdentity {
        ModelIdentity {
            name: "test-model".into(),
            provider: "test-provider".into(),
            version: None,
            quantization: None,
            hash_method: ModelHashMethod::StructuredDescriptor,
            hash: [42u8; 32],
        }
    }

    fn fixed_time() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn sample_entry() -> crate::entry::AuditEntry {
        AuditEntryBuilder::new()
            .seq(0)
            .actor(Principal {
                kind: "System".into(),
                id: "test".into(),
            })
            .event("eval.started")
            .authorization("Allowed")
            .outcome("Succeeded")
            .at(fixed_time())
            .detail(serde_json::json!({"trial": 1}))
            .build()
            .unwrap()
    }

    fn build_chain(n: usize) -> Vec<SealedAuditEntry> {
        assert!(n > 0, "chain must have at least 1 entry (genesis)");
        let (mut head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let mut entries = vec![genesis];
        for _ in 1..n {
            entries.push(head.link(sample_entry()).unwrap());
        }
        entries
    }

    #[test]
    fn clean_chain_verifies_clean() {
        let chain = build_chain(4);
        let result = ChainVerifier::verify(&chain);
        assert!(result.is_clean());
        assert_eq!(result.findings().len(), 0);
    }

    #[test]
    fn empty_chain_verifies_clean() {
        let result = ChainVerifier::verify(&[]);
        assert!(result.is_clean());
    }

    #[test]
    fn tampered_context_detected() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[2] {
            base.detail = serde_json::json!({"tampered": true});
        }
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.has_entry_hash_mismatch_at_seq(2));
    }

    #[test]
    fn tampered_parent_hash_detected() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained {
            ref mut parent_hash,
            ..
        } = chain[2]
        {
            *parent_hash = [0u8; 32];
        }
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.has_parent_hash_mismatch_at_seq(2));
    }

    #[test]
    fn seq_discontinuity_detected() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[2] {
            base.seq = 99;
        }
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.has_seq_discontinuity_at_index(2));
    }

    #[test]
    fn non_genesis_at_zero_detected() {
        let chain = build_chain(2);
        let forged = SealedAuditEntry::Chained {
            base: chain[0].base().clone(),
            parent_hash: [1u8; 32],
            entry_hash: chain[0].entry_hash(),
        };
        let tampered = vec![forged, chain[1].clone()];
        let result = ChainVerifier::verify(&tampered);
        assert!(result.has_non_genesis_at_index_zero());
    }

    #[test]
    fn recompute_entry_hash_matches() {
        let chain = build_chain(3);
        for entry in &chain {
            let recomputed = ChainVerifier::recompute_entry_hash(entry).unwrap();
            assert_eq!(recomputed, entry.entry_hash());
        }
    }

    #[test]
    fn multiple_findings_can_accumulate() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[1] {
            base.detail = serde_json::json!({"tampered": true});
        }
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[3] {
            base.seq = 99;
        }
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.findings().len() >= 2);
    }

    #[test]
    fn genesis_with_model_hash_verifies_clean() {
        let chain = build_chain(1);
        assert!(chain[0].is_genesis());
        let result = ChainVerifier::verify(&chain);
        assert!(result.is_clean());
        let recomputed = ChainVerifier::recompute_entry_hash(&chain[0]).unwrap();
        assert_eq!(recomputed, chain[0].entry_hash());
    }
}
```

- [ ] **Step 3: Run tests for audit-chain**

Run: `cargo test -p audit-chain`

Expected: All tests in seal.rs and verify.rs PASS. This validates the core type transformation.

- [ ] **Step 4: Commit**

```bash
git add crates/audit-chain/src/seal.rs crates/audit-chain/src/verify.rs
git commit -m "feat(audit-chain): SealedAuditEntry enum with genesis sentinel

SealedAuditEntry is now a Genesis|Chained enum. ChainHead::new()
requires ModelIdentity and produces genesis immediately. Model hash
replaces [0u8; 32] sentinel. compute_entry_hash split into
compute_genesis_hash and compute_chained_hash."
```

---

### Task 4: ChainVerifier + ChainFinding — Full Rewrite

Now rewrite the verifier with proper genesis-aware findings. The mechanical bridge from Task 3 gets replaced.

**Files:**
- Modify: `crates/audit-chain/src/verify.rs`

- [ ] **Step 1: Replace verify.rs with the full genesis-aware verifier and tests**

```rust
use crate::canonical::CanonicalEncodingError;
use crate::model_identity::ModelIdentity;
use crate::seal::{compute_chained_hash, compute_genesis_hash, SealedAuditEntry};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ChainFinding {
    EntryHashMismatch {
        index: usize,
        seq: u64,
    },
    ParentHashMismatch {
        index: usize,
        seq: u64,
    },
    SeqDiscontinuity {
        index: usize,
        expected: u64,
        actual: u64,
    },
    MissingGenesis,
    ChainedAtIndexZero,
    GenesisHashMismatch,
    DuplicateGenesis {
        index: usize,
    },
}

#[derive(Debug)]
pub struct ChainFindings {
    findings: Vec<ChainFinding>,
}

impl ChainFindings {
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn findings(&self) -> &[ChainFinding] {
        &self.findings
    }

    pub fn has_entry_hash_mismatch_at_seq(&self, seq: u64) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::EntryHashMismatch { seq: s, .. } if *s == seq))
    }

    pub fn has_parent_hash_mismatch_at_seq(&self, seq: u64) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::ParentHashMismatch { seq: s, .. } if *s == seq))
    }

    pub fn has_seq_discontinuity_at_index(&self, index: usize) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::SeqDiscontinuity { index: i, .. } if *i == index))
    }

    pub fn has_missing_genesis(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::MissingGenesis))
    }

    pub fn has_chained_at_index_zero(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::ChainedAtIndexZero))
    }

    pub fn has_genesis_hash_mismatch(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::GenesisHashMismatch))
    }

    pub fn has_duplicate_genesis_at_index(&self, index: usize) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f, ChainFinding::DuplicateGenesis { index: i } if *i == index))
    }
}

pub struct ChainVerifier;

impl ChainVerifier {
    pub fn verify(entries: &[SealedAuditEntry]) -> ChainFindings {
        let mut findings = Vec::new();

        if entries.is_empty() {
            findings.push(ChainFinding::MissingGenesis);
            return ChainFindings { findings };
        }

        for (i, entry) in entries.iter().enumerate() {
            if i == 0 {
                match entry {
                    SealedAuditEntry::Chained { .. } => {
                        findings.push(ChainFinding::ChainedAtIndexZero);
                    }
                    SealedAuditEntry::Genesis {
                        base,
                        model_identity,
                        entry_hash,
                    } => {
                        if let Ok(recomputed) =
                            compute_genesis_hash(base, model_identity.hash)
                        {
                            if recomputed != *entry_hash {
                                findings.push(ChainFinding::GenesisHashMismatch);
                            }
                        }
                    }
                }
            } else {
                match entry {
                    SealedAuditEntry::Genesis { .. } => {
                        findings.push(ChainFinding::DuplicateGenesis { index: i });
                    }
                    SealedAuditEntry::Chained {
                        base,
                        parent_hash,
                        entry_hash,
                    } => {
                        let expected_parent = entries[i - 1].entry_hash();
                        if *parent_hash != expected_parent {
                            findings.push(ChainFinding::ParentHashMismatch {
                                index: i,
                                seq: base.seq,
                            });
                        }

                        if let Ok(recomputed) = compute_chained_hash(base, *parent_hash)
                        {
                            if recomputed != *entry_hash {
                                findings.push(ChainFinding::EntryHashMismatch {
                                    index: i,
                                    seq: base.seq,
                                });
                            }
                        }
                    }
                }
            }

            if i > 0 {
                let expected_seq = entries[i - 1].seq() + 1;
                if entry.seq() != expected_seq {
                    findings.push(ChainFinding::SeqDiscontinuity {
                        index: i,
                        expected: expected_seq,
                        actual: entry.seq(),
                    });
                }
            }
        }

        ChainFindings { findings }
    }

    pub fn recompute_entry_hash(
        entry: &SealedAuditEntry,
    ) -> Result<[u8; 32], CanonicalEncodingError> {
        match entry {
            SealedAuditEntry::Genesis {
                base,
                model_identity,
                ..
            } => compute_genesis_hash(base, model_identity.hash),
            SealedAuditEntry::Chained {
                base, parent_hash, ..
            } => compute_chained_hash(base, *parent_hash),
        }
    }

    pub fn model_identity(entries: &[SealedAuditEntry]) -> Option<&ModelIdentity> {
        match entries.first()? {
            SealedAuditEntry::Genesis {
                model_identity, ..
            } => Some(model_identity),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{AuditEntryBuilder, Principal};
    use crate::model_identity::{ModelHashMethod, ModelIdentity};
    use crate::seal::ChainHead;
    use chrono::{TimeZone, Utc};

    fn sample_model() -> ModelIdentity {
        ModelIdentity {
            name: "test-model".into(),
            provider: "test-provider".into(),
            version: None,
            quantization: None,
            hash_method: ModelHashMethod::StructuredDescriptor,
            hash: [42u8; 32],
        }
    }

    fn fixed_time() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn sample_entry() -> crate::entry::AuditEntry {
        AuditEntryBuilder::new()
            .seq(0)
            .actor(Principal {
                kind: "System".into(),
                id: "test".into(),
            })
            .event("eval.started")
            .authorization("Allowed")
            .outcome("Succeeded")
            .at(fixed_time())
            .detail(serde_json::json!({"trial": 1}))
            .build()
            .unwrap()
    }

    fn build_chain(n: usize) -> Vec<SealedAuditEntry> {
        assert!(n > 0, "chain must have at least 1 entry (genesis)");
        let (mut head, genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let mut entries = vec![genesis];
        for _ in 1..n {
            entries.push(head.link(sample_entry()).unwrap());
        }
        entries
    }

    #[test]
    fn clean_chain_verifies_clean() {
        let chain = build_chain(4);
        let result = ChainVerifier::verify(&chain);
        assert!(result.is_clean());
    }

    #[test]
    fn empty_chain_has_missing_genesis() {
        let result = ChainVerifier::verify(&[]);
        assert!(!result.is_clean());
        assert!(result.has_missing_genesis());
    }

    #[test]
    fn chained_at_index_zero_detected() {
        let chain = build_chain(2);
        let forged = SealedAuditEntry::Chained {
            base: chain[0].base().clone(),
            parent_hash: [0u8; 32],
            entry_hash: chain[0].entry_hash(),
        };
        let tampered = vec![forged, chain[1].clone()];
        let result = ChainVerifier::verify(&tampered);
        assert!(result.has_chained_at_index_zero());
    }

    #[test]
    fn genesis_hash_mismatch_detected() {
        let mut chain = build_chain(2);
        if let SealedAuditEntry::Genesis {
            ref mut model_identity,
            ..
        } = chain[0]
        {
            model_identity.hash = [99u8; 32];
        }
        let result = ChainVerifier::verify(&chain);
        assert!(result.has_genesis_hash_mismatch());
    }

    #[test]
    fn duplicate_genesis_detected() {
        let mut chain = build_chain(3);
        let (_, extra_genesis) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        chain[2] = extra_genesis;
        let result = ChainVerifier::verify(&chain);
        assert!(result.has_duplicate_genesis_at_index(2));
    }

    #[test]
    fn tampered_context_detected() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[2] {
            base.detail = serde_json::json!({"tampered": true});
        }
        let result = ChainVerifier::verify(&chain);
        assert!(result.has_entry_hash_mismatch_at_seq(2));
    }

    #[test]
    fn tampered_parent_hash_detected() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained {
            ref mut parent_hash,
            ..
        } = chain[2]
        {
            *parent_hash = [0u8; 32];
        }
        let result = ChainVerifier::verify(&chain);
        assert!(result.has_parent_hash_mismatch_at_seq(2));
    }

    #[test]
    fn seq_discontinuity_detected() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[2] {
            base.seq = 99;
        }
        let result = ChainVerifier::verify(&chain);
        assert!(result.has_seq_discontinuity_at_index(2));
    }

    #[test]
    fn recompute_entry_hash_matches_all() {
        let chain = build_chain(4);
        for entry in &chain {
            let recomputed = ChainVerifier::recompute_entry_hash(entry).unwrap();
            assert_eq!(recomputed, entry.entry_hash());
        }
    }

    #[test]
    fn model_identity_accessor_works() {
        let chain = build_chain(3);
        let mi = ChainVerifier::model_identity(&chain).unwrap();
        assert_eq!(mi.name, "test-model");
        assert_eq!(mi.hash, [42u8; 32]);
    }

    #[test]
    fn model_identity_returns_none_for_empty() {
        assert!(ChainVerifier::model_identity(&[]).is_none());
    }

    #[test]
    fn model_identity_returns_none_when_no_genesis() {
        let chain = build_chain(2);
        let forged = SealedAuditEntry::Chained {
            base: chain[0].base().clone(),
            parent_hash: [0u8; 32],
            entry_hash: chain[0].entry_hash(),
        };
        assert!(ChainVerifier::model_identity(&[forged]).is_none());
    }

    #[test]
    fn multiple_findings_can_accumulate() {
        let mut chain = build_chain(4);
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[1] {
            base.detail = serde_json::json!({"tampered": true});
        }
        if let SealedAuditEntry::Chained { ref mut base, .. } = chain[3] {
            base.seq = 99;
        }
        let result = ChainVerifier::verify(&chain);
        assert!(!result.is_clean());
        assert!(result.findings().len() >= 2);
    }

    #[test]
    fn genesis_only_chain_verifies_clean() {
        let chain = build_chain(1);
        assert!(chain[0].is_genesis());
        let result = ChainVerifier::verify(&chain);
        assert!(result.is_clean());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p audit-chain`

Expected: All tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/audit-chain/src/verify.rs
git commit -m "feat(audit-chain): genesis-aware ChainVerifier with new findings

Replaces NonGenesisAtIndexZero with MissingGenesis, ChainedAtIndexZero,
GenesisHashMismatch, DuplicateGenesis. Adds model_identity accessor."
```

---

### Task 5: EventKind::ChainGenesis

**Files:**
- Modify: `crates/audit-events/src/event_kind.rs`

- [ ] **Step 1: Add ChainGenesis variant to EventKind**

In `crates/audit-events/src/event_kind.rs`, add the variant to the enum (after `ChainVerified`, line 31):

```rust
    ChainGenesis,
```

Add to `as_str()` match (after the `ChainVerified` arm):

```rust
            Self::ChainGenesis => "chain.genesis",
```

Add to `parse()` match (after the `"chain.verified"` arm):

```rust
            "chain.genesis" => Some(Self::ChainGenesis),
```

Add to `all()` array (after `Self::ChainVerified`):

```rust
            Self::ChainGenesis,
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p audit-events`

Expected: PASS (existing serde tests still work, new variant is wired through).

- [ ] **Step 3: Commit**

```bash
git add crates/audit-events/src/event_kind.rs
git commit -m "feat(audit-events): add ChainGenesis event kind"
```

---

### Task 6: ChainHeadSnapshot model_hash

**Files:**
- Modify: `crates/audit-sign/src/snapshot.rs`

- [ ] **Step 1: Add model_hash field and update tests**

In `crates/audit-sign/src/snapshot.rs`, add `model_hash` to the struct (after `seq_through`, line 7):

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_hash: Option<String>,
```

Update `from_chain_head` — since `ChainHead` does not carry the model hash (it only tracks tip hash and next seq), `from_chain_head` cannot populate this field. Add a separate constructor for when model hash is known. Update the existing constructor to set `model_hash: None`:

```rust
impl ChainHeadSnapshot {
    pub fn from_chain_head(head: &ChainHead) -> Self {
        Self {
            tip_hash: head.last_entry_hash().map(hex_encode),
            seq_through: if head.next_seq() == 0 {
                0
            } else {
                head.next_seq() - 1
            },
            model_hash: None,
        }
    }

    pub fn with_model_hash(mut self, model_hash: [u8; 32]) -> Self {
        self.model_hash = Some(hex_encode(model_hash));
        self
    }

    pub fn canonical_bytes(
        &self,
    ) -> Result<Vec<u8>, audit_chain::canonical::CanonicalEncodingError> {
        audit_chain::canonical::encode(self)
    }
}
```

Update tests — the `empty_chain_snapshot` test needs adjustment since `ChainHead::new()` now requires `ModelIdentity`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use audit_chain::entry::{AuditEntryBuilder, Principal};
    use audit_chain::model_identity::{ModelHashMethod, ModelIdentity};
    use audit_chain::seal::ChainHead;
    use chrono::{TimeZone, Utc};

    fn sample_model() -> ModelIdentity {
        ModelIdentity {
            name: "test-model".into(),
            provider: "test-provider".into(),
            version: None,
            quantization: None,
            hash_method: ModelHashMethod::StructuredDescriptor,
            hash: [42u8; 32],
        }
    }

    fn fixed_time() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn sample_entry() -> audit_chain::entry::AuditEntry {
        AuditEntryBuilder::new()
            .seq(0)
            .actor(Principal {
                kind: "System".into(),
                id: "test".into(),
            })
            .event("eval.started")
            .authorization("Allowed")
            .outcome("Succeeded")
            .at(fixed_time())
            .detail(serde_json::json!({"trial": 1}))
            .build()
            .unwrap()
    }

    #[test]
    fn snapshot_after_genesis() {
        let (head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head);
        assert!(snap.tip_hash.is_some());
        assert_eq!(snap.tip_hash.as_ref().unwrap().len(), 64);
        assert_eq!(snap.seq_through, 0);
        assert!(snap.model_hash.is_none());
    }

    #[test]
    fn snapshot_after_entries() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        head.link(sample_entry()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head);
        assert!(snap.tip_hash.is_some());
        assert_eq!(snap.seq_through, 1);
    }

    #[test]
    fn snapshot_with_model_hash() {
        let (head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head)
            .with_model_hash(sample_model().hash);
        assert!(snap.model_hash.is_some());
        assert_eq!(snap.model_hash.as_ref().unwrap().len(), 64);
    }

    #[test]
    fn snapshot_serde_round_trip() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        head.link(sample_entry()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head)
            .with_model_hash(sample_model().hash);
        let json = serde_json::to_string(&snap).unwrap();
        let back: ChainHeadSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tip_hash, snap.tip_hash);
        assert_eq!(back.seq_through, snap.seq_through);
        assert_eq!(back.model_hash, snap.model_hash);
    }

    #[test]
    fn snapshot_omits_model_hash_when_none() {
        let (head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head);
        let json = serde_json::to_string(&snap).unwrap();
        assert!(!json.contains("model_hash"));
    }

    #[test]
    fn canonical_bytes_deterministic() {
        let (mut head, _) = ChainHead::new(sample_model(), fixed_time()).unwrap();
        head.link(sample_entry()).unwrap();
        let snap = ChainHeadSnapshot::from_chain_head(&head);
        let b1 = snap.canonical_bytes().unwrap();
        let b2 = snap.canonical_bytes().unwrap();
        assert_eq!(b1, b2);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p audit-sign`

Expected: All tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/audit-sign/src/snapshot.rs
git commit -m "feat(audit-sign): add model_hash to ChainHeadSnapshot"
```

---

### Task 7: audit-recover — Replay for New Enum

**Files:**
- Modify: `crates/audit-recover/src/replay.rs`
- Modify: `crates/audit-recover/src/gc.rs`

- [ ] **Step 1: Update replay.rs to use accessor methods**

The replay function deserializes `SealedAuditEntry` from JSONL. With the new tagged enum format, serde handles deserialization automatically. The only code change is using accessor methods instead of direct field access.

Replace the body of `replay_chain_str` (lines 27-67):

```rust
pub fn replay_chain_str(contents: &str) -> Result<ReplayResult, ReplayError> {
    let mut head = None::<ChainHead>;
    let mut count = 0usize;
    let mut truncated = 0usize;

    let lines: Vec<&str> = contents.lines().collect();
    let total_lines = lines.len();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<SealedAuditEntry>(trimmed) {
            Ok(entry) => {
                head = Some(ChainHead::resume(entry.entry_hash(), entry.seq() + 1));
                count += 1;
            }
            Err(e) => {
                if i == total_lines - 1 {
                    truncated += 1;
                    eprintln!(
                        "audit-recover: truncated last line {}, skipping (crash recovery)",
                        i + 1
                    );
                } else {
                    return Err(ReplayError::JsonParse {
                        line: i + 1,
                        source: e,
                    });
                }
            }
        }
    }

    let chain_head = head.unwrap_or_else(|| {
        ChainHead::resume([0u8; 32], 0)
    });

    Ok(ReplayResult {
        chain_head,
        entry_count: count,
        truncated_lines: truncated,
    })
}
```

Wait — there's a problem. The old code starts with `ChainHead::new()` for an empty chain, then calls `ChainHead::resume()` for each entry found. But `ChainHead::new()` now requires a `ModelIdentity`. For replay of an empty chain (no entries), we have no model identity.

The replay function is used by `Emitter::open()` to resume an existing chain. For empty chains, the emitter currently gets a fresh `ChainHead::new()`. But `new()` now requires `ModelIdentity`.

This is a design consideration: `Emitter::open()` needs to know the `ModelIdentity` to create a new chain, but can replay an existing one from the JSONL file (which already contains the genesis entry with model identity baked in).

For `replay_chain_str`, when the chain is empty (no entries), we return a `ReplayResult` that the caller can check. The caller (`Emitter`) then decides whether to create a new chain or error.

Let me restructure: `ReplayResult` should indicate whether entries were found, and the caller handles the empty case.

Actually, looking more carefully at the replay code and how it's used: `replay_chain_str` is called by `replay_chain_file`, which is called by `Emitter::open()`. The emitter uses the returned `chain_head` to resume linking. When the chain file is empty, the emitter gets a fresh `ChainHead::new()` — but now that requires `ModelIdentity`.

The cleanest fix: change `ReplayResult` to return `Option<ChainHead>` — `None` if no entries were replayed, `Some(head)` if entries were found. The emitter caller then handles the None case.

Update `replay.rs`:

```rust
use audit_chain::seal::{ChainHead, SealedAuditEntry};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ReplayError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error at line {line}: {source}")]
    JsonParse {
        line: usize,
        source: serde_json::Error,
    },
}

#[derive(Debug)]
pub struct ReplayResult {
    pub chain_head: Option<ChainHead>,
    pub entry_count: usize,
    pub truncated_lines: usize,
}

pub fn replay_chain_file(path: &std::path::Path) -> Result<ReplayResult, ReplayError> {
    let contents = std::fs::read_to_string(path)?;
    replay_chain_str(&contents)
}

pub fn replay_chain_str(contents: &str) -> Result<ReplayResult, ReplayError> {
    let mut head = None::<ChainHead>;
    let mut count = 0usize;
    let mut truncated = 0usize;

    let lines: Vec<&str> = contents.lines().collect();
    let total_lines = lines.len();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<SealedAuditEntry>(trimmed) {
            Ok(entry) => {
                head = Some(ChainHead::resume(entry.entry_hash(), entry.seq() + 1));
                count += 1;
            }
            Err(e) => {
                if i == total_lines - 1 {
                    truncated += 1;
                    eprintln!(
                        "audit-recover: truncated last line {}, skipping (crash recovery)",
                        i + 1
                    );
                } else {
                    return Err(ReplayError::JsonParse {
                        line: i + 1,
                        source: e,
                    });
                }
            }
        }
    }

    Ok(ReplayResult {
        chain_head: head,
        entry_count: count,
        truncated_lines: truncated,
    })
}
```

- [ ] **Step 2: Update gc.rs to use accessor methods**

In `crates/audit-recover/src/gc.rs`, line 38, change:

```rust
if let Some(blob_ref) = &sealed.base.blob_ref {
```

to:

```rust
if let Some(blob_ref) = &sealed.base().blob_ref {
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p audit-recover`

Expected: PASS (no unit tests in this crate, but it must compile).

- [ ] **Step 4: Commit**

```bash
git add crates/audit-recover/src/replay.rs crates/audit-recover/src/gc.rs
git commit -m "fix(audit-recover): update replay and gc for SealedAuditEntry enum

ReplayResult.chain_head is now Option<ChainHead> since empty chains
have no genesis to replay. gc uses accessor methods."
```

---

### Task 8: audit-emit — Emitter for New ChainHead API

**Files:**
- Modify: `crates/audit-emit/src/emitter.rs`

- [ ] **Step 1: Update Emitter::open() to handle the new replay result and ChainHead API**

The emitter must handle two cases:
1. **Resuming**: chain.jsonl exists with entries → use replayed `ChainHead`
2. **New chain**: no chain.jsonl → caller must provide `ModelIdentity` to create genesis

Change `Emitter::open()` to require `ModelIdentity` for new chains. When resuming, the model identity is already baked into the genesis entry on disk.

Replace the full file:

```rust
use std::io::Write;
use std::path::{Path, PathBuf};

use audit_chain::entry::{
    AuditEntryBuilder, BlobLocation as ChainBlobLocation, BlobRef as ChainBlobRef,
    Principal as ChainPrincipal, ResourceRef as ChainResourceRef,
};
use audit_chain::model_identity::ModelIdentity;
use audit_chain::seal::{ChainHead, SealedAuditEntry};
use audit_events::{validate_tags, AuditEvent, BlobLocation};
use audit_sign::signing::AuditSigner;
use fs2::FileExt;

use crate::blob_store::BlobStore;
use crate::config::EmitterConfig;
use crate::error::AuditError;

pub struct Emitter {
    chain: ChainHead,
    chain_path: PathBuf,
    blob_store: BlobStore,
    signer: Option<Box<dyn AuditSigner>>,
    config: EmitterConfig,
    lock_file: std::fs::File,
    audit_dir: PathBuf,
    genesis_pending: Option<SealedAuditEntry>,
}

impl Emitter {
    pub fn open(audit_dir: &Path, model: ModelIdentity) -> Result<Self, AuditError> {
        std::fs::create_dir_all(audit_dir)?;

        let lock_path = audit_dir.join(".lock");
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;
        lock_file.lock_exclusive()?;

        let chain_path = audit_dir.join("chain.jsonl");
        let (chain, genesis_pending) = if chain_path.exists() {
            let result = audit_recover::replay::replay_chain_file(&chain_path)?;
            match result.chain_head {
                Some(head) => (head, None),
                None => {
                    let (head, genesis) =
                        ChainHead::new(model, chrono::Utc::now())?;
                    (head, Some(genesis))
                }
            }
        } else {
            let (head, genesis) = ChainHead::new(model, chrono::Utc::now())?;
            (head, Some(genesis))
        };

        let blob_store = BlobStore::new(audit_dir.join("blobs"));

        let mut emitter = Self {
            chain,
            chain_path: chain_path.clone(),
            blob_store,
            signer: None,
            config: EmitterConfig::default(),
            lock_file,
            audit_dir: audit_dir.to_path_buf(),
            genesis_pending,
        };

        if let Some(genesis) = emitter.genesis_pending.take() {
            let line = serde_json::to_string(&genesis)?;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&chain_path)?;
            writeln!(file, "{line}")?;
            file.sync_all()?;
        }

        Ok(emitter)
    }

    pub fn with_signer(mut self, signer: Box<dyn AuditSigner>) -> Self {
        self.signer = Some(signer);
        self
    }

    pub fn with_config(mut self, config: EmitterConfig) -> Self {
        self.config = config;
        self
    }

    pub fn emit(&mut self, event: AuditEvent) -> Result<SealedAuditEntry, AuditError> {
        self.emit_inner(event, None)
    }

    pub fn emit_with_blob(
        &mut self,
        event: AuditEvent,
        blob: &[u8],
        content_type: &str,
    ) -> Result<SealedAuditEntry, AuditError> {
        self.emit_inner(event, Some((blob, content_type)))
    }

    fn emit_inner(
        &mut self,
        mut event: AuditEvent,
        blob: Option<(&[u8], &str)>,
    ) -> Result<SealedAuditEntry, AuditError> {
        validate_tags(
            &event.tags,
            self.config.tags_max_pairs,
            self.config.tag_value_max_bytes,
        )?;

        if let Some((data, ct)) = blob {
            let blob_ref = self.blob_store.store(data, ct)?;
            event.blob_ref = Some(audit_events::BlobRef {
                hash: blob_ref.hash,
                location: blob_ref.location,
                size_bytes: blob_ref.size_bytes,
                content_type: blob_ref.content_type,
            });
        }

        let detail_json = serde_json::to_string(&event.detail)?;
        if detail_json.len() > self.config.detail_max_bytes && event.blob_ref.is_none() {
            let blob_ref = self
                .blob_store
                .store(detail_json.as_bytes(), "application/json")?;
            event.detail = serde_json::json!({
                "__promoted_to_blob": true
            });
            event.blob_ref = Some(audit_events::BlobRef {
                hash: blob_ref.hash,
                location: blob_ref.location,
                size_bytes: blob_ref.size_bytes,
                content_type: blob_ref.content_type,
            });
        }

        let authorization_str = serde_json::to_string(&event.authorization)?;
        let outcome_str = serde_json::to_string(&event.outcome)?;

        let mut builder = AuditEntryBuilder::new()
            .seq(0)
            .at(event.at)
            .actor(ChainPrincipal {
                kind: event.actor.kind.clone(),
                id: event.actor.id.clone(),
            })
            .event(event.event.as_str())
            .authorization(authorization_str.trim_matches('"'))
            .outcome(outcome_str.trim_matches('"'))
            .tags(event.tags)
            .detail(event.detail);

        if let Some(ns) = event.monotonic_ns {
            builder = builder.monotonic_ns(ns);
        }
        if let Some(trace_id) = event.trace_id {
            builder = builder.trace_id(trace_id.0);
        }
        builder = builder.resource(ChainResourceRef::new(
            &event.resource.kind,
            &event.resource.id,
        ));
        if let Some(blob_ref) = event.blob_ref {
            builder = builder.blob_ref(ChainBlobRef {
                hash: blob_ref.hash,
                location: ChainBlobLocation::File {
                    path: match blob_ref.location {
                        BlobLocation::File { path } => path,
                    },
                },
                size_bytes: blob_ref.size_bytes,
                content_type: blob_ref.content_type,
            });
        }

        let entry = builder
            .build()
            .map_err(|e| AuditError::BlobStore(format!("entry build failed: {e}")))?;

        let sealed = self.chain.link(entry)?;

        let line = serde_json::to_string(&sealed)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.chain_path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;

        if let Some(signer) = &self.signer {
            let snapshot = audit_sign::snapshot::ChainHeadSnapshot::from_chain_head(&self.chain);
            let cbor = audit_sign::attestation::build_tip_attestation(signer.as_ref(), &snapshot)
                .map_err(|e| AuditError::BlobStore(format!("attestation failed: {e}")))?;

            let att_dir = self.audit_dir.join("attestations");
            std::fs::create_dir_all(&att_dir)?;
            std::fs::write(att_dir.join(format!("{}.cbor", sealed.seq())), &cbor)?;
        }

        Ok(sealed)
    }

    pub fn chain_head(&self) -> &ChainHead {
        &self.chain
    }
}

impl Drop for Emitter {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.lock_file);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p audit-emit`

Expected: PASS (compiles with new API).

- [ ] **Step 3: Commit**

```bash
git add crates/audit-emit/src/emitter.rs
git commit -m "fix(audit-emit): Emitter::open requires ModelIdentity for new chains

Genesis entry is created and persisted when opening a new audit
directory. Existing chains resume via replay."
```

---

### Task 9: mojave-cli — Audit Commands

**Files:**
- Modify: `crates/mojave-cli/src/commands/audit.rs`
- Modify: `crates/mojave-cli/src/main.rs`

- [ ] **Step 1: Update SealInput to include model identity fields**

In `crates/mojave-cli/src/commands/audit.rs`, add model identity fields to `SealInput` (after `actor`, line 19):

```rust
#[derive(Debug, serde::Deserialize)]
pub struct SealInput {
    pub run_id: String,
    pub eval_name: String,
    pub date_issued: String,
    pub data_file: PathBuf,
    pub data_sha256: String,
    pub actor: ActorInput,
    pub model: ModelInput,
}

#[derive(Debug, serde::Deserialize)]
pub struct ModelInput {
    pub name: String,
    pub provider: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub quantization: Option<String>,
    pub hash_method: String,
    pub hash: String,
}
```

- [ ] **Step 2: Add model_hash to ChainHeadState**

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct ChainHeadState {
    #[serde(skip_serializing_if = "Option::is_none")]
    tip_hash: Option<String>,
    next_seq: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_hash: Option<String>,
}
```

- [ ] **Step 3: Add helper to convert ModelInput to ModelIdentity**

```rust
fn parse_model_input(input: &ModelInput) -> Result<ModelIdentity, CliError> {
    let hash = hex_decode_32(&input.hash)?;
    let hash_method = match input.hash_method.as_str() {
        "WeightFile" => ModelHashMethod::WeightFile,
        "StructuredDescriptor" => ModelHashMethod::StructuredDescriptor,
        other => {
            return Err(CliError::Audit(format!(
                "unknown hash_method: {other} (expected WeightFile or StructuredDescriptor)"
            )))
        }
    };
    Ok(ModelIdentity {
        name: input.name.clone(),
        provider: input.provider.clone(),
        version: input.version.clone(),
        quantization: input.quantization.clone(),
        hash_method,
        hash,
    })
}
```

Add the necessary imports at the top of the file:

```rust
use audit_chain::model_identity::{ModelHashMethod, ModelIdentity};
```

- [ ] **Step 4: Update load_chain_head and save_chain_head**

Replace `load_chain_head` (lines 57-73):

```rust
fn load_chain_head(audit_dir: &Path) -> Result<(ChainHead, Option<String>), CliError> {
    let head_path = audit_dir.join("chain-head.json");
    if !head_path.exists() {
        return Err(CliError::Audit(
            "no chain-head.json found; use 'seal' to create a new chain".into(),
        ));
    }
    let data = std::fs::read_to_string(&head_path)
        .map_err(|e| CliError::Audit(format!("cannot read chain head: {e}")))?;
    let state: ChainHeadState = serde_json::from_str(&data)
        .map_err(|e| CliError::Audit(format!("invalid chain head JSON: {e}")))?;
    match state.tip_hash {
        Some(hex) => {
            let bytes = hex_decode_32(&hex)?;
            Ok((ChainHead::resume(bytes, state.next_seq), state.model_hash))
        }
        None => Err(CliError::Audit("chain-head.json has no tip_hash".into())),
    }
}
```

Replace `save_chain_head` (lines 75-85):

```rust
fn save_chain_head(
    audit_dir: &Path,
    head: &ChainHead,
    model_hash: Option<&str>,
) -> Result<(), CliError> {
    let state = ChainHeadState {
        tip_hash: head.last_entry_hash().map(|h| hex_encode(&h)),
        next_seq: head.next_seq(),
        model_hash: model_hash.map(String::from),
    };
    let json = serde_json::to_string_pretty(&state)
        .map_err(|e| CliError::Audit(format!("cannot serialize chain head: {e}")))?;
    std::fs::write(audit_dir.join("chain-head.json"), json)
        .map_err(|e| CliError::Audit(format!("cannot write chain head: {e}")))?;
    Ok(())
}
```

- [ ] **Step 5: Rewrite run_seal to create genesis for new chains**

Replace `run_seal` (lines 125-221):

```rust
pub fn run_seal(key_file: Option<&Path>) -> Result<(), CliError> {
    let mut stdin_buf = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin_buf)
        .map_err(|e| CliError::Audit(format!("cannot read stdin: {e}")))?;

    let input: SealInput = serde_json::from_str(&stdin_buf)
        .map_err(|e| CliError::Audit(format!("invalid seal input JSON: {e}")))?;

    let actual_hash = hash_file(&input.data_file)?;
    if actual_hash != input.data_sha256 {
        return Err(CliError::Audit(format!(
            "data file hash mismatch: expected {}, got {actual_hash}",
            input.data_sha256
        )));
    }

    let model = parse_model_input(&input.model)?;
    let model_hash_hex = hex_encode(&model.hash);

    let audit_dir = PathBuf::from("data/audit/chains")
        .join(&model_hash_hex[..16]);
    std::fs::create_dir_all(&audit_dir)
        .map_err(|e| CliError::Audit(format!("cannot create audit dir: {e}")))?;

    let head_path = audit_dir.join("chain-head.json");
    let mut head = if head_path.exists() {
        let (h, stored_model_hash) = load_chain_head(&audit_dir)?;
        if let Some(stored) = &stored_model_hash {
            if *stored != model_hash_hex {
                return Err(CliError::Audit(format!(
                    "model hash mismatch: chain has {stored}, input has {model_hash_hex}"
                )));
            }
        }
        h
    } else {
        let (h, genesis) = ChainHead::new(model.clone(), chrono::Utc::now())
            .map_err(|e| CliError::Audit(format!("cannot create chain: {e}")))?;
        append_chain_entry(&audit_dir, &genesis)?;
        h
    };

    let actor = Principal {
        kind: input.actor.kind.clone(),
        id: input.actor.id.clone(),
    };

    let entry = AuditEntryBuilder::new()
        .seq(0)
        .actor(actor)
        .event("run_card.generated")
        .resource(ResourceRef::new("eval", &input.eval_name))
        .authorization("Allowed")
        .outcome("Succeeded")
        .at(chrono::Utc::now())
        .detail(serde_json::json!({
            "run_id": input.run_id,
            "eval_name": input.eval_name,
            "date_issued": input.date_issued,
            "data_file": input.data_file.to_string_lossy(),
            "data_sha256": input.data_sha256,
        }))
        .build()
        .map_err(|e| CliError::Audit(format!("cannot build audit entry: {e}")))?;

    let sealed = head
        .link(entry)
        .map_err(|e| CliError::Audit(format!("cannot seal audit entry: {e}")))?;

    append_chain_entry(&audit_dir, &sealed)?;
    save_chain_head(&audit_dir, &head, Some(&model_hash_hex))?;

    let entry_hash = hex_encode(&sealed.entry_hash());
    let chain_tip_hash = hex_encode(&sealed.entry_hash());
    let chain_tip_seq = sealed.seq();

    let (attestation_cbor_b64, verifying_key_spki_b64) = match resolve_signer(key_file)? {
        Some(signer) => {
            let snapshot = ChainHeadSnapshot::from_chain_head(&head)
                .with_model_hash(model.hash);
            let cbor = audit_sign::attestation::build_tip_attestation(&signer, &snapshot)
                .map_err(|e| CliError::Audit(format!("attestation failed: {e}")))?;

            let att_dir = audit_dir.join("attestations");
            std::fs::create_dir_all(&att_dir)
                .map_err(|e| CliError::Audit(format!("cannot create attestations dir: {e}")))?;
            std::fs::write(att_dir.join(format!("{chain_tip_seq}.cbor")), &cbor)
                .map_err(|e| CliError::Audit(format!("cannot write attestation: {e}")))?;

            let spki = signer
                .verifying_key_spki_der()
                .map_err(|e| CliError::Audit(format!("cannot export public key: {e}")))?;
            let pubkey_path = PathBuf::from("data/audit/pubkey.spki.der");
            if let Some(parent) = pubkey_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| CliError::Audit(format!("cannot create dir: {e}")))?;
            }
            std::fs::write(&pubkey_path, &spki)
                .map_err(|e| CliError::Audit(format!("cannot write public key: {e}")))?;

            use base64::Engine;
            let b64_cbor = base64::engine::general_purpose::STANDARD.encode(&cbor);
            let b64_spki = base64::engine::general_purpose::STANDARD.encode(&spki);
            (Some(b64_cbor), Some(b64_spki))
        }
        None => (None, None),
    };

    let output = SealOutput {
        chain_tip_hash,
        chain_tip_seq,
        entry_hash,
        data_file_hash: actual_hash,
        attestation_cbor_b64,
        verifying_key_spki_b64,
    };

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| CliError::Audit(format!("cannot serialize output: {e}")))?;
    println!("{json}");
    Ok(())
}
```

- [ ] **Step 6: Update run_emit to pass ModelIdentity to Emitter::open**

In `run_emit` (lines 247-283), read model identity from stdin alongside the event. For now, use a placeholder model since `run_emit` takes events via stdin and model identity context isn't in the event payload. Add a `--model-hash` CLI arg or extract from the chain-head.json.

Simpler approach: `run_emit` operates on an existing chain (already has genesis). Read the model hash from chain-head.json and construct a stub `ModelIdentity`:

```rust
pub fn run_emit(blob_file: Option<&Path>, audit_dir: Option<&Path>) -> Result<(), CliError> {
    let audit_path = audit_dir.unwrap_or(Path::new("data/audit"));

    let mut stdin_buf = String::new();
    std::io::stdin()
        .read_to_string(&mut stdin_buf)
        .map_err(|e| CliError::Audit(format!("cannot read stdin: {e}")))?;

    let event: audit_events::AuditEvent = serde_json::from_str(&stdin_buf)
        .map_err(|e| CliError::Audit(format!("invalid event JSON: {e}")))?;

    let model = ModelIdentity {
        name: "unknown".into(),
        provider: "unknown".into(),
        version: None,
        quantization: None,
        hash_method: ModelHashMethod::StructuredDescriptor,
        hash: [0u8; 32],
    };

    let mut emitter = audit_emit::emitter::Emitter::open(audit_path, model)
        .map_err(|e| CliError::Audit(format!("cannot open emitter: {e}")))?;

    let sealed = if let Some(blob_path) = blob_file {
        let blob_data = std::fs::read(blob_path)
            .map_err(|e| CliError::Audit(format!("cannot read blob file: {e}")))?;
        emitter
            .emit_with_blob(event, &blob_data, "application/octet-stream")
            .map_err(|e| CliError::Audit(format!("emit failed: {e}")))?
    } else {
        emitter
            .emit(event)
            .map_err(|e| CliError::Audit(format!("emit failed: {e}")))?
    };

    let output = serde_json::json!({
        "seq": sealed.seq(),
        "entry_hash": hex_encode(&sealed.entry_hash()),
        "event": sealed.base().event,
    });

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| CliError::Audit(format!("cannot serialize output: {e}")))?;
    println!("{json}");
    Ok(())
}
```

Note: The placeholder `ModelIdentity` with zero hash will only be used if the chain is new (no existing chain.jsonl). For existing chains, `Emitter::open` resumes via replay and the model identity is not used. This is safe because `run_emit` should only be called on existing chains. If called on a new directory, it will error from `ChainHead::new` rejecting the zero hash.

- [ ] **Step 7: Update run_verify to use accessor methods**

In `run_verify`, update the output serialization (line 329):

```rust
    let findings = audit_chain::verify::ChainVerifier::verify(&entries);

    let model_info = audit_chain::verify::ChainVerifier::model_identity(&entries);

    let output = serde_json::json!({
        "entries_verified": entries.len(),
        "is_clean": findings.is_clean(),
        "findings": findings.findings().iter().map(|f| format!("{f:?}")).collect::<Vec<_>>(),
        "model": model_info.map(|mi| serde_json::json!({
            "name": mi.name,
            "provider": mi.provider,
            "hash": hex_encode(&mi.hash),
        })),
    });
```

- [ ] **Step 8: Build and test**

Run: `cargo build -p mojave-cli && cargo test -p mojave-cli`

Expected: Compiles and any existing tests pass.

- [ ] **Step 9: Commit**

```bash
git add crates/mojave-cli/src/commands/audit.rs
git commit -m "feat(mojave-cli): genesis sentinel in seal/verify/emit commands

seal creates per-model chain directory, emits genesis on first seal.
verify reports model identity. emit uses ModelIdentity for new chains."
```

---

### Task 10: Python Scripts

**Files:**
- Modify: `scripts/v2/verify_cards.py`
- Modify: `scripts/v2/repo.py`
- Modify: `scripts/v2/generate_run_cards.py`

- [ ] **Step 1: Update verify_cards.py for tagged enum JSON**

In `scripts/v2/verify_cards.py`, update `check_chain_integrity` to handle the new JSON format. The key difference is entries now have a `"type"` field (`"Genesis"` or `"Chained"`).

Find the parent hash linkage check and update it:

```python
def check_chain_integrity(entries: list[dict]) -> list[str]:
    """Verify hash chain linkage (DAG parent_hash -> entry_hash)."""
    issues = []
    if not entries:
        issues.append("FAIL: empty chain (missing genesis)")
        return issues

    # Check genesis
    first = entries[0]
    if first.get("type") != "Genesis":
        issues.append(f"FAIL: first entry is {first.get('type', 'unknown')}, expected Genesis")

    seen_hashes = {}
    for i, entry in enumerate(entries):
        entry_type = entry.get("type", "unknown")
        base = entry.get("base", {})
        seq = base.get("seq", "?")
        entry_hash_key = "entry_hash"

        if entry_type == "Genesis":
            if i != 0:
                issues.append(f"FAIL: duplicate Genesis at index {i}")
        elif entry_type == "Chained":
            parent_hash = entry.get("parent_hash")
            if parent_hash is not None:
                parent_key = str(parent_hash)
                if i > 0:
                    prev_hash = str(entries[i - 1].get(entry_hash_key))
                    if parent_key != prev_hash:
                        issues.append(
                            f"FAIL: entry {i} (seq {seq}) parent_hash does not match "
                            f"previous entry_hash"
                        )
        else:
            issues.append(f"FAIL: unknown entry type {entry_type!r} at index {i}")

        entry_hash = str(entry.get(entry_hash_key))
        if entry_hash in seen_hashes:
            issues.append(
                f"FAIL: duplicate entry_hash at index {i} (seq {seq}), "
                f"first seen at index {seen_hashes[entry_hash]}"
            )
        seen_hashes[entry_hash] = i

    if not issues:
        max_seq = max(e.get("base", {}).get("seq", 0) for e in entries)
        issues.append(f"OK: chain integrity verified ({len(entries)} entries, max seq {max_seq})")

    return issues
```

- [ ] **Step 2: Update repo.py audit_seal to include model identity**

In `scripts/v2/repo.py`, update the `audit_seal` function to accept and pass model identity:

```python
def audit_seal(
    run_id: str,
    eval_name: str,
    data_file: Path,
    model_name: str = "unknown",
    model_provider: str = "unknown",
    model_hash: str = "00" * 32,
    model_hash_method: str = "StructuredDescriptor",
    actor: str = "generate_run_cards_v2.py",
) -> dict[str, Any] | None:
    """Seal a run card into the audit chain via mojave CLI."""
    data_sha256 = compute_file_sha256(data_file)
    seal_input = {
        "run_id": run_id,
        "eval_name": eval_name,
        "date_issued": datetime.date.today().isoformat(),
        "data_file": str(data_file),
        "data_sha256": data_sha256,
        "actor": {"kind": "System", "id": actor},
        "model": {
            "name": model_name,
            "provider": model_provider,
            "hash_method": model_hash_method,
            "hash": model_hash,
        },
    }
    try:
        result = subprocess.run(
            [cargo_bin("mojave"), "audit", "seal"],
            input=json.dumps(seal_input),
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode != 0:
            print(f"WARNING: audit seal failed: {result.stderr}", file=sys.stderr)
            return None
        return json.loads(result.stdout)
    except (subprocess.TimeoutExpired, json.JSONDecodeError, FileNotFoundError) as e:
        print(f"WARNING: audit seal error: {e}", file=sys.stderr)
        return None
```

Add the import at the top of repo.py:

```python
import datetime
```

- [ ] **Step 3: Update generate_run_cards.py to pass model identity to audit_seal**

In `scripts/v2/generate_run_cards.py`, find the `audit_seal` call (around line 306) and update it to pass model identity. The model info should come from the analysis JSON or eval config:

```python
    seal_result = audit_seal(
        run_id=run_id,
        eval_name=eval_name,
        data_file=analysis_path,
        model_name=analysis.get("model_name", "unknown"),
        model_provider=analysis.get("model_provider", "unknown"),
        model_hash=analysis.get("model_hash", "00" * 32),
        model_hash_method=analysis.get("model_hash_method", "StructuredDescriptor"),
    )
```

- [ ] **Step 4: Commit**

```bash
git add scripts/v2/verify_cards.py scripts/v2/repo.py scripts/v2/generate_run_cards.py
git commit -m "fix(scripts): update Python pipeline for genesis sentinel format

verify_cards.py handles tagged enum JSON with Genesis/Chained types.
audit_seal passes model identity to mojave CLI."
```

---

### Task 11: Full Build + Integration Verification

**Files:** None (verification only)

- [ ] **Step 1: Run full workspace build**

Run: `cargo build --workspace`

Expected: Zero errors, zero warnings.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`

Expected: Zero warnings (enforced by workspace lints).

- [ ] **Step 3: Run all tests**

Run: `cargo test --workspace`

Expected: All tests pass across all crates.

- [ ] **Step 4: Run rustfmt check**

Run: `cargo fmt --all -- --check`

Expected: No formatting differences.

- [ ] **Step 5: Verify TCK scenarios are covered by tests**

Compare each scenario in `tck/audit-chain/features/chain_integrity.feature` against the test functions in `seal.rs` and `verify.rs`:

| TCK Scenario | Test Function |
|---|---|
| Genesis entry is created by ChainHead::new | `seal::tests::genesis_entry_created_by_new` |
| Genesis hash uses model hash as sentinel | `seal::tests::genesis_hash_uses_model_hash` |
| Different model hash produces different genesis hash | `seal::tests::different_model_hash_changes_genesis_hash` |
| Zero model hash is rejected | `seal::tests::zero_model_hash_rejected` |
| Second entry chains to genesis | `seal::tests::second_entry_chains_to_genesis` |
| Chained entry has required parent_hash | `seal::tests::chained_entry_has_parent_hash` |
| Clean chain verifies clean | `verify::tests::clean_chain_verifies_clean` |
| Empty chain produces MissingGenesis | `verify::tests::empty_chain_has_missing_genesis` |
| Chained entry at index zero detected | `verify::tests::chained_at_index_zero_detected` |
| Genesis hash mismatch detected | `verify::tests::genesis_hash_mismatch_detected` |
| Duplicate genesis detected | `verify::tests::duplicate_genesis_detected` |
| Tampered entry body detected | `verify::tests::tampered_context_detected` |
| Tampered parent hash detected | `verify::tests::tampered_parent_hash_detected` |
| Sequence discontinuity detected | `verify::tests::seq_discontinuity_detected` |
| Resumed chain continues | `seal::tests::resumed_chain_continues` |
| entry_hash is deterministic | `seal::tests::entry_hash_is_deterministic` |
| Single bit flip changes hash | `seal::tests::single_bit_detail_change_changes_hash` |
| Model identity accessor | `verify::tests::model_identity_accessor_works` |

- [ ] **Step 6: Commit any remaining fixes**

If any issues surfaced during verification, fix and commit them individually.
