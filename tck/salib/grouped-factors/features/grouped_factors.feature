Feature: Grouped factor support — Morris

  Scenario: Ungrouped equals singleton groups (Morris identity)
    Given a 3-factor linear model
    When I run Morris with singleton groups and without groups
    Then the mu_star values are the same within 0.01

  Scenario: Grouped Morris ranks group B higher
    Given a 4-factor linear model f(x) = x1 + x2 + 3*x3 + 3*x4
    And groups: A=[0,1], B=[2,3]
    When I run grouped Morris with R=100
    Then group B mu_star is larger than group A mu_star

  Scenario: Grouped trajectory shape
    Given 4 factors grouped into 2 groups
    When I generate grouped Morris trajectories
    Then each trajectory has 3 points (n_groups + 1)
