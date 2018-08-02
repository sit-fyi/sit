.PHONY: test

osx: target/x86_64-apple-darwin/release/sit target/x86_64-apple-darwin/release/sit-web
linux: target/x86_64-unknown-linux-musl/release/sit target/x86_64-unknown-linux-musl/release/sit-web

target/x86_64-apple-darwin/release/sit target/x86_64-apple-darwin/release/sit-web: build-tools/cross-compile-osx/Dockerfile sit-core sit-web sit
	docker build --iidfile ._docker_osx build-tools/cross-compile-osx
	sed -i s/sha256://g ._docker_osx
	docker run -u `id -u`:`id -g` -v `pwd`:/sit -w /sit -t `cat ._docker_osx` sh -c "cargo build --release --target=x86_64-apple-darwin"
	rm -f ._docker_osx

target/x86_64-unknown-linux-musl/release/sit target/x86_64-unknown-linux-musl/release/sit-web: build-tools/linux-build-container/Dockerfile sit-core sit-web sit
	docker build --iidfile ._docker_linux build-tools/linux-build-container
	sed -i s/sha256://g ._docker_linux
	docker run -u `id -u`:`id -g` -v `pwd`:/sit -w /sit -t `cat ._docker_linux` sh -c "cargo build --release --target=x86_64-unknown-linux-musl && strip target/x86_64-unknown-linux-musl/release/sit target/x86_64-unknown-linux-musl/release/sit-web"
	rm -f ._docker_linux

test:
	# Test without deprecated-item-api
	cd sit-core && cargo test --no-default-features --features="sha-1 blake2 duktape-reducers duktape-mmap duktape-require"
	# Test
	cargo test
	# Test sit without deprecated-items
	cd sit && cargo test --no-default-features
