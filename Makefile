.PHONY: help build test bench clean doc check fmt clippy example benchmark

# Default target
help:
	@echo "AuraDB - High-performance Rust storage engine"
	@echo ""
	@echo "Available targets:"
	@echo "  build      - Build the project in release mode"
	@echo "  test       - Run all tests"
	@echo "  bench      - Run benchmarks"
	@echo "  clean      - Clean build artifacts"
	@echo "  doc        - Generate documentation"
	@echo "  check      - Check code without building"
	@echo "  fmt        - Format code with rustfmt"
	@echo "  clippy     - Run clippy linter"
	@echo "  example    - Run the basic usage example"
	@echo "  benchmark  - Run the benchmark tool"
	@echo "  install    - Install development dependencies"
	@echo "  watch      - Watch for changes and run tests"

# Build the project
build:
	cargo build --release

# Run tests
test:
	cargo test --all-features

# Run benchmarks
bench:
	cargo bench

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/

# Generate documentation
doc:
	cargo doc --open

# Check code without building
check:
	cargo check --all-features

# Format code
fmt:
	cargo fmt

# Run clippy
clippy:
	cargo clippy --all-features -- -D warnings

# Run the basic usage example
example:
	cargo run --example basic_usage

# Run the benchmark tool
benchmark:
	cargo run --bin benchmark -- --operations 10000 --workload random

# Run benchmark with large values (test WAL-time KV separation)
benchmark-large:
	cargo run --bin benchmark -- --operations 10000 --workload random --large-values

# Run different workload types
benchmark-sequential:
	cargo run --bin benchmark -- --operations 10000 --workload sequential

benchmark-batch:
	cargo run --bin benchmark -- --operations 10000 --workload batch --batch-size 100

benchmark-mixed:
	cargo run --bin benchmark -- --operations 10000 --workload mixed

# Install development dependencies
install:
	cargo install cargo-watch
	cargo install cargo-audit
	cargo install cargo-tarpaulin

# Watch for changes and run tests
watch:
	cargo watch -x check -x test -x run

# Run all checks (useful for CI)
ci: fmt clippy test

# Performance profiling
profile:
	cargo build --release
	perf record --call-graph=dwarf target/release/auradb
	perf report

# Memory profiling with valgrind
memcheck:
	cargo build --release
	valgrind --tool=memcheck --leak-check=full target/release/auradb

# Run with different configurations
run-default:
	cargo run --bin benchmark -- --db-path ./db_default

run-large-memtable:
	cargo run --bin benchmark -- --db-path ./db_large_mem --memtable-size 256MB

run-async-wal:
	cargo run --bin benchmark -- --db-path ./db_async_wal --async-wal

# Clean all test databases
clean-dbs:
	rm -rf ./db_* ./benchmark_db ./auradb_data

# Show project statistics
stats:
	@echo "Project Statistics:"
	@echo "=================="
	@echo "Lines of code:"
	@find src -name "*.rs" -exec wc -l {} + | tail -1
	@echo ""
	@echo "Source files:"
	@find src -name "*.rs" | wc -l
	@echo ""
	@echo "Dependencies:"
	@grep -c "^\[dependencies\]" Cargo.toml || echo "0"
	@echo ""
	@echo "Build size:"
	@ls -lh target/release/auradb 2>/dev/null || echo "Not built yet"

# Development setup
setup:
	@echo "Setting up AuraDB development environment..."
	rustup update
	cargo install cargo-watch
	cargo install cargo-audit
	cargo install cargo-tarpaulin
	@echo "Development environment setup complete!"

# Quick development cycle
dev: fmt clippy test

# Full development cycle
full-dev: clean fmt clippy test bench doc

# Show help for benchmark tool
benchmark-help:
	cargo run --bin benchmark -- --help
