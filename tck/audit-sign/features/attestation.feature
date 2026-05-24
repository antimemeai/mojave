Feature: COSE_Sign1 detached attestation for audit chain tips
  Ed25519 signing and verification of chain head snapshots via COSE_Sign1
  with detached payload (RFC 9052 §4.2).

  Background:
    Given a generated Ed25519 signing key with key_id "test-key"

  Scenario: Sign and verify round-trip
    Given a payload "hello audit chain"
    When I build a detached attestation over the payload
    And I verify the attestation with the correct payload
    Then verification succeeds

  Scenario: Tampered payload rejected
    Given a payload "original"
    When I build a detached attestation over the payload
    And I verify the attestation with payload "tampered"
    Then verification fails with SignatureInvalid

  Scenario: Unknown key id rejected
    Given a payload "hello"
    When I build a detached attestation over the payload
    And I verify the attestation with an empty keyring
    Then verification fails with UnknownKeyId

  Scenario: Invalid CBOR rejected
    When I verify raw bytes "not cbor" as an attestation
    Then verification fails with Cbor error

  Scenario: Chain tip attestation round-trip
    Given an audit chain with 3 entries
    When I build a tip attestation from the chain head
    And I verify the tip attestation with the chain head snapshot
    Then verification succeeds
