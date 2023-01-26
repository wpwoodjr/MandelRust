# Mandelbrot for Docker and Kubernetes

## Overview
This full-featured Mandelbrot web app is a client/server application using a local build, Docker, or a Kubernetes cluster to compute the Mandelbrot set, 
with a browser-based client interface for viewing and navigating.

Forked from a browser-only version by David Eck, at http://math.hws.edu/eck/index.html

This version's backend web server has been rewritten from JavaScript to Rust and is much faster.

![dancing](dancing.png)

### Getting started
#### Docker
Clone or download this repo, then build and run in Docker as follows:
```
cd Mandelbrot-for-Docker-and-Kubernetes/mb-kubernetes
./docker-run.sh
```
Then browse to `localhost:8001`.

#### Local build/run
If you don't want to use Docker, you can run the server locally.  A Rust compiler is a pre-requisite.  First, clone or download this repo. Then, build and run as follows:
```
cd Mandelbrot-for-Docker-and-Kubernetes/mb-kubernetes
./local-build.sh
./local-run.sh
```
Then browse to `localhost:8000`.

#### Kubernetes
Clone or download this repo, then build and deploy in Kubernetes as follows:
```
cd Mandelbrot-for-Docker-and-Kubernetes/mb-kubernetes
... WIP ...
```
Once the application is deployed to Kubernetes, browse to `<url>`.

![flaming](flaming.png)
