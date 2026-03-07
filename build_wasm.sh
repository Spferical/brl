ROOT=$(cargo metadata --format-version=1 | jq -r .resolve.root)
PROJECT_NAME=$(cargo metadata --format-version=1 \
               | jq -r ".packages[] | select(.id==\"${ROOT}\") | .name")
TARGET_DIR=$(cargo metadata --format-version=1 | jq -r .target_directory)/wasm32-unknown-unknown
export RUSTFLAGS='--cfg getrandom_backend="wasm_js" --cfg=web_sys_unstable_apis'

if [ -n "$RELEASE" ]; then
	TARGET_DIR="$TARGET_DIR/release"
else
	TARGET_DIR="$TARGET_DIR/debug"
fi

mkdir -p dist

build_and_bind() {
    local FEATURE_FLAG=$1
    local OUT_SUFFIX=$2
    
    echo "Building for $OUT_SUFFIX..."
    if [ -n "$RELEASE" ]; then
        cargo build --release --no-default-features $FEATURE_FLAG --target wasm32-unknown-unknown
    else
        cargo build --no-default-features $FEATURE_FLAG --target wasm32-unknown-unknown
    fi

    # Generate bindgen outputs
    wasm-bindgen "$TARGET_DIR"/"$PROJECT_NAME".wasm --out-dir dist --out-name "${PROJECT_NAME}_${OUT_SUFFIX}" --target web --no-typescript
    
    if [ -n "$RELEASE" ]; then
        wasm-opt -Os dist/"${PROJECT_NAME}_${OUT_SUFFIX}_bg.wasm" -o dist/"${PROJECT_NAME}_${OUT_SUFFIX}_bg.wasm"
    fi
}

# WebGL2 build (no webgpu feature)
build_and_bind "" "webgl2"

# WebGPU build (with webgpu feature)
build_and_bind "--features webgpu" "webgpu"

cp -r wasm/* dist/
cp -r assets/ dist/
