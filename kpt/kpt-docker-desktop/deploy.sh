#!/bin/bash
# mandelbrot deploy

set -e
[ -z "$1" ] && echo "No Kubernetes context specified, exiting..." && exit 1
[ -z "$2" ] && echo "No Kubernetes namespace specified, exiting..." && exit 1
[ -z "$3" ] && echo "No url specified, exiting..." && exit 1
[ ! -z "$4" ] && kpt cfg set . replicas "$4"
kpt cfg set . namespace "$2"
kpt cfg set . url "$3"
kpt cfg list-setters .
kpt live init . --inventory-namespace "$2"
kpt live apply . --context "$1" --reconcile-timeout 30s --prune-timeout 30s --prune-propagation-policy Foreground
kubectl --context "$1" --namespace "$2" get pods
echo "Browse to https://$3"
