.PHONY: release debug clean

release: target/release/libmilo.a dist/milo.h 
	cp target/aarch64-apple-darwin/release/libmilo.a dist/libmilo.a

debug: target/debug/libmilo.a dist/milo.h 
	cp target/aarch64-apple-darwin/debug/libmilo.a dist/libmilo.a

target/release/libmilo.a: src/lib.rs
	cargo build -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --release --target aarch64-apple-darwin

target/debug/libmilo.a: src/lib.rs
	cargo build --target aarch64-apple-darwin

dist/milo.h:
	cbindgen --config cbindgen.toml --output dist/milo.h
	node ./tools/export_version/main.mjs
	node ./tools/export_constants/main.mjs
	
clean: 
	rm -rf dist target