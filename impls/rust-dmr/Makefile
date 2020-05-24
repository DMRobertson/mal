SOURCES = $(wildcard src/*) $(wildcard src/bin/*) Cargo.toml Cargo.lock

target/debug/%: $(SOURCES)
	cargo build --bin $(*F)

target/release/%: $(SOURCES)
	cargo build --release --bin $(*F)

