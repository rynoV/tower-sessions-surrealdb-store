default:
    @just --justfile {{justfile()}} --list

install:
    cargo install cargo-msrv --locked
    cargo +stable install cargo-hack --locked
    cargo +stable install cargo-minimal-versions --locked

test surreal *args:
    cargo {{args}} test --no-default-features --features {{surreal}},{{surreal}}/kv-mem

test-stable: (test "surrealdb")
test-stable-minimal: (test "surrealdb" "minimal-versions" "--direct")
test-nightly: (test "surrealdb-nightly")
test-nightly-minimal: (test "surrealdb-nightly" "minimal-versions" "--direct")

test-all: test-stable test-nightly
test-all-minimal: test-stable-minimal test-nightly-minimal

check *args:
    cargo {{args}} fmt --check
    cargo {{args}} msrv verify
    cargo {{args}} check
    cargo {{args}} clippy --no-deps

check-minimal: (check "minimal-versions" "--direct")

ci: check-minimal test-all-minimal check test-all

fix:
    cargo clippy --fix --profile test --features surrealdb,surrealdb/kv-mem --allow-dirty --allow-staged --allow-no-vcs

format:
    cargo fmt

fix-and-fmt:
    just fix
    just format

