# Changelog

All notable changes to AIOS will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added
- Project initialized with AI-generated foundation
- x86_64 architecture directory structure
- Core subsystem directories (kernel, drivers, fs, mm, net, ipc, lib)
- Test framework structure (unit, integration, QEMU)
- GitHub Actions workflows for AI auto-merge and AI feature research
- Git commit hooks enforcing AI model/tool attribution
- i18n support planned in roadmap

### Changed

### Deprecated

### Removed

### Fixed

### Security

---

## Conventional Commit Format for AIOS

All commits MUST include AI model and tool information:

```
<type>(<scope>): <description>

Model: <model-name>
Tool: <ai-tool-used>
Prompt: <brief-prompt-summary>
```

**Example:**
```
feat(mm): implement physical memory manager

Model: Claude 4 Opus
Tool: opencode
Prompt: Create bitmap-based physical memory manager for x86_64
```
