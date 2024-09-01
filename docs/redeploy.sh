#!/bin/bash

cargo build --release
sudo systemctl restart plakait-backend
