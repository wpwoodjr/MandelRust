#!/bin/bash
# mandelbrot delete

set -e
[ -z "$1" ] && echo "No Kubernetes context specified, exiting..." && exit 1
kpt live destroy . --context "$1"
rm -f inventory-template.yaml
