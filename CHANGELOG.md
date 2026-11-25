### 2026-11-23 / 0.10.1

- feat!: Added aggregate limits for HTTP fields. (#68)
- feat: Added opt-in Host validation for HTTP/1.1 requests. (#69)
- feat: add coverage-guided fuzzing pipeline for security regressions (#37)
- fix: Preserved the GitHub Actions SHA in npm provenance. (#72)
- fix: Accepted framing headers on 204 and 304 responses. (#71)
- fix: Preserved managed input for event readers. (#70)
- fix: dispatch control method callbacks (#45)
- fix: commit header events atomically (#54)
- fix: Guarded parser reentrancy during callback replay. (#67)
- fix: Avoided rescanning retained HTTP lines. (#66)
- fix: bound simple API span storage (#57)
- fix: reject unrepresentable event ranges (#56)
- fix: reject partial start lines at EOF (#44)
- fix: Rejected security-sensitive trailer fields (#55).
- fix: Rejected body framing on CONNECT requests (#53).
- fix: Made the Rust parser input memory-safe (#51).
- fix: preserve WASM content length precision (#48)
- fix: size C++ reference callback buffers (#47)
- fix: use the C ABI for native callbacks (#46)
- fix: Rejected chunked GET and HEAD bodies (#43).
- fix: Merged managed tunnel data cleanup.
- fix: defer fragmented response autodetection (#41)
- fix: preserve valid error descriptions (#40)
- fix: validate transfer encoding syntax (#39)
- fix: discard managed tunnel data
- ci: pin release build inputs (#50)
- ci: fix fuzz artifact directories and Miri toolchain (#62)
- ci: attest immutable release assets (#49)
- docs: document security findings (#38)

### 2026-11-23 / 0.10.0

- feat!: Added aggregate limits for HTTP fields. (#68)
- feat: Added opt-in Host validation for HTTP/1.1 requests. (#69)
- feat: add coverage-guided fuzzing pipeline for security regressions (#37)
- fix: Accepted framing headers on 204 and 304 responses. (#71)
- fix: Preserved managed input for event readers. (#70)
- fix: dispatch control method callbacks (#45)
- fix: commit header events atomically (#54)
- fix: Guarded parser reentrancy during callback replay. (#67)
- fix: Avoided rescanning retained HTTP lines. (#66)
- fix: bound simple API span storage (#57)
- fix: reject unrepresentable event ranges (#56)
- fix: reject partial start lines at EOF (#44)
- fix: Rejected security-sensitive trailer fields (#55).
- fix: Rejected body framing on CONNECT requests (#53).
- fix: Made the Rust parser input memory-safe (#51).
- fix: preserve WASM content length precision (#48)
- fix: size C++ reference callback buffers (#47)
- fix: use the C ABI for native callbacks (#46)
- fix: Rejected chunked GET and HEAD bodies (#43).
- fix: Merged managed tunnel data cleanup.
- fix: defer fragmented response autodetection (#41)
- fix: preserve valid error descriptions (#40)
- fix: validate transfer encoding syntax (#39)
- fix: discard managed tunnel data
- ci: pin release build inputs (#50)
- ci: fix fuzz artifact directories and Miri toolchain (#62)
- ci: attest immutable release assets (#49)
- docs: document security findings (#38)

### 2026-10-05 / 0.9.0

- fix: Correctly drop memory. (#36)
- fix: Fixed OWS stripping. (#35)
- fix: Fixed events buffer handling. (#34)
- fix: Upgrade only on requests or 101 responses. (#33)
- fix: Fixed dealloc. (#32)
- fix: Reversed WASM arguments. (#31)

### 2026-06-25 / 0.8.0

- feat: Added parser fields, events interface and other flags. (#21)

### 2026-06-18 / 0.7.0

- feat: Added parseWithError. (#20)

### 2026-06-07 / 0.6.0

- feat: Added dual package publishing on NPM.

### 2026-06-07 / 0.5.0

- feat: Generate CommonJS files as well.

### 2026-06-01 / 0.4.0

- feat: Added binary parser.
- feat: Rewritten using SIMD. (#14)

### 2026-06-01 / 0.4.0-alpha.6

- feat: Added binary parser.
- feat: Rewritten using SIMD. (#14)

### 2025-04-20 / 0.3.0

- feat: Avoid function calls.

### 2024-04-06 / 0.2.1

- feat: Use TypedArrays instead of Nodejs' Buffer by @MarcoPolo in #5

### 2024-04-06 / 0.2.0

- feat: Various headers and setter fixes.
- chore: Updated dependencies.
- chore: Formatted and linted all code.
- feat: Removed wasm-bindgen.
- chore: Improve website visibility.
- chore: Implemented release automation.

### 2023-09-08 / 0.1.0

- Initial version.
