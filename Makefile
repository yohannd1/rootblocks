nothing:
	@echo "Please select an action"
	@exit 1

run: build
	./target/release/rootblocks

build:
	cargo build --release

install: build
	mkdir -p ~/.local/bin
	cp ./target/release/rootblocks -t ~/.local/bin
