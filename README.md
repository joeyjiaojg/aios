# AIOS

> **An entirely AI-generated operating system for x86_64**

[![AI Auto-Merge](https://github.com/joeyjiaojg/aios/actions/workflows/ai-auto-merge.yml/badge.svg)](https://github.com/joeyjiaojg/aios/actions/workflows/ai-auto-merge.yml)
[![AI Feature Research](https://github.com/joeyjiaojg/aios/actions/workflows/ai-research.yml/badge.svg)](https://github.com/joeyjiaojg/aios/actions/workflows/ai-research.yml)

## What is AIOS?

AIOS is an operating system where **every line of code is written by AI models**. No human writes kernel code here — AI handles everything from the bootloader to the filesystem.

- **Architecture**: x86_64 (more to come)
- **Language**: Rust (memory-safe by design)
- **Testing**: QEMU-based simulation + comprehensive unit/integration tests
- **Development**: AI-driven with automated review and feature research

## Project Architecture

```mermaid
flowchart TB
    subgraph Hardware
        CPU["x86_64 CPU"]
        RAM["Physical Memory"]
        DISK["Disk/SSD"]
    end

    subgraph Boot["Boot Layer"]
        BOOT["Bootloader<br/>(GRUB/Multiboot)"]
    end

    subgraph Kernel["Kernel Core"]
        GDT["GDT<br/>Global Descriptor Table"]
        IDT["IDT<br/>Interrupt Descriptor Table"]
        VMM["VMM<br/>Virtual Memory Manager"]
        PIT["PIT<br/>Timer Interrupt"]
        PIC["PIC<br/>Interrupt Controller"]
    end

    subgraph Subsystems["Kernel Subsystems"]
        TASK["Task Manager<br/>Scheduler"]
        PROC["Process Manager"]
        SYSCALL["Syscall Interface"]
        VFS["VFS<br/>Virtual Filesystem"]
        EXT2["ext2<br/>Filesystem"]
        DEV["Device Drivers"]
        NET["Network Stack"]
    end

    subgraph Memory["Memory Management"]
        ALLOC["Allocator<br/>Heap Manager"]
        PAGE["Paging<br/>Page Tables"]
    end

    subgraph Shell["User Interface"]
        SHELL["Shell"]
        SERIAL["Serial Console"]
        VGA["VGA Display"]
    end

    BOOT --> GDT
    BOOT --> IDT
    BOOT --> RAM

    GDT --> KERNEL
    IDT --> KERNEL
    VMM --> KERNEL

    KERNEL --> Subsystems
    KERNEL --> Memory
    KERNEL --> Shell

    PIT --> CPU
    PIC --> CPU
    CPU --> RAM

    RAM --> ALLOC
    ALLOC --> Subsystems

    SHELL --> SYSCALL
    SYSCALL --> TASK
    TASK --> PROC
```

## Development Workflow

```mermaid
flowchart LR
    subgraph Issue["Issue Creation"]
        ISSUE["New Issue<br/>#N"]
    end

    subgraph Branch["Branch Development"]
        FEAT["feat/auto-issue-N<br/>or human branch"]
    end

    subgraph PR["Pull Request"]
        PR["PR Created<br/>#M"]
    end

    subgraph CI["CI Pipeline"]
        FMT["make fmt"]
        CLIPPY["make clippy"]
        TEST["make test-unit"]
        AI_REVIEW["AI Review"]
    end

    subgraph Decision["Decision"]
        APPROVED["✅ APPROVED"]
        REJECTED["❌ REJECTED"]
    end

    ISSUE --> FEAT
    FEAT --> PR
    PR --> FMT
    FMT --> CLIPPY
    CLIPPY --> TEST
    TEST --> AI_REVIEW

    AI_REVIEW --> APPROVED
    AI_REVIEW --> REJECTED

    REJECTED -->|Fix & Push| FEAT
    APPROVED --> MASTER["✅ Merge to master"]
```

## Self-Evolution Workflow

```mermaid
flowchart TB
    subgraph Schedule["Every 30 Minutes"]
        SCHED["Self-Evolve Trigger"]
    end

    subgraph Find["Find Issue"]
        CHECK1["No labels?"]
        CHECK2["No comments?"]
        CHECK3["No existing PR?"]
        CHECK4["No existing branch?"]
        FOUND["Issue Found<br/>#N"]
        SKIP["Skip - Human handling"]
    end

    subgraph Process["Process Issue"]
        CREATE["Create branch<br/>feat/auto-issue-N"]
        GENERATE["Generate code<br/>with OpenCode"]
        COMMIT["Commit changes"]
        PUSH["Push branch"]
        PR_CREATE["Create PR"]
    end

    subgraph Merge["Auto-Merge"]
        REVIEW["AI Review"]
        CHECK5["APPROVED?"]
        MERGE["Merge to master"]
        CLOSE["Close Issue"]
    end

    SCHED --> CHECK1
    CHECK1 -->|No| CHECK2
    CHECK2 -->|No| CHECK3
    CHECK3 -->|No| CHECK4
    CHECK4 -->|No| FOUND
    CHECK4 -->|Yes| SKIP
    CHECK1 -->|Yes| SKIP
    CHECK2 -->|Yes| SKIP
    CHECK3 -->|Yes| SKIP

    FOUND --> CREATE
    CREATE --> GENERATE
    GENERATE --> COMMIT
    COMMIT --> PUSH
    PUSH --> PR_CREATE

    PR_CREATE --> REVIEW
    REVIEW --> CHECK5
    CHECK5 -->|Yes| MERGE
    CHECK5 -->|No| SKIP
    MERGE --> CLOSE
```

## PR Review Process

```mermaid
flowchart TB
    subgraph Submit["PR Submitted"]
        PR["Pull Request #M"]
    end

    subgraph Review["AI Code Review"]
        MEMORY["Memory Safety Check<br/>unsafe blocks justified?"]
        ARCH["Architecture Check<br/>x86_64 correctness?"]
        NOSTD["no_std Check<br/>No Vec/String?"]
        SECRETS["Secrets Check<br/>No hardcoded keys?"]
        TESTS["Test Coverage<br/>Minimum 10 tests?"]
        COMMIT["Commit Format<br/>Model/Tool/Prompt?"]
    end

    subgraph Result["Result"]
        REJECT["REJECTED<br/>Fix issues and push"]
        APPROVE["APPROVED<br/>Auto-merge"]
    end

    PR --> MEMORY
    MEMORY --> ARCH
    ARCH --> NOSTD
    NOSTD --> SECRETS
    SECRETS --> TESTS
    TESTS --> COMMIT
    COMMIT --> APPROVE
    COMMIT --> REJECT

    REJECT -->|Push fix| PR
```

## Quick Start

```bash
# Clone
git clone https://github.com/joeyjiaojg/aios.git
cd aios

# Build
make build

# Run in QEMU
make run

# Run tests
make test
```

## Project Structure

| Directory | Description |
|-----------|-------------|
| `docs/` | Roadmap, features, changelog, i18n documentation |
| `src/` | Source code (kernel, drivers, fs, mm, net, ipc, lib) |
| `test/` | Unit, integration, and QEMU tests |
| `.github/workflows/` | AI auto-merge, self-evolve, auto-rebase pipelines |

## Documentation

- [Roadmap](docs/ROADMAP.md)
- [Features](docs/FEATURES.md)
- [Debug Flag](docs/DEBUG.md)
- [Changelog](docs/CHANGELOG.md)
- [Internationalization](docs/I18N.md)
- [Funding](docs/FUNDING.md)

## Funding

AIOS requires significant AI API tokens for code generation. Your sponsorship directly funds:

- AI model API access (Claude, GPT-4, etc.)
- Compute for testing and research
- Model diversity for different subsystems

👉 [**Sponsor @joeyjiaojg**](https://github.com/sponsors/joeyjiaojg)

## AI Commit Convention

All commits include AI generation metadata:

```
feat(mm): implement physical memory manager

Model: Claude 4 Opus
Tool: opencode
Prompt: Create bitmap-based physical memory manager for x86_64
```

## License

MIT

---

> Built by AI, for the future. 🤖