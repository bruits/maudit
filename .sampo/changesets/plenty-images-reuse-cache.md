---
cargo/maudit: patch
---

Fixed images being re-optimized on every build when alternating between `maudit dev` and `maudit build`. The transformed-image cache is now keyed by image content rather than the hashing-strategy-dependent URL fingerprint, so encoded images are reused across development and production builds instead of re-encoded on each switch.
