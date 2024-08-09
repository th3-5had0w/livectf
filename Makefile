all:
	cargo build --release
	cp -r src/web_interface/static ./RELEASE
	cp ./target/release/livectf ./RELEASE

dev:
	cargo build --release
	cp -r src/web_interface/static ./RELEASE
	cp ./target/release/livectf ./RELEASE
	cargo watch -x 'run'
static:
	cp -r src/web_interface/static ./RELEASE