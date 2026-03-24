# Contributing to Capsule OS

Thank you for your interest in contributing to Capsule OS. This project aims to stay clean, minimal, and well-structured. Please follow the guidelines below to maintain consistency and quality.

---

## Philosophy

Capsule OS is designed around:

* Simplicity over complexity
* Clear structure over abstraction
* Performance and portability
* Predictable behavior

Avoid unnecessary dependencies and over-engineering.

---

## Getting Started

### 1. Fork and Clone

```bash
git clone https://github.com/your-username/capsule-os.git
cd capsule-os
```

### 2. Create a Branch

Use descriptive branch names:

```bash
git checkout -b feat/shell-history
git checkout -b fix/theme-loading
git checkout -b refactor/fs-cleanup
```

---

## Development Workflow

### Build

```bash
cargo build
```

### Run

```bash
cargo run
```

### Format

```bash
cargo fmt
```

### Lint

```bash
cargo clippy -- -D warnings
```

---

## Code Guidelines

* Keep functions small and focused
* Prefer explicit logic over clever abstractions
* Avoid unnecessary traits, macros, or generics
* Keep modules well-separated (boot, config, fs, shell, theme)
* Use meaningful names (no abbreviations unless standard)

---

## Project Structure

* `boot/` – startup logic and rendering
* `config/` – configuration system
* `fs/` – virtual filesystem
* `shell/` – command handling and REPL
* `theme/` – theming and styling

Maintain clear boundaries between these modules.

---

## Adding Features

Before adding a feature:

* Check if it aligns with the project philosophy
* Avoid bloating the core runtime
* Keep it modular and optional if possible

Examples of good contributions:

* Shell improvements
* Filesystem enhancements
* Theme system improvements
* Performance optimizations

---

## Pull Requests

### Before submitting:

* Code builds successfully
* `cargo fmt` passes
* `cargo clippy` passes with no warnings
* No unnecessary files or debug code

---

### PR Guidelines

* Keep PRs focused and small
* Provide a clear description of changes
* Reference related issues if applicable

Example:

```text
feat: add basic command history support

- stores last 100 commands
- integrates with shell input loop
```

---

## Commit Style

Use conventional commit prefixes:

* `feat:` new feature
* `fix:` bug fix
* `refactor:` internal changes
* `docs:` documentation
* `chore:` maintenance

---

## Issues

When opening an issue:

* Clearly describe the problem
* Provide steps to reproduce
* Include logs or screenshots if relevant

---

## What Not to Do

* Do not introduce heavy dependencies without justification
* Do not break existing behavior without discussion
* Do not submit large, unrelated changes in one PR

---

## Code of Conduct

Be respectful and constructive. This is a learning-oriented and experimental project.

---

## Final Notes

Capsule OS is an evolving system. Contributions should aim to improve clarity, stability, and usability while preserving the core idea:

> An operating system within an application.

If you are unsure about a change, open an [Issue](https://github.com/greenbugx/CapsuleOS/issues) before implementing it.
