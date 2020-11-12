# TODO (richo) make this smart
all:
	cargo build
	cargo build --features web
	(cd wasm; wasm-pack build -t web)
