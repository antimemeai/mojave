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
