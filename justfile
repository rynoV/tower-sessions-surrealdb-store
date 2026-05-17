default:
    @just --justfile {{justfile()}} --list

install:
    cargo install cargo-msrv --locked
    cargo +stable install cargo-hack --locked
    cargo +stable install cargo-minimal-versions --locked

test *args:
    cargo {{args}} test

test-stable: (test)
test-stable-minimal: (test "minimal-versions" "--direct")

test-all: test-stable
test-all-minimal: test-stable-minimal

check *args:
    cargo {{args}} fmt --check
    cargo {{args}} msrv verify
    cargo {{args}} check
    cargo {{args}} clippy --no-deps

check-minimal: (check "minimal-versions" "--direct")

ci: check-minimal test-all-minimal check test-all

fix:
    cargo clippy --fix --profile test --allow-dirty --allow-staged --allow-no-vcs

format:
    cargo fmt

fix-and-fmt:
    just fix
    just format

