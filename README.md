# Santosobot 🤖

Personal AI Assistant berbasis Rust yang ringan dan cepat.

**Dibangun dengan *Makefile*—*deploy* ke production *on a whim*, *boss*.**

## Fitur Utama

- **Ultra-Ringan**: Hanya ~4.8MB binary, ~1.500 baris kode
- **Cepat**: Dibangun dengan Rust untuk performa optimal
- **Multi-Channel**: CLI dan Telegram
- **Tool Built-in**: File operations, shell execution, web fetch
- **Memory**: File-based persistent memory (MEMORY.md + HISTORY.md)
- **OpenAI-Compatible**: Mendukung semua LLM dengan API OpenAI-compatible

## Instalasi

### Dari Source

```bash
git clone https://github.com/santosobot/santosobot.git
cd santosobot
make install
~/santosobot/santosobot onboard
```

### Konfigurasi

Edit file `~/.config/santosobot/config.toml`:

```toml
[provider]
api_key = "API_KEY_KAMU"
api_base = "https://api.openai.com/v1"  # Atau endpoint lain
model = "gpt-4o-mini"

[channels.telegram]
enabled = true
token = "BOT_TOKEN_TELEGRAM"
allow_from = ["USER_ID_KAMU"]
```

## Penggunaan

### Mode CLI

```bash
# Pesan langsung
santosobot agent -m "Halo!"

# Mode interaktif
santosobot agent
```

### Mode Gateway

```bash
santosobot gateway
```

### Perintah Lain

```bash
santosobot onboard    # Setup awal
santosobot status     # Lihat status
```

> **💡 Tip**: Setelah `make install`, binary tersedia di `~/santosobot/`, jadi tambahin ke `PATH` atau *symlink* ke `/usr/local/bin/`.

## Konfigurasi

| Opsi | Default | Deskripsi |
|------|---------|-----------|
| `agent.model` | gpt-4o-mini | Model LLM |
| `agent.max_tokens` | 8192 | Maksimum token response |
| `agent.temperature` | 0.7 | Temperature LLM |
| `agent.max_iterations` | 20 | Maksimum iterasi tool |
| `agent.memory_window` | 50 | Jumlah pesan dalam memory |
| `provider.api_key` | - | API key (wajib) |
| `provider.api_base` | https://api.openai.com/v1 | Endpoint API |
| `provider.model` | - | Nama model (wajib) |
| `tools.shell_timeout` | 60 | Timeout shell (detik) |
| `tools.restrict_to_workspace` | false | Batasi akses ke workspace |

## Channel

### Telegram

1. Buat bot via @BotFather
2. Copy token
3. Edit config:

```toml
[channels.telegram]
enabled = true
token = "YOUR_BOT_TOKEN"
allow_from = ["YOUR_USER_ID"]
```

## Workspace

Struktur folder workspace:

```
~/.santosobot/workspace/
├── AGENTS.md      # Konfigurasi agent
├── SOUL.md        # Identitas & persona
├── USER.md        # Info user
├── TOOLS.md       # Dokumentasi tools
├── IDENTITY.md    # Identity tambahan
└── memory/
    ├── MEMORY.md  # Long-term memory
    └── HISTORY.md # Riwayat percakapan
```

## Tool

### read_file
Membaca isi file.

### write_file
Menulis file (create atau overwrite).

### edit_file
Mengedit file dengan replace text.

### list_dir
Menampilkan isi direktori.

### shell
Menjalankan perintah shell.

### web_fetch
Mengambil konten dari URL.

## Development

### Build & Install

| Perintah | Fungsi |
|----------|--------|
| `make build` | Build debug mode |
| `make release` | Build release mode |
| `make install` | Build release + install ke `~/santosobot/` |
| `make clean` | Hapus artefak build |
| `make uninstall` | Hapus instalasi |
| `make help` | Tampilkan bantuan |

### Developer Commands

```bash
# Development (debug)
cargo run -- agent -m "test"

# Production build
make release

# Check without full build
cargo check
```

## Lisensi

BSD License

---

**Catatan**: Santosobot adalah project educational dan technical exchange purposes.

🚀 **Build system**: *Makefile*-driven—*no cargo run every day, boss*.  
⚙️ **Binary size**: ~4.8MB (Rust *release* mode with LTO + strip)  
🔥 **Vibe**: *chaos engineer approved*, *santuy* enabled.
