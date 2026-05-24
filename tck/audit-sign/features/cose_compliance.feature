Feature: COSE_Sign1 header compliance with RFC 9052 and RFC 9597
  Attestation envelopes must use standard IANA COSE header labels.

  Background:
    Given a generated Ed25519 signing key with key_id "compliance-test"

  Scenario: Content type uses standard label 3
    When I build a detached attestation over payload "test"
    Then the protected header content_type is "application/vnd.mojave.audit.chain-head+json"
    And the content_type uses the standard coset content_type field (label 3)

  Scenario: Timestamp uses CWT Claims label 15 with iat key 6
    When I build a detached attestation over payload "test"
    Then the protected header contains CWT Claims (label 15)
    And the CWT Claims map contains iat (key 6) as an integer

  Scenario: Algorithm is EdDSA in protected header
    When I build a detached attestation over payload "test"
    Then the protected header algorithm is EdDSA

  Scenario: Unprotected headers are empty
    When I build a detached attestation over payload "test"
    Then the unprotected header map is empty

  Scenario: Payload is detached
    When I build a detached attestation over payload "test"
    Then the COSE_Sign1 payload field is empty or nil

  Scenario: Algorithm allowlist rejects non-EdDSA
    Given a COSE_Sign1 envelope with algorithm ES256 and a valid structure
    When I verify the envelope
    Then verification fails with UnsupportedAlgorithm

  Scenario: Critical headers rejected
    Given a COSE_Sign1 envelope with crit header listing label 99
    When I verify the envelope
    Then verification fails with CriticalHeadersNotUnderstood
