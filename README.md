# MandelRust

![flaming](flaming.png)

## Overview
This full-featured Mandelbrot web app is fast due to its use of Rust for doing the calculations. These can be done in your browser (with Rust WebAssembly), or you can install the backend web server for even more speed.  The web server can run on bare metal (your PC or a server), in Docker, or on Kubernetes, where it can scale to multiple pods.

Try the browser-only version at https://wpwoodjr.github.io/MandelRust

![Interface](interface.png)

The original version by [David Eck](http://math.hws.edu/eck/index.html) runs entirely in the browser, and all calculations are done in Javascript.  This rewritten version calculates in the browser or optionally using a backend server.  The in-browser calculations are implemented in Rust and deployed in the browser via WebAssembly.  For high precision calculations, this is about 5-7 times faster than the Javascript version.  The backend server is implemented entirely in Rust using Actix-web.  It can be up to 15 times faster than the original Javascript version.

### Getting started
![Mandala](mandala.png)

#### Docker
You can use a pre-built Docker image, or download this repo and build it using Docker.  To use the pre-built image:
```
docker run -d --rm --name mb-server -p 8001:8001 wpwoodjr/mb-server:latest 0.0.0.0:8001
```
Then browse to `localhost:8001`.

To build it in Docker, clone or download this repo, then build and run in Docker as follows:
```
cd MandelRust
./docker-build.sh
./docker-run.sh
```
Then browse to `localhost:8001`.

#### Local build/run (faster than Docker)
For a 35% speed boost over Docker, you can run the server locally.  A Rust compiler is a pre-requisite.  First, clone or download this repo. Then, build and run as follows:
```
cd MandelRust
./local-build.sh
./local-run.sh
```
Then browse to `localhost:8000`.

#### Kubernetes (potentially very fast)
Clone or download this repo, then build and deploy in Kubernetes as follows:
```
cd MandelRust
... WIP ...
```
Once the application is deployed to Kubernetes, browse to `<url>`.

![diamonds](diamonds.png)
