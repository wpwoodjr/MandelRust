# Mandelbrot for Kubernetes

## Overview
This client/server application uses a Kubernetes cluster to compute the Mandelbrot iteration count for each pixel in a set of coordinates, 
and provides a browser-based client interface for viewing and navigating.

Forked from a browser-only version by David Eck, http://math.hws.edu/eck/index.html

### Getting started
Build and deploy as follows:
```
$ ./build.sh <kubernetes-environment> <image-tag> <url> <num-replicas> <uname:pwd>
```

### Running the application
Once the application is deployed, browse to `<url>`.
