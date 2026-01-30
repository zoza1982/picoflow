# homebrew-picoflow

Homebrew tap for [PicoFlow](https://github.com/zoza1982/picoflow) — a lightweight DAG workflow orchestrator for edge devices.

## Installation

```bash
brew tap zoza1982/picoflow
brew install picoflow
```

## Upgrade

```bash
brew upgrade picoflow
```

## Uninstall

```bash
brew uninstall picoflow
brew untap zoza1982/picoflow
```

## Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| macOS | Apple Silicon (ARM64) | Supported |
| macOS | Intel (x86_64) | Supported |
| Linux | x86_64 | Supported (Linuxbrew) |
| Linux | ARM64 | Supported (Linuxbrew) |
| Linux | ARM32 | Supported (Linuxbrew) |

## About PicoFlow

PicoFlow is a Rust-native workflow orchestrator designed for resource-constrained edge devices like the Raspberry Pi Zero 2 W. It provides DAG-based task scheduling with minimal memory footprint (<20MB idle).

- **Repository:** https://github.com/zoza1982/picoflow
- **License:** MIT
