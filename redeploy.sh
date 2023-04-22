#!/bin/bash

cargo build --release
sudo fuser -k 3000/tcp
sudo systemctl restart plakait-backend
