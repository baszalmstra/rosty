#!/bin/bash

source /opt/ros/melodic/setup.bash
cargo b
cargo fmt --all -- --check
cargo clippy --all -- -D warnings
cargo t
