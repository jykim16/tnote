INSTALL_PATH ?= $(HOME)/bin/tnote

.PHONY: build install clean test integration-test

build:
	cargo build --release

install: build
	cp target/release/tnote $(INSTALL_PATH)
	@echo "Installed to $(INSTALL_PATH)"

clean:
	cargo clean

test:
	cargo test

integration-test:
	docker build -t tnote-integration -f tests/integration/Dockerfile .
	docker run --rm tnote-integration
