doc:
	cd ../../ && cargo doc --lib --no-deps --document-private-items --target-dir docs

.PHONY: doc
