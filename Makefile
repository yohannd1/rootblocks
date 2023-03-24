DESTDIR :=

nothing:
	@echo "Please select an action"
	@exit 1

run: build
	./target/release/rootblocks

build:
	cargo build --release

install: build
	@if [ -z "$(DESTDIR)" ]; then printf >&2 "Unspecified DESTDIR.\n" && exit 1; fi
	mkdir -p $(DESTDIR)/bin
	cp ./target/release/rootblocks -t $(DESTDIR)/bin
