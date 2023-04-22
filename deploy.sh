#!/bin/bash

cargo build --release
sudo fuser -k 3000/tcp
sudo ./target/release/plakait-backend
