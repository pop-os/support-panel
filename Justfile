name := 'pop-support'

# Installation paths

rootdir := ''
prefix := '/usr'
usrdir := absolute_path(clean(rootdir / prefix))
bin-dst := usrdir / 'bin' / name
policy-dst := usrdir / 'share' / 'polkit-1' / 'actions' / 'org.pop.support.policy'

# Compile-time env variables

cargo-target-dir := env('CARGO_TARGET_DIR', 'target')

[private]
default: build-release

# Build a debian package locally without a schroot or vendoring
build-deb:
    dpkg-buildpackage -d -nc

# Compile with debug profile
build-debug *args:
    cargo build {{ args }}

# Compile with release profile
build-release *args: (build-debug '--release' args)

# Compile with a vendored tarball
build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

# Check for errors and linter warnings
check *args:
    cargo clippy --all-features {{ args }} -- -W clippy::pedantic

# Runs a check with JSON message format for IDE integration
check-json: (check '--message-format=json')

# Remove Cargo build artifacts
[no-cd]
clean:
    cargo clean

# Also remove .cargo and vendored dependencies
[no-cd]
clean-dist: clean
    rm -rf .cargo vendor vendor.tar target

install:
    install -Dm0755 {{ cargo-target-dir / 'release' / name }} {{ bin-dst }}
    install -Dm0644 'data/org.pop.support.policy' {{ policy-dst }}

uninstall:
    rm {{ bin-dst }} {{ policy-dst }}

# Run the application for testing purposes
run *args:
    env RUST_LOG=debug RUST_BACKTRACE=full cargo run {{ args }} --release

# Run `cargo test`
test *args:
    cargo test {{ args }}

# Vendor Cargo dependencies locally
[no-cd]
vendor:
    mkdir -p .cargo
    cargo vendor | head -n -1 > .cargo/config.toml
    echo 'directory = "vendor"' >> .cargo/config.toml
    tar pcf vendor.tar vendor
    rm -rf vendor

# Extracts vendored dependencies
[no-cd]
[private]
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar

# Bump cargo version, create git commit, and create tag
tag version:
    find -type f -name Cargo.toml -exec sed -i '0,/^version/s/^version.*/version = "{{ version }}"/' '{}' \; -exec git add '{}' \;
    cargo check
    cargo clean
    git add Cargo.lock
    git commit -m 'release: {{ version }}'
    git commit --amend
    git tag -a {{ version }} -m ''
