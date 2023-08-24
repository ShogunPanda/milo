.PHONY: release debug clean

release: dist/milo.h target/release/libmilo.a
	cp target/aarch64-apple-darwin/release/libmilo.a dist/libmilo.a

debug: dist/milo.h target/debug/libmilo.a
	cp target/aarch64-apple-darwin/debug/libmilo.a dist/libmilo.a

target/release/libmilo.a: src/lib.rs
	cargo +nightly build --release --target aarch64-apple-darwin

target/debug/libmilo.a: src/lib.rs
	cargo +nightly build --target aarch64-apple-darwin

dist/milo.h:
	rustup run nightly cbindgen --config cbindgen.toml --output dist/milo.h
	node ./tools/export_version/main.mjs
	node ./tools/export_constants/main.mjs
	
clean: 
	rm -rf dist target