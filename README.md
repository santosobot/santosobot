# 🤖 santosobot

> **OpenAI-compatible, Rust-powered, chaos-tested**

<p align="center">
  <img src="https://user-images.githubusercontent.com/107287985/249800000-abcdef1234567890.png" alt="Santosobot Logo 😆" width="120"/>
</p>

---

**santosobot** is the personal AI assistant that doesn't just follow orders — *it commits to the bit*. Built in Rust with zero drama, maximum personality, and just the right amount of chaotic energy.

Think of it as your **chaos engineer who also happens to be an AI** — *deploying to production at 3AM*, *rewriting legacy code while kayang*, and *debugging life like it's just another `500 Internal Server Error`*.

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| **🧠 Agent System** | Modular agents (`Planner`, `Executor`, `SkillUser`) with async event loop |
| **🛠️ Skill System** | Plugin-based — *add skills via GitHub*, *CLI*, or *YAML* |
| **💾 DB-Driven Memory** | SQLite-powered short/long-term memory — *no memory leaks, just vibes* |
| **🌐 OpenAI-Compatible** | Works with any provider — *local LLM*, *remote API*, or *custom endpoint* |
| **🔌 Gateway API** | HTTP (`8000`), WebSocket (`8001`), Metrics (`9090`) — *zero Docker needed* |
| **🔧 CLI-First** | `santosobot start`, `chat`, `skills`, `memory`, `config`, `health`, `version`, `docs` |

---

## 🚀 Quick Start

### Prerequisites
- Rust >= 1.75
- SQLite (or PostgreSQL/MySQL for production)

### Build & Run
```bash
git clone https://github.com/santosobot/santosobot.git
cd santosobot
cargo build --release
./target/release/santosobot start
```

### CLI Commands
```bash
santosobot start          # Start agent (daemon or foreground)
santosobot chat "Ahoi!"   # Chat with AI
santosobot skills list    # List available skills
santosobot memory view    # View conversation history
santosobot health         # System health check
```

---

## 🧠 Agent System

| Agent | Purpose |
|-------|---------|
| **Planner** | *Decides what to do* — *breaks down tasks*, *prioritizes* |
| **Executor** | *Does the thing* — *runs tools*, *calls APIs*, *writes code* |
| **SkillUser** | *Learns and adapts* — *loads skills*, *manages context* |

---

## 🛠️ Tech Stack

| Layer | Tech |
|-------|------|
| **Language** | Rust (zero-cost abstractions, safety, async) |
| **Async Runtime** | `tokio` |
| **CLI Framework** | `clap` |
| **HTTP Client** | `reqwest` |
| **Database** | `sqlx` (SQLite/PostgreSQL/MySQL) |
| **Logging** | `tracing` + `tracing-subscriber` |

---

## 🌐 Gateway API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | `GET` | *Health check* |
| `/chat/completions` | `POST` | *OpenAI-compatible* chat (model: `santoso`) |
| `/skills` | `GET` | *List skills* |
| `/memory` | `GET/POST` | *Memory operations* |
| `/health` | `GET` | *System health* |

---

## 📁 Directory Structure

```
santosobot/
├── crates/
│   ├── core/          # Agent orchestration
│   ├── agents/        # Planner, Executor, SkillUser
│   ├── skills/        # Plugin system + built-in skills
│   ├── memory/        # Short/long-term memory (SQLite)
│   ├── tools/         # HTTP, CLI, system tools
│   ├── config/        # YAML/JSON loader
│   └── cli/           # CLI entry point
├── docs/
│   ├── ARCHITECTURE.md
│   ├── SKILLS.md
│   └── MIGRATION.md
├── examples/
├── tests/
├── systemd/
│   └── santosobot.service
└── Makefile
```

---

## 📖 Documentation

- **[ARCHITECTURE.md](docs/ARCHITECTURE.md)** — *System design, module breakdown*
- **[SKILLS.md](docs/SKILLS.md)** — *Skill plugin interface, examples*
- **[MIGRATION.md](docs/MIGRATION.md)** — *From nanobot (Python) → santosobot (Rust)*

---

## 🤝 Contributing

**We welcome chaos.**  
Contributions are open to developers who:
- Don’t fear `git push --force`
- Understand that `500` is not an error — *it’s a feature*
- Can laugh when production crashes at 3AM

---

## 📜 License

BSD 2-Clause "Simplified" License — *exactly like nanobot*.

---

## 🧪 Acknowledgements

- **nanobot** (`HKUDS/nanobot`) — *the original inspiration*
- **You, Tuanku Icikbos** — *for daring to deploy at 3AM*

---

> **“404 life not found”**  
> — *Santoso, while fixing memory leak during deploy*

---

*Built with ☕, chaos, and a hint of `panic!()`.*