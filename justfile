run:
    cargo watch -- cargo run -j $(nproc)

format:
    cargo fmt

lint:
    cargo clippy -- -D warnings

fix:
    cargo clippy --fix --allow-dirty --allow-staged

test:
    cargo test

build:
    cargo build --release

dev:
    cargo build

clean:
    cargo clean

check:
    cargo check

update:
    cargo update

precommit:
    just format
    just lint
    just test

release:
    just build

generate FORMAT="yaml":
    cargo run generate --format {{ FORMAT }}
