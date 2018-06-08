osx: target/x86_64-apple-darwin/release/sit target/x86_64-apple-darwin/release/sit-web

target/x86_64-apple-darwin/release/sit target/x86_64-apple-darwin/release/sit-web: build-tools/cross-compile-osx/Dockerfile sit-core sit-web sit
	docker build --iidfile ._docker_osx build-tools/cross-compile-osx
	sed -i s/sha256://g ._docker_osx
	docker run -u `id -u`:`id -g` -v `pwd`:/sit -w /sit -t `cat ._docker_osx` sh -c "cargo build --release --target=x86_64-apple-darwin"
	rm -f ._docker_osx
