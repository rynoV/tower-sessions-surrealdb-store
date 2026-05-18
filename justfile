default:
    @just --justfile {{justfile()}} --list

install:
    cargo install cargo-msrv --locked
    cargo +stable install cargo-hack --locked
    cargo +stable install cargo-minimal-versions --locked

test *args:
    cargo {{args}} test

test-stable: (test)

test-all: test-stable

check *args:
    cargo {{args}} fmt --check
    cargo {{args}} msrv verify
    cargo {{args}} check
    cargo {{args}} clippy --no-deps


# These minimal checks are currently failing to compile, I think because surrealdb 3.0.0 isn't compatible with its 3.0.5 surrealdb-* dependencies
check-minimal: (check "minimal-versions" "--direct")
test-stable-minimal: (test "minimal-versions" "--direct")
test-all-minimal: test-stable-minimal

ci: check test-all

fix:
    cargo clippy --fix --profile test --allow-dirty --allow-staged --allow-no-vcs

format:
    cargo fmt

fix-and-fmt:
    just fix
    just format

