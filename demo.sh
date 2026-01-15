#!/bin/bash

INPUT_IMAGE="./demo/weather.png"
OUTPUT_DIR="./demo_output"
TARGET_PROFILE="debug"
RELEASE_FLAG=""
PLUGIN_PATH_ARG=""

for arg in "$@"; do
  if [ "$arg" == "--release" ]; then
    TARGET_PROFILE="release"
    RELEASE_FLAG="--release"
    PLUGIN_PATH_ARG="--plugin-path target/release"
  fi
done

echo "Preparing output directory"
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

echo "Building project in $TARGET_PROFILE mode..."
cargo build $RELEASE_FLAG

run_processor() {
    local plugin=$1
    local params=$2
    local output=$3
    local description=$4

    echo "$description..."
    ./target/$TARGET_PROFILE/image_processor \
        --input "$INPUT_IMAGE" \
        --output "$OUTPUT_DIR/$output" \
        --plugin "$plugin" \
        --params "$params" \
        $PLUGIN_PATH_ARG
}

run_processor "mirror" "demo/mirror_h.json" "out_mirror_h.png" "Applying mirror horizontal"
run_processor "mirror" "demo/mirror_v.json" "out_mirror_v.png" "Applying mirror vertical"
run_processor "mirror" "demo/mirror_both.json" "out_mirror_both.png" "Applying mirror both"
run_processor "blur" "demo/blur_box.json" "out_blur_box.png" "Applying box blur"
run_processor "blur" "demo/blur_gauss.json" "out_blur_gauss.png" "Applying Gaussian blur"

echo "Done! Results are in $OUTPUT_DIR"