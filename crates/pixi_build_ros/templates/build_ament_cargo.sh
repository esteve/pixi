#!/usr/bin/env bash
set -euo pipefail

source_dir=@SOURCE_DIR@
ros_package_name=@ROS_PACKAGE_NAME@
runtime_state="$BUILD_PREFIX/.pixi-build-ros/ament-cargo"
cargo_home="$runtime_state/cargo-home"
staging_dir="$runtime_state/staging"
target_dir="$runtime_state/target"
cargo="$BUILD_PREFIX/bin/cargo"
output_dir="$PREFIX/lib/$ros_package_name"
marker_path="$PREFIX/share/ament_index/resource_index/packages/$ros_package_name"
package_xml_path="$PREFIX/share/$ros_package_name/package.xml"

remove_runtime_state() {
    if ! rm -rf "$runtime_state"; then
        echo "failed to remove ament_cargo runtime state: $runtime_state" >&2
        return 1
    fi
    if [ -e "$runtime_state" ] || [ -L "$runtime_state" ]; then
        echo "ament_cargo runtime state remains after cleanup: $runtime_state" >&2
        return 1
    fi
}

cleanup_on_exit() {
    status=$?
    if [ "$status" -ne 0 ]; then
        rm -rf "$output_dir" "$marker_path" "$package_xml_path" || status=1
        if [ -e "$output_dir" ] || [ -e "$marker_path" ] || [ -e "$package_xml_path" ]; then
            status=1
        fi
    fi
    if ! remove_runtime_state; then
        status=1
    fi
    exit "$status"
}
trap cleanup_on_exit EXIT

remove_runtime_state
mkdir -p "$cargo_home" "$staging_dir" "$target_dir"
export CARGO_HOME="$cargo_home"

rm -rf "$output_dir"
mkdir -p "$output_dir"

"$cargo" install \
    --root "$staging_dir" \
    --path "$source_dir" \
    --target-dir "$target_dir" \
    --no-track \
    --force

if [ -d "$staging_dir/bin" ]; then
    for binary in "$staging_dir/bin/"*; do
        [ -f "$binary" ] || continue
        cp "$binary" "$output_dir/"
        printf '%s\n' "$output_dir/$(basename "$binary")" >> "$RATTLER_BUILD_PACKAGE_FILES"
    done
fi

mkdir -p \
    "$PREFIX/share/ament_index/resource_index/packages" \
    "$PREFIX/share/$ros_package_name"
touch "$marker_path"
cp "$source_dir/package.xml" "$package_xml_path"

printf '%s\n' \
    "$marker_path" "$package_xml_path" >> "$RATTLER_BUILD_PACKAGE_FILES"
