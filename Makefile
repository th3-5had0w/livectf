all:
	cargo build --release
	cp -r src/web_interface/static ./RELEASE
	cp ./target/release/livectf ./RELEASE

static:
	cp -r src/web_interface/static ./RELEASE