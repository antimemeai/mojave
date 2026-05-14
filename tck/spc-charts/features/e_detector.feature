Feature: E-detector — Shin, Ramdas, Rinaldo 2023

  Scenario: E-process starts at 1
    Given an e-detector with alpha=0.05
    Then the initial e_process is 1.0

  Scenario: E-detector signals when M_t >= 1/alpha
    Given an e-detector with alpha=0.05 (threshold=20)
    When I feed a sustained shift until M_t >= 20
    Then OutOfControl is signaled

  Scenario: E-process floor is 1 (growing window)
    Given an e-detector with growing window
    When I feed observations that would shrink the process
    Then e_process is always >= 1.0

  Scenario: False alarm rate is at most alpha
    Given an e-detector with alpha=0.05
    When I simulate 10000 in-control sequences of length 500
    Then the false alarm rate is <= 0.06
