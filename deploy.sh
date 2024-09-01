#!/bin/bash

cargo clean
cargo build --release
sudo systemctl restart plakait-backend
