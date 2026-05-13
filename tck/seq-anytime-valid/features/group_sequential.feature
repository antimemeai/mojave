Feature: Group-sequential boundaries and monitor
  Validates Pocock, OBF, and Lan-DeMets group-sequential designs.

  Scenario: OBF boundaries decrease across looks
    Given a 5-look OBF design at alpha = 0.05
    When I compute all boundaries
    Then boundary at look 1 > boundary at look 5

  Scenario: OBF at K=1 equals fixed-sample z
    Given a 1-look OBF design at alpha = 0.05
    When I compute boundary at look 1
    Then the boundary is approximately 1.96

  Scenario: Pocock boundaries are all equal
    Given a 3-look Pocock design at alpha = 0.05
    When I compute all boundaries
    Then all boundaries are equal

  Scenario: Pocock equals OBF at K=1
    Given a 1-look Pocock design at alpha = 0.05
    And a 1-look OBF design at alpha = 0.05
    When I compute both boundaries
    Then they are equal
