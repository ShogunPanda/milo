TARGET:= $(shell rustc -vV | sed -n 's|host: ||p')

.PHONY: all release debug clean

all: debug release

debug: dist/reference-debug

release: dist/reference-release

clean: 
	rm -rf dist lib

dist/reference-debug: lib/libmilo-debug.a lib/milo.h
	mkdir -p dist
	clang++ -std=c++11 -I lib -o dist/reference-debug lib/libmilo-debug.a src/main.cc

dist/reference-release: lib/libmilo-release.a lib/milo.h 
	mkdir -p dist
	clang++ -std=c++11 -I lib -o dist/reference-release lib/libmilo-release.a src/main.cc

lib/milo.h:
	cd ../parser && make dist/milo.h && cd ../reference-cpp
	mv ../parser/dist/milo.h lib/milo.h

lib/libmilo-debug.a: ../parser/src/lib.rs
	mkdir -p lib
	cd ../parser && make debug && cd ../reference-cpp
	mv ../parser/target/${TARGET}/debug/libmilo.a lib/libmilo-debug.a

lib/libmilo-release.a: ../parser/src/lib.rs
	mkdir -p lib
	cd ../parser && make release && cd ../reference-cpp
	mv ../parser/target/${TARGET}/release/libmilo.a lib/libmilo-release.a
