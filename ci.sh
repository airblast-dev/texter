set -e

cargo clippy
cargo test
cargo machete
