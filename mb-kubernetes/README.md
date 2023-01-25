# Mandelbrot for Docker and Kubernetes

## Overview
This client/server application uses Docker or a Kubernetes cluster to compute the Mandelbrot set, 
and provides a browser-based client interface for viewing and navigating.

This version's backend web server has been rewritten from JavaScript to Rust and is much faster.

Forked from a browser-only version by David Eck, at http://math.hws.edu/eck/index.html

![dancing](dancing.png)

### Getting started
#### Docker
Clone or download this repo, then build and deploy in Docker as follows:
```
cd Mandelbrot-for-Docker-and-Kubernetes/mb-kubernetes
./docker-run.sh
```
Then browse to `localhost:8001`.

#### Kubernetes
Clone or download this repo, then build and deploy in Kubernetes as follows:
```
cd Mandelbrot-for-Docker-and-Kubernetes/mb-kubernetes
... men at work ...
```
Once the application is deployed to Kubernetes, browse to `<url>`.

![flaming](flaming.png)
