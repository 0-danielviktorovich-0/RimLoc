#!/bin/bash
# Сборка swift-хелперов автоматизации (один раз на машину).
set -e
DIR="$(cd "$(dirname "$0")" && pwd)"
swiftc -O "$DIR/place-win.swift" -o /tmp/place-win
echo "built /tmp/place-win"
