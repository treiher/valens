# Conventions

Cross-cutting decisions for changes to Valens that are not enforced by tooling. For a description of the system itself, see the [Architecture](ARCHITECTURE.md) document.

## Code layout

- Functions are ordered caller before callee (stepdown rule): high-level functions first, followed by the helpers they call.

## Terminology

- UI text uses "training session", never "workout". The REST API and the database schema predate this decision and keep `workout` in routes and model names. The two vocabularies coexist deliberately; new user-facing text always uses "training session".

## Error messages

The mechanisms by which errors are surfaced are described in the [error handling](ARCHITECTURE.md#error-handling) section of the Architecture document.

Error messages are written as sentence fragments: lowercase unless the first word is an acronym, no leading article, no terminating period, so that they read naturally after a prefix, as in "failed to delete rotation: rotation is used in the schedule". This covers the `Display` strings of domain error enums, the `details` strings of backend responses and the conflict reasons of the domain service.

The same message is also shown on its own, as in a field error, and is capitalized where it is displayed. Messages printed by the CLI are the exception, since nothing capitalizes them: they are written capitalized.

When the same condition is checked in both the domain service and the backend, both sides use the byte-identical fragment so the surfaced message is the same regardless of which layer rejects the operation.

## Comments

- The default is no comment. One is added only when the *why* is non-obvious, such as a hidden constraint, a workaround, or an invariant. Comments do not restate *what* the code does.
- Comments in `domain` describe behavior and invariants. They never name UI callers, motivating scenarios, or other layers.
- Doc comments outside `domain` may describe how a module collaborates with its direct dependencies; module-level (`//!`) docs are the place for it. They do not enumerate the callers of an item, which goes stale as callers are added.
- Identifiers of other modules named in doc comments are written as intra-doc links, so a rename or removal is caught by `cargo doc`.
- A comment ends with a period only if it is a grammatically complete sentence. Short fragments and labels (`// Capture scroll position`, `// Brzycki`) take no terminating period.
- Identifiers in comments are wrapped in backticks: parameter names, local variables, function names, type names, and enum variants (`None`, `Some`, `Ok`, `Err`). Rule of thumb: backtick anything a reader could `grep` for. English nouns, math expressions, and numeric literals stay plain. Clippy's `doc_markdown` lint enforces this for Rust `///` comments; `//` comments, Python comments, and docstrings rely on convention.
- Prose in comments contains no em-dashes.

## Tests

- Coverage is a floor, not a goal: tests assert invariants and observable behavior. Tests that merely restate the implementation or exist only to satisfy coverage are not added.
- End-to-end tests locate elements by test-id, not by styling. Test-ids are never used in tests directly; they are encapsulated in the page objects under [`tests/e2e/pages`](../tests/e2e/pages), which are introduced or extended as needed.
- End-to-end tests assert element state through data attributes, not through CSS classes. A state that tests need to observe, such as the drop state during a drag or the severity of a log entry, is rendered as a `data-*` attribute, so that restyling does not break the tests.
- End-to-end tests of drag & drop use `mouse.move` instead of `hover` during drags, and scroll the drag handle and the targets to the center of the viewport first, avoiding auto-scroll artifacts in Playwright. A drag is activated by a movement on the handle itself, so that a release without an intermediate hover drops on the source instead of on whatever element the pointer happens to be over.
- End-to-end tests run against each browser in the form factor its users have: Chromium and WebKit with a phone device profile, Firefox on the desktop, as Playwright does not support device emulation there (`make test-e2e DEVICE="Pixel 7"`, `make test-e2e BROWSER=firefox`, `make test-e2e BROWSER=webkit DEVICE="iPhone 15"`). Tests that depend on the Chrome DevTools Protocol, like touch input and the virtual authenticator, are marked `chromium_only` and skipped on other browsers. This set of browsers is what [the README](../README.md#features) states as supported, so extending or reducing it changes both.

## Intentional duplication

- The DTOs in [`rest.rs`](../crates/storage/src/rest.rs) and [`indexed_db.rs`](../crates/storage/src/indexed_db.rs) are kept separate even where they currently coincide. Each adapter owns its serialization format (wire format of the REST API, persisted browser data) and must be able to evolve it independently, so the duplication is intentional and must not be removed.
- The backend validates every constraint the domain validates, even though the frontend enforces them as well, so that clients bypassing the frontend cannot store invalid data. Where this requires mirrored constants or bounds, they are kept in sync by hand and carry a comment naming the domain definition they mirror. Where the backend is deliberately stricter than the domain, the comment states so. Changes to such domain definitions include the corresponding backend update.
- The mirrored values are collected in [`limits.py`](../valens/limits.py). The check constraints in [`models.py`](../valens/models.py) are the exception: they are versioned schema and keep their literals.
- The domain represents an unset numeric value as zero, the backend as `NULL`. Nullable columns therefore reject 0, while non-nullable columns store 0 as the unset value. This asymmetry follows from the column definition and needs no per-site comment.

## Changelog

- Entries are written for users: user-facing language ("screen" rather than "viewport") and one noun phrase per bullet that reads as the object of the section verb ("Added ...", "Changed ...").
- Changes affecting several pages get one bullet per page, even if the text repeats. A change to a shared element that behaves identically wherever it appears gets a single bullet naming that element instead, with a page-specific bullet added only where the behavior differs.
- The "Unreleased" section describes the net change since the last release. Intermediate states that are added and reworked within the same cycle are not mentioned.
