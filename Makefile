INSTALL_PATH ?= $(HOME)/bin/tnote

.PHONY: build install clean

build:
	cargo build --release

install: build
	cp target/release/tnote $(INSTALL_PATH)
	@echo "Installed to $(INSTALL_PATH)"

clean:
	cargo clean
