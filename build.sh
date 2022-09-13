#!/usr/bin/env bash
set -e
WORK_PATH=$(
    # shellcheck disable=SC2046
    cd $(dirname "${BASH_SOURCE[0]}") || exit 1
    pwd
)
build() {
    cargo build --release
}

offline-build() {
    if [ -f "$WORK_PATH/offline.tgz" ]; then
        tar Czxf "$WORK_PATH" offline.tgz
    fi
    mkdir -p .cargo
    cat <<EOF | tee "$WORK_PATH"/.cargo/config.toml >/dev/null
    [source.crates-io]
    replace-with = "vendored-sources"

    [source.vendored-sources]
    directory = "vendor"
EOF
    cargo build --release --offline
    rm -f "$WORK_PATH"/.cargo/config.toml
}
offline-package() {
    cd "$WORK_PATH" || exit 1
    rm -rf "$WORK_PATH"/vendor "$WORK_PATH"/offline.tgz
    cargo vendor
    tar zcvf "$WORK_PATH"/offline.tgz vendor
}
"$1"
