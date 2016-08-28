TARGET = target/debug/semantic-rs
SRC = $(wildcard src/*.rs)

test: unit integrations

build: $(TARGET)

$(TARGET): $(SRC)
	cargo build

unit:
	cargo test

integrations: $(TARGET)
	./tests/integration/run-locally.sh
