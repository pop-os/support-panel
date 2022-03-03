# Standard Pop variables
DESTDIR := ''
prefix := '/usr/local'
DEBUG := '0'
VENDOR := '0'
TARGET := if DEBUG == '1' { 'debug' } else { 'release' }
ARGS_VENDOR := if VENDOR == '1' { '--frozen --offline' } else { '' }
ARGS_DEBUG := if DEBUG == '1' { '' } else { '--release' }
ARGS := ARGS_VENDOR + ' ' + ARGS_DEBUG

# Locations of essential files
sysconfdir := '/etc'
bindir := prefix + '/bin'
includedir := prefix + '/include'
libdir := prefix + '/lib'

PACKAGE := 'pop_support'
PKGCONFIG := 'target/' + PACKAGE + '.pc'
FFI := 'target/' + TARGET + '/lib' + PACKAGE + '.so'
INSTALL_CLIB := DESTDIR + libdir + '/lib' + PACKAGE + '.so'
INSTALL_HEADER := DESTDIR + includedir + '/' + PACKAGE + '.h'
INSTALL_PKGCONF := DESTDIR + libdir + '/pkgconfig/' + PACKAGE + '.pc'

all: extract-vendor
    cargo build {{ARGS}}
    cargo build {{ARGS}} --manifest-path ffi/Cargo.toml
    cargo run -p tools --bin pkgconfig -- {{PACKAGE}} {{libdir}} {{includedir}}

install:
    install -Dm0644 target/{{TARGET}}/lib{{PACKAGE}}.so {{INSTALL_CLIB}}
    install -Dm0644 data/{{PACKAGE}}.h {{INSTALL_HEADER}}
    install -Dm0644 {{PKGCONFIG}} {{INSTALL_PKGCONF}}

uninstall:
    rm {{INSTALL_CLIB}} {{INSTALL_HEADER}} {{INSTALL_PKGCONF}}

# Pop standard build routines

clean:
    cargo clean

distclean:
    rm -rf .cargo vendor vendor.tar target

extract-vendor:
    test {{VENDOR}} -eq '1' && (rm -rf vendor; tar pxf vendor.tar) || true

vendor:
    mkdir -p .cargo
    cargo vendor --sync ffi/Cargo.toml \
        --sync tools/Cargo.toml \
        | head -n -1 > .cargo/config
    echo 'directory = "vendor"' >> .cargo/config
    tar pcf vendor.tar vendor
    rm -rf vendor