.PHONY: build release install clean uninstall help

# Default target
all: build

# Show help
help:
	@echo "Santosobot Makefile"
	@echo ""
	@echo "Usage:"
	@echo "  make build      - Build debug mode"
	@echo "  make release    - Build release mode"
	@echo "  make install    - Build release and install to ~/santosobot/"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make uninstall  - Remove installed files"
	@echo ""
	@echo "Examples:"
	@echo "  make build      # Build debug version"
	@echo "  make release    # Build optimized version"
	@echo "  make install    # Install to home directory"

# Build debug mode
build:
	cargo build

# Build release mode
release:
	cargo build --release

# Build release and install to ~/santosobot/
install: release
	mkdir -p ~/santosobot
	cp -r target/release/santosobot ~/santosobot/
	cp -r README.md ~/santosobot/
	cp -r ~/.config/santosobot/config.toml ~/santosobot/config.toml 2>/dev/null || true
	@echo "Installed to ~/santosobot/"

# Clean build artifacts
clean:
	cargo clean

# Uninstall - remove installed files
uninstall:
	rm -rf ~/santosobot
	@echo "Uninstalled from ~/santosobot/"
