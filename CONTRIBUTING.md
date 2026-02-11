# Contributing to PicoFlow

Thank you for your interest in contributing to PicoFlow! This document provides guidelines and instructions for contributing to the project.

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [How Can I Contribute?](#how-can-i-contribute)
3. [Development Setup](#development-setup)
4. [Development Workflow](#development-workflow)
5. [Coding Standards](#coding-standards)
6. [Testing](#testing)
7. [Documentation](#documentation)
8. [Pull Request Process](#pull-request-process)
9. [Issue Reporting](#issue-reporting)
10. [Community](#community)

---

## Code of Conduct

### Our Pledge

We pledge to make participation in PicoFlow a harassment-free experience for everyone, regardless of age, body size, disability, ethnicity, gender identity and expression, level of experience, nationality, personal appearance, race, religion, or sexual identity and orientation.

### Our Standards

**Positive behavior includes:**
- Using welcoming and inclusive language
- Being respectful of differing viewpoints and experiences
- Gracefully accepting constructive criticism
- Focusing on what is best for the community
- Showing empathy towards other community members

**Unacceptable behavior includes:**
- Trolling, insulting/derogatory comments, and personal or political attacks
- Public or private harassment
- Publishing others' private information without explicit permission
- Other conduct which could reasonably be considered inappropriate

### Enforcement

Instances of abusive, harassing, or otherwise unacceptable behavior may be reported by opening an issue or contacting the project maintainers. All complaints will be reviewed and investigated promptly and fairly.

---

## How Can I Contribute?

### Reporting Bugs

Found a bug? Please help us fix it!

1. **Check existing issues:** Search [GitHub Issues](https://github.com/zoza1982/picoflow/issues) to see if it's already reported
2. **Create a new issue** with:
   - Clear, descriptive title
   - Steps to reproduce the bug
   - Expected behavior
   - Actual behavior
   - Environment details (OS, PicoFlow version, hardware)
   - Logs and error messages (with `--log-level debug`)
   - Minimal workflow YAML that reproduces the issue

**Bug report template:**

```markdown
**Description:**
Brief description of the bug

**Steps to Reproduce:**
1. Step 1
2. Step 2
3. Step 3

**Expected Behavior:**
What should happen

**Actual Behavior:**
What actually happens

**Environment:**
- PicoFlow version: 0.1.1
- OS: Raspberry Pi OS (Debian 12)
- Hardware: Raspberry Pi Zero 2 W
- Rust version: 1.75.0

**Logs:**
```
[paste debug logs here]
```

**Workflow YAML:**
```yaml
[paste minimal workflow that reproduces issue]
```
```

### Suggesting Features

Have an idea for a new feature?

1. **Check the roadmap:** See [PRD.md](PRD.md) for planned features
2. **Search existing issues** for similar suggestions
3. **Open a GitHub Discussion** to discuss the idea first
4. **Create a feature request issue** with:
   - Clear description of the feature
   - Use case and motivation
   - Proposed implementation (if you have ideas)
   - Any alternatives considered

**Feature request template:**

```markdown
**Feature Description:**
Clear description of the proposed feature

**Use Case:**
Why is this feature needed? What problem does it solve?

**Proposed Implementation:**
(Optional) How might this be implemented?

**Alternatives Considered:**
(Optional) Other ways to solve this problem

**Additional Context:**
Any other relevant information
```

### Contributing Code

Ready to write some code?

1. **Start with small contributions:** Fix typos, improve docs, add tests
2. **Discuss major changes first:** Open an issue or discussion before large PRs
3. **Follow the workflow:** See [Development Workflow](#development-workflow)
4. **Write tests:** All new features need tests
5. **Update documentation:** Keep docs in sync with code changes

### Improving Documentation

Documentation improvements are always welcome!

- Fix typos and grammatical errors
- Clarify confusing explanations
- Add examples and tutorials
- Improve API documentation
- Translate documentation (future)

### Helping Others

Join the community and help others:

- Answer questions in GitHub Discussions
- Review pull requests
- Test beta features
- Share your workflows and use cases
- Write blog posts or tutorials

---

## Development Setup

### Prerequisites

- **Rust:** 1.83 or newer (stable)
- **Git:** For version control
- **SQLite:** Usually pre-installed on Linux/macOS
- **SSH:** For testing SSH executor
- **Optional:**
  - Docker (for cross-compilation)
  - `cross` tool for cross-compilation
  - `cargo-tarpaulin` for coverage reports

### Clone the Repository

```bash
git clone https://github.com/zoza1982/picoflow.git
cd picoflow
```

### Install Rust (if needed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Install Development Tools

```bash
# Formatting
rustup component add rustfmt

# Linting
rustup component add clippy

# Code coverage (optional)
cargo install cargo-tarpaulin

# Cross-compilation (optional)
cargo install cross
```

### Build the Project

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Check compilation without building
cargo check
```

### Run Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# With output
cargo test -- --nocapture

# Specific test
cargo test test_dag_validation
```

### Run PicoFlow Locally

```bash
# Run from source
cargo run -- run examples/backup.yaml

# Or use built binary
./target/debug/picoflow run examples/backup.yaml
```

---

## Development Workflow

### 1. Find or Create an Issue

- Browse [open issues](https://github.com/zoza1982/picoflow/issues)
- Comment on the issue to claim it
- If no issue exists, create one first
- Wait for maintainer feedback before starting large changes

### 2. Fork and Branch

```bash
# Fork the repository on GitHub, then:

# Clone your fork
git clone https://github.com/YOUR_USERNAME/picoflow.git
cd picoflow

# Add upstream remote
git remote add upstream https://github.com/zoza1982/picoflow.git

# Create a feature branch
git checkout -b feature/my-feature-name
# Or for bug fixes:
git checkout -b fix/issue-123-description
```

**Branch naming conventions:**
- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation changes
- `refactor/description` - Code refactoring
- `perf/description` - Performance improvements
- `test/description` - Test additions/improvements

### 3. Make Changes

```bash
# Edit code
vim src/executor/shell.rs

# Format code
cargo fmt --all

# Check for issues
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test

# Build
cargo build
```

### 4. Commit Changes

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```bash
git add .
git commit -m "feat(executor): add Docker executor support"
```

**Commit message format:**

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style (formatting, missing semicolons, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `build`: Build system changes
- `ci`: CI/CD changes
- `chore`: Maintenance tasks

**Example commits:**

```bash
# Simple feature
git commit -m "feat(cli): add workflow list command"

# Bug fix with details
git commit -m "fix(ssh): resolve authentication timeout issue

- Increase connection timeout to 30s
- Add retry logic for transient failures
- Improve error messages

Fixes #123"

# Documentation
git commit -m "docs(user-guide): add SSH troubleshooting section"

# Performance improvement
git commit -m "perf(dag): optimize topological sort algorithm

Reduces parsing time by 40% for workflows with 100+ tasks"
```

### 5. Push and Create Pull Request

```bash
# Push to your fork
git push origin feature/my-feature-name

# Go to GitHub and create Pull Request
```

---

## Coding Standards

### Rust Style Guide

**Follow Rust conventions:**

```rust
// Good: Snake case for functions and variables
fn execute_workflow() -> Result<()> {
    let task_name = "backup";
}

// Good: CamelCase for types
struct WorkflowExecution {
    id: Uuid,
    status: TaskStatus,
}

// Good: SCREAMING_SNAKE_CASE for constants
const MAX_RETRY_COUNT: u32 = 10;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
```

**Use descriptive names:**

```rust
// Bad
fn exec(t: &T) -> R {}

// Good
fn execute_task(task: &Task) -> Result<TaskExecution> {}
```

**Error handling:**

```rust
// Use anyhow::Result for application code
use anyhow::{Context, Result};

fn load_workflow(path: &Path) -> Result<Workflow> {
    let content = fs::read_to_string(path)
        .context("Failed to read workflow file")?;

    let workflow: Workflow = serde_yaml::from_str(&content)
        .context("Failed to parse workflow YAML")?;

    Ok(workflow)
}

// Use thiserror for library errors
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Timeout after {0}s")]
    Timeout(u64),

    #[error("SSH connection failed: {0}")]
    SshError(#[from] ssh2::Error),
}
```

**Documentation:**

```rust
/// Executes a workflow by running tasks in topological order.
///
/// This function validates the workflow DAG, resolves dependencies,
/// and executes tasks in parallel where possible.
///
/// # Arguments
///
/// * `workflow` - The workflow to execute
/// * `config` - Execution configuration
///
/// # Returns
///
/// Returns `Ok(WorkflowExecution)` on success, or an error if:
/// - The workflow DAG is invalid (contains cycles)
/// - Any task fails and has no retry attempts remaining
/// - The workflow is interrupted by a signal
///
/// # Example
///
/// ```
/// let workflow = Workflow::from_file("backup.yaml")?;
/// let config = ExecutionConfig::default();
/// let result = execute_workflow(&workflow, &config)?;
/// ```
pub fn execute_workflow(
    workflow: &Workflow,
    config: &ExecutionConfig,
) -> Result<WorkflowExecution> {
    // Implementation
}
```

**Async code:**

```rust
// Use async-trait for trait methods
use async_trait::async_trait;

#[async_trait]
pub trait Executor {
    async fn execute(&self, task: &Task) -> Result<TaskExecution>;
}

// Avoid blocking in async contexts
async fn fetch_data() -> Result<String> {
    // Good: Use async HTTP client
    let response = reqwest::get("https://api.example.com/data").await?;
    let body = response.text().await?;
    Ok(body)

    // Bad: Don't use blocking std::fs
    // let content = std::fs::read_to_string("file.txt")?;

    // Good: Use tokio::fs instead
    // let content = tokio::fs::read_to_string("file.txt").await?;
}
```

### Code Quality

**Run these before committing:**

```bash
# Format code
cargo fmt --all

# Check for issues
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test

# Check documentation
cargo doc --no-deps --open
```

**Clippy configuration:**

We use strict Clippy lints. Fix all warnings before submitting PR.

```toml
# In Cargo.toml
[lints.clippy]
all = "warn"
pedantic = "warn"
```

**Performance considerations:**

- Minimize memory allocations
- Use `&str` instead of `String` where possible
- Prefer iteration over indexing
- Use `Arc` for shared state in async contexts
- Profile critical paths

---

## Testing

### Test Organization

```
tests/
├── integration/        # Integration tests
│   ├── cli_tests.rs
│   ├── workflow_tests.rs
│   └── executor_tests.rs
└── fixtures/          # Test data
    ├── workflows/
    └── scripts/

src/
└── executor/
    └── shell.rs
        // Unit tests in same file
        #[cfg(test)]
        mod tests { }
```

### Writing Unit Tests

```rust
// In src/executor/shell.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_executor_success() {
        let config = ShellConfig {
            command: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            ..Default::default()
        };

        let executor = ShellExecutor::new(config);
        let result = executor.execute().await.unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));
    }

    #[test]
    fn test_shell_executor_command_not_found() {
        let config = ShellConfig {
            command: "/nonexistent/command".to_string(),
            ..Default::default()
        };

        let executor = ShellExecutor::new(config);
        let result = executor.execute().await;

        assert!(result.is_err());
    }
}
```

### Writing Integration Tests

```rust
// In tests/integration/workflow_tests.rs

use picoflow::{Workflow, ExecutionConfig};
use std::path::Path;

#[tokio::test]
async fn test_simple_workflow_execution() {
    let workflow = Workflow::from_file(
        Path::new("tests/fixtures/workflows/simple.yaml")
    ).unwrap();

    let config = ExecutionConfig::default();
    let result = workflow.execute(&config).await.unwrap();

    assert_eq!(result.status, WorkflowStatus::Success);
    assert_eq!(result.total_tasks, 3);
    assert_eq!(result.successful_tasks, 3);
}
```

### Test Coverage

We aim for >80% test coverage.

```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir target/coverage

# Open report
open target/coverage/index.html
```

### Testing on Target Hardware

For Raspberry Pi testing:

```bash
# Cross-compile for ARM
cross build --release --target armv7-unknown-linux-gnueabihf

# Copy to Pi
scp target/armv7-unknown-linux-gnueabihf/release/picoflow pi@raspberrypi:~/

# SSH to Pi and test
ssh pi@raspberrypi
./picoflow run workflow.yaml
```

---

## Documentation

### Types of Documentation

1. **Code documentation** (rustdoc)
2. **User guide** (docs/user-guide.md)
3. **API reference** (docs/api-reference.md)
4. **Examples** (examples/)
5. **README** (README.md)

### Writing Rustdoc Comments

```rust
/// Brief one-line description.
///
/// More detailed explanation of what this function does,
/// including any important behavior, edge cases, or gotchas.
///
/// # Arguments
///
/// * `workflow` - Description of workflow parameter
/// * `config` - Description of config parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// This function will return an error if:
/// - Condition 1
/// - Condition 2
///
/// # Examples
///
/// ```
/// use picoflow::Workflow;
///
/// let workflow = Workflow::from_file("backup.yaml")?;
/// workflow.validate()?;
/// ```
///
/// # Panics
///
/// This function panics if (only if it can panic)
///
/// # Safety
///
/// (Only for unsafe functions)
pub fn execute_workflow(workflow: &Workflow, config: &Config) -> Result<()> {
    // Implementation
}
```

### Updating User Documentation

When adding features, update:

- **docs/user-guide.md** - User-facing documentation
- **docs/api-reference.md** - YAML schema and CLI reference
- **examples/** - Add example workflows
- **README.md** - Update feature list if major feature

### Example Workflows

Add realistic examples to `examples/`:

```yaml
# examples/new_feature.yaml
name: new-feature-example
description: "Demonstrates the new feature"

# ... complete, working example
```

---

## Pull Request Process

### Before Submitting

**Checklist:**

- [ ] Code compiles without warnings
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Clippy checks pass (`cargo clippy -- -D warnings`)
- [ ] Documentation is updated
- [ ] Examples added/updated (if applicable)
- [ ] Commit messages follow conventions
- [ ] Branch is up to date with main

```bash
# Update your branch with latest main
git fetch upstream
git rebase upstream/main
```

### Creating the Pull Request

1. **Push to your fork:**
   ```bash
   git push origin feature/my-feature
   ```

2. **Open PR on GitHub**

3. **Fill out PR template:**

```markdown
## Description

Brief description of changes

## Related Issue

Closes #123

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Testing

Describe testing performed:
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Tested on Raspberry Pi Zero 2 W
- [ ] Tested on x86_64 Linux

## Checklist

- [ ] Code compiles without warnings
- [ ] Tests pass
- [ ] Code formatted (cargo fmt)
- [ ] Clippy checks pass
- [ ] Documentation updated
- [ ] Examples added/updated (if applicable)

## Screenshots (if applicable)

## Additional Notes

Any additional context
```

### Review Process

1. **Automated checks run:** CI/CD pipeline runs tests and checks
2. **Maintainers review:** Code review by maintainers
3. **Address feedback:** Make requested changes
4. **Approval:** PR approved by maintainer(s)
5. **Merge:** Maintainer merges PR

### Responding to Feedback

```bash
# Make requested changes
vim src/file.rs

# Commit changes
git add .
git commit -m "fix: address review feedback"

# Push to update PR
git push origin feature/my-feature
```

**Tips:**
- Be open to feedback
- Ask questions if feedback is unclear
- Don't take criticism personally
- Learn from the review process

---

## Issue Reporting

### Good Issue Reports

**Include:**

1. **Clear title:** Descriptive, specific
2. **Environment details:**
   - PicoFlow version: `picoflow --version`
   - OS: `uname -a`
   - Hardware: Raspberry Pi model, RAM
3. **Steps to reproduce:**
   - Step-by-step instructions
   - Minimal workflow YAML
   - Commands run
4. **Expected behavior:** What should happen
5. **Actual behavior:** What actually happens
6. **Logs:** Debug logs (`--log-level debug`)
7. **Error messages:** Full error output

### Issue Labels

Issues are tagged with labels:

- `bug`: Something isn't working
- `enhancement`: New feature request
- `documentation`: Documentation improvements
- `good first issue`: Good for newcomers
- `help wanted`: Extra attention needed
- `question`: Further information requested
- `wontfix`: This will not be worked on
- `duplicate`: This issue already exists

### Finding Issues to Work On

**Good for beginners:**
- Issues labeled `good first issue`
- Documentation improvements
- Adding examples
- Writing tests

**Browse issues:**
- [Good first issues](https://github.com/zoza1982/picoflow/labels/good%20first%20issue)
- [Help wanted](https://github.com/zoza1982/picoflow/labels/help%20wanted)
- [All issues](https://github.com/zoza1982/picoflow/issues)

---

## Community

### Communication Channels

- **GitHub Issues:** Bug reports, feature requests
- **GitHub Discussions:** General discussion, questions, ideas
- **Pull Requests:** Code contributions

### Getting Help

- **Documentation:** Start with [docs/](docs/)
- **Examples:** Browse [examples/](examples/)
- **Discussions:** Ask in GitHub Discussions
- **Issues:** Search existing issues

### Stay Updated

- **Watch the repository** on GitHub
- **Star the project** to show support
- **Follow releases** for new versions

### Recognition

Contributors are recognized in:
- [CHANGELOG.md](CHANGELOG.md) - Release notes
- Release announcements
- Project README (major contributors)

---

## Development Tips

### Debugging

```bash
# Run with debug logging
RUST_LOG=debug cargo run -- run workflow.yaml

# Pretty logs for easier reading
cargo run -- --log-format pretty run workflow.yaml

# Rust backtraces
RUST_BACKTRACE=1 cargo run -- run workflow.yaml

# Full backtraces
RUST_BACKTRACE=full cargo run -- run workflow.yaml
```

### Performance Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile execution
cargo flamegraph -- run workflow.yaml

# Open flamegraph.svg in browser
```

### Memory Profiling

```bash
# Install valgrind (Linux)
sudo apt install valgrind

# Check for memory leaks
valgrind --leak-check=full ./target/release/picoflow run workflow.yaml
```

### Cross-Compilation

```bash
# Install cross
cargo install cross

# Build for ARM32 (Pi Zero 2 W)
cross build --release --target armv7-unknown-linux-gnueabihf

# Build for ARM64 (Pi 4/5)
cross build --release --target aarch64-unknown-linux-gnu
```

---

## Questions?

If you have questions about contributing:

1. Check this guide first
2. Search [GitHub Discussions](https://github.com/zoza1982/picoflow/discussions)
3. Ask in a new Discussion thread
4. Maintainers are happy to help!

---

## Thank You!

Thank you for contributing to PicoFlow! Every contribution, no matter how small, helps make PicoFlow better for everyone.

---

**Document Version:** 0.1.1
**Last Updated:** November 12, 2025
**License:** MIT
