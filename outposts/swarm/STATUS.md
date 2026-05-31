# Swarm Status Board — mojave advisory execution

Last updated: 2026-05-30

## Stream Status

| Stream | Peer | Status | Blocked By | Current Task |
|--------|------|--------|------------|--------------|
| A: Statistical Correctness | 1 | READY | — | A1: Fix AnytimeMonitor sigma |
| B: Audit Chain Trust | 2 | READY | — | B1: Retire Python audit writer |
| C: QMU + Defense | 3 | BLOCKED | A | Lit review (QMU papers) |
| D: GSA + G-Theory | 4 | PARTIAL | A (D5-D6 only) | D1: Sobol code dedup |
| E: IRR + Measurement | 5 | READY | — | E1: Wire bootstrap CIs |

## Signals

- [ ] STREAM_A_COMPLETE — unblocks C and D (D5-D6)
- [ ] STREAM_B_COMPLETE
- [ ] STREAM_C_COMPLETE
- [ ] STREAM_D_COMPLETE
- [ ] STREAM_E_COMPLETE

## Notes

Peers: update your status line when you start/finish a task.
Controller: merges streams to master after review.
