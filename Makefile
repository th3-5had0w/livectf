all:
	cargo build --release
	cp ./target/release/livectf ./RELEASE