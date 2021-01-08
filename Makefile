nothing:
	@echo "Please select an action"
	@exit 1

install:
	cargo build --release
	mkdir -p ~/.local/bin
	cp ./target/release/rootblocks -t ~/.local/bin
