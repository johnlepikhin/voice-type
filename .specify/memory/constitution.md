<!--
  Sync Impact Report
  ===================
  Version change: N/A → 1.0.0 (initial ratification)
  Modified principles: N/A (initial)
  Added sections:
    - Core Principles (5 principles)
    - Technology Stack & Constraints
    - Development Workflow & Quality Gates
    - Governance
  Removed sections: N/A
  Templates requiring updates:
    - .specify/templates/plan-template.md ✅ compatible (Constitution Check section exists)
    - .specify/templates/spec-template.md ✅ compatible (requirements use MUST/SHOULD)
    - .specify/templates/tasks-template.md ✅ compatible (test-first workflow supported)
  Follow-up TODOs: None
-->

# Voice Type Constitution

## Core Principles

### I. Algebraic Type Design

All domain state MUST be modeled using Rust's algebraic type system
(enums, structs, newtypes) to make illegal states unrepresentable.

- Every domain concept MUST have a dedicated type; raw primitives
  (`String`, `i32`, `bool`) MUST NOT appear in public API signatures
  without a newtype wrapper when they carry domain meaning.
- Enum variants MUST exhaustively represent all valid states of a
  domain entity. `Option` and `Result` MUST be used instead of
  sentinel values or panics.
- State machines MUST be encoded as enums with transitions enforced
  at compile time (typestate pattern) when the state space is finite
  and well-defined.
- `#[non_exhaustive]` MUST be applied to public enums that may gain
  variants in future versions.

**Rationale**: The compiler becomes the first line of defense against
logic errors. If a state cannot be constructed, a bug cannot exist.

### II. Extensibility Through Composition

The system MUST be extended through trait-based composition and
generics, never through inheritance-like hierarchies or downcasting.

- Public extension points MUST be defined as traits with default
  method implementations where sensible.
- Concrete types MUST be behind trait boundaries at module edges so
  that implementations can be swapped without changing callers.
- Generic parameters MUST use trait bounds (`where` clauses) to
  document capabilities required by each function.
- Feature flags (`Cargo.toml` features) MUST be used for optional
  functionality; conditional compilation (`#[cfg]`) MUST NOT leak
  into core domain logic.

**Rationale**: Composition yields flat dependency graphs, easier
testing, and lower coupling—critical for a GTK application that must
separate UI from domain.

### III. Test-Driven Quality

Tests MUST be written before or alongside implementation. No merge
is permitted when test coverage for new public API surface is absent.

- Unit tests MUST live in `#[cfg(test)] mod tests` inside the module
  they verify.
- Integration tests MUST live in `tests/` and exercise public API
  contracts without relying on internal details.
- Property-based tests (via `proptest` or `quickcheck`) MUST be used
  for any function operating on numeric ranges, collections, or
  serialization round-trips.
- GTK widget behavior MUST be tested through programmatic signal
  emission and state inspection; manual-only verification is
  NOT acceptable for regression-critical flows.
- `#[should_panic]` tests MUST include `expected` string to avoid
  masking unrelated panics.

**Rationale**: Tests are executable specifications. Property-based
testing finds edge cases humans overlook. Automated GTK tests
prevent UI regressions.

### IV. GTK Architecture Discipline

UI code MUST be strictly separated from domain logic via a
unidirectional data-flow boundary.

- Domain logic MUST reside in a library crate (`src/lib.rs` subtree)
  with zero GTK dependencies.
- The GTK application crate MUST depend on the library crate and
  act as a thin adapter translating domain types into widgets.
- Widget construction MUST use GTK's composite template pattern or
  builder API; manual widget tree assembly MUST NOT exceed 30 lines
  without extraction into a dedicated component.
- Asynchronous operations (file I/O, network) MUST use `glib::MainContext`
  futures or channels; blocking the main loop is forbidden.
- CSS theming MUST be loaded from external `.css` files, not
  inline strings, to enable user customization.

**Rationale**: Separating domain from UI enables headless testing,
future UI toolkit migration, and keeps the GTK layer focused on
presentation.

### V. Code Quality & Safety

All code MUST pass `clippy` with `#![warn(clippy::all, clippy::pedantic)]`
and `rustfmt` with project-standard configuration.

- `unsafe` code MUST NOT be used unless wrapping a C FFI boundary
  (e.g., GTK bindings). Every `unsafe` block MUST have a `// SAFETY:`
  comment explaining the invariant upheld.
- Error handling MUST use `thiserror` for library errors and `anyhow`
  (or equivalent) only in the application binary entry point.
  `unwrap()` / `expect()` are forbidden outside tests and
  infallible contexts proven by preceding checks.
- Public items MUST have `///` doc comments. Internal items SHOULD
  have comments only when the logic is non-obvious.
- Dependencies MUST be audited (`cargo audit`) before addition.
  Direct dependencies SHOULD be minimized; `cargo deny` MUST
  enforce license and duplicate-crate policies.

**Rationale**: Automated enforcement removes style debates from
reviews and catches common mistakes before they reach production.

## Technology Stack & Constraints

- **Language**: Rust (latest stable, minimum edition 2021)
- **UI Toolkit**: GTK 4 via `gtk4-rs` bindings
- **Build System**: Cargo, with `cargo-make` or `just` for
  multi-step workflows
- **Testing**: `cargo test`, `proptest` for property-based tests
- **Linting**: `clippy` (pedantic), `rustfmt`
- **Dependency Audit**: `cargo audit`, `cargo deny`
- **Target Platform**: Linux (primary), cross-platform as stretch goal
- **Minimum Supported Rust Version (MSRV)**: MUST be declared in
  `Cargo.toml` `rust-version` field and tested in CI

## Development Workflow & Quality Gates

- Every change MUST pass the following gates before merge:
  1. `cargo fmt -- --check` (zero diff)
  2. `cargo clippy -- -D warnings` (zero warnings)
  3. `cargo test` (all tests green)
  4. `cargo audit` (no known vulnerabilities)
  5. `cargo doc --no-deps` (documentation builds without warnings)
- Commits MUST follow Conventional Commits format
  (`type(scope): description`).
- Feature branches MUST be rebased on `main` before merge;
  merge commits are NOT permitted on `main`.
- Code review MUST verify compliance with all five Core Principles
  before approval.

## Governance

This Constitution is the authoritative source of project standards.
It supersedes informal conventions, ad-hoc decisions, and external
style guides where conflicts arise.

- **Amendments** require: (1) a written proposal referencing the
  principle(s) affected, (2) documented rationale for the change,
  (3) an updated constitution version committed atomically with
  the code changes that necessitated the amendment.
- **Versioning** follows SemVer applied to governance:
  - MAJOR: principle removed or redefined incompatibly
  - MINOR: new principle added or existing one materially expanded
  - PATCH: clarification, typo, or non-semantic wording change
- **Compliance Review**: every PR review MUST include a Constitution
  Check verifying adherence to all applicable principles.
- **Exceptions**: any deviation from a principle MUST be recorded in
  the Complexity Tracking table of the relevant `plan.md` with
  justification and rejected alternatives.

**Version**: 1.0.0 | **Ratified**: 2026-02-20 | **Last Amended**: 2026-02-20
