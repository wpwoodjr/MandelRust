#!/bin/bash
# Stamp a content hash into client/MB.html so that changing any cache-sensitive
# client asset also changes the URL it is requested from.
#
# MB.html is the entry point and cannot version itself, so it carries the hash:
#   - as ASSET_VERSION, used by JS to fetch the wasm and spawn the workers
#   - as ?v=<hash> on the <script src> tags, which are markup and cannot read JS
#
# Usage: stamp-assets.sh            rewrite the stamps from the assets
#        stamp-assets.sh --check    exit 1 if any stamp is out of date
#
# --check compares content, not mtimes: git checkout rewrites mtimes on every
# branch switch, so timestamps say nothing about which file is stale.
set -euo pipefail
cd "$(dirname "$0")"

check=0
[ "${1:-}" = "--check" ] && check=1

# Fetched from JS, versioned via ASSET_VERSION.
JS_ASSETS=(
    client/mb-wasm.wasm
    client/mandelbrot-worker-local-wasm.js
    client/mandelbrot-worker-local-js.js
    client/mandelbrot-worker-remote.js
)

# Loaded by <script src> in MB.html, versioned by stamping the attribute.
PAGE_SCRIPTS=(
    BigDecimal-all-last.min.js
    slider.js
    touch.js
    percentage-complete.js
)

assets=("${JS_ASSETS[@]}")
for s in "${PAGE_SCRIPTS[@]}"; do assets+=("client/$s"); done

for f in "${assets[@]}"; do
    [ -f "$f" ] || { echo "stamp-assets: missing $f" >&2; exit 1; }
done

# MB.html is excluded from the hash: it carries the result.
hash=$(cat "${assets[@]}" | sha256sum | cut -c1-10)
current=$(sed -nE 's|^const ASSET_VERSION = "([0-9a-f]+)";$|\1|p' client/MB.html)

if [ -z "$current" ]; then
    echo "stamp-assets: no ASSET_VERSION line to stamp in client/MB.html" >&2
    exit 1
fi

# Every ?v= in MB.html must agree with ASSET_VERSION, or some asset is versioned
# against a stamp nobody refreshed.
stale_tags=$(grep -oE '\?v=[0-9a-f]+' client/MB.html | grep -vE "\?v=$hash" || true)

if [ "$check" -eq 1 ]; then
    if [ "$current" != "$hash" ]; then
        echo "stamp-assets: ASSET_VERSION is $current, assets hash to $hash" >&2
        echo "  a client asset changed without re-stamping; run ./stamp-assets.sh" >&2
        exit 1
    fi
    if [ -n "$stale_tags" ]; then
        echo "stamp-assets: <script src> stamped with a stale hash: $(echo $stale_tags | tr '\n' ' ')" >&2
        echo "  expected ?v=$hash; run ./stamp-assets.sh" >&2
        exit 1
    fi
    # Guard against a <script src> that was added but never stamped.
    for s in "${PAGE_SCRIPTS[@]}"; do
        if ! grep -qF "src=\"$s?v=$hash\"" client/MB.html; then
            echo "stamp-assets: <script src=\"$s\"> is not stamped with ?v=$hash" >&2
            exit 1
        fi
    done
    echo "ASSET_VERSION $hash is current"
    exit 0
fi

sed -i -E "s|^const ASSET_VERSION = \"[0-9a-f]+\";\$|const ASSET_VERSION = \"$hash\";|" client/MB.html

# Add ?v= if absent, refresh it if present. Filenames are matched literally.
for s in "${PAGE_SCRIPTS[@]}"; do
    esc=$(printf '%s' "$s" | sed -e 's|[.[\*^$/]|\\&|g')
    sed -i -E "s|src=\"$esc(\?v=[0-9a-f]+)?\"|src=\"$s?v=$hash\"|g" client/MB.html
done

echo "stamped ASSET_VERSION = $hash (${#assets[@]} assets)"
