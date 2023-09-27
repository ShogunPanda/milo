TARGET:= $(shell rustc -vV | sed -n 's|host: ||p')
COMMON_OPTIONS := -Z unstable-options --target ${TARGET}

.PHONY: all release debug clean

all: debug release

debug: dist/reference-debug

release: dist/reference-release

clean: 
	rm -rf dist target

dist/reference-debug: src/main.rs
	cargo build ${COMMON_OPTIONS} --out-dir dist
	cp dist/milo-reference-rs dist/reference-debug

dist/reference-release: src/main.rs
	cargo build ${COMMON_OPTIONS} --out-dir dist --release
	cp dist/milo-reference-rs dist/reference-release
