Feature: Hash chain integrity for sealed audit entries
  SHA-256 hash chain linking sealed audit entries via parent_hash → entry_hash.
  ChainHead produces entries; ChainVerifier detects tampering.

  Background:
    Given a new ChainHead

  Scenario: Genesis entry has no parent hash
    When I link one AuditEntry
    Then the sealed entry has seq 0
    And the sealed entry has no parent_hash
    And the entry_hash is 32 bytes

  Scenario: Second entry chains to first
    When I link two AuditEntry values
    Then the second sealed entry has seq 1
    And the second entry's parent_hash equals the first entry's entry_hash

  Scenario: Clean chain verifies clean
    When I link 4 AuditEntry values
    And I verify the chain
    Then the result is clean
    And entries_parsed is 4
    And seq_range is 0 to 3

  Scenario: Tampered entry body detected as entry hash mismatch
    When I link 4 AuditEntry values
    And I tamper with the context of entry at index 2
    And I verify the chain
    Then the result has an EntryHashMismatch finding at seq 2

  Scenario: Tampered parent hash detected
    When I link 4 AuditEntry values
    And I overwrite the parent_hash of entry at index 2 with zeros
    And I verify the chain
    Then the result has a ParentHashMismatch finding at seq 2

  Scenario: Sequence discontinuity detected
    When I link 4 AuditEntry values
    And I set the seq of entry at index 2 to 99
    And I verify the chain
    Then the result has a SeqDiscontinuity finding at index 2

  Scenario: Non-genesis at index zero detected
    When I link 2 AuditEntry values
    And I set the parent_hash of entry at index 0 to non-None
    And I verify the chain
    Then the result has a NonGenesisAtIndexZero finding

  Scenario: Empty chain verifies clean
    When I verify an empty chain
    Then the result is clean

  Scenario: Resumed chain continues from prior head
    Given a ChainHead resumed with a known entry_hash and next_seq 10
    When I link one AuditEntry
    Then the sealed entry has seq 10
    And the sealed entry's parent_hash equals the known entry_hash

  Scenario: entry_hash is deterministic
    Given two identical AuditEntry values with identical parent_hash
    When I compute entry_hash for both
    Then the hashes are identical

  Scenario: Single bit flip in context changes entry_hash
    Given two AuditEntry values differing by one bit in context
    When I compute entry_hash for both with the same parent_hash
    Then the hashes differ
