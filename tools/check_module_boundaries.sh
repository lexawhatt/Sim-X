#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo-modules >/dev/null 2>&1; then
    echo "cargo-modules is required: cargo install cargo-modules --version 0.27.0 --locked" >&2
    exit 2
fi

module_options=(
    --lib
    --all-features
    --no-externs
    --no-fns
    --no-sysroot
    --no-traits
    --no-types
)

check_focus() {
    local focus="$1"
    local forbidden_pattern="$2"
    local forbidden_description="$3"
    local graph
    graph="$(NO_COLOR=1 cargo modules dependencies "${module_options[@]}" --focus-on "$focus")"
    if rg -q "$forbidden_pattern" <<<"$graph"; then
        echo "forbidden dependency from $focus into $forbidden_description" >&2
        echo "$graph" >&2
        exit 1
    fi
}

check_focus 'sim_x::foundation' 'sim_x::(app|domains|presentation)' 'app/domains/presentation'
check_focus 'sim_x::domains' 'sim_x::(app|presentation)' 'app/presentation'

physics_subdomains=(
    mechanics
    thermodynamics
    waves_optics
    electromagnetism
)

for owner in "${physics_subdomains[@]}"; do
    focus="sim_x::domains::phys::$owner"
    graph="$(NO_COLOR=1 cargo modules dependencies "${module_options[@]}" --focus-on "$focus")"
    for sibling in "${physics_subdomains[@]}"; do
        if [[ "$sibling" != "$owner" ]] && rg -q "sim_x::domains::phys::$sibling" <<<"$graph"; then
            echo "forbidden sibling dependency from $focus into $sibling" >&2
            echo "$graph" >&2
            exit 1
        fi
    done
done

headless_tree="$(cargo tree --no-default-features -e normal)"
if rg -q '(^| )(sim-engine|winit|rodio|wgpu) v' <<<"$headless_tree"; then
    echo "desktop dependency leaked into the headless scientific graph" >&2
    echo "$headless_tree" >&2
    exit 1
fi

echo "module and headless dependency boundaries are clean"
