# Mandelbrot for Kubernetes

## Overview
This client/server application uses a Kubernetes cluster to compute the Mandelbrot iteration count for each pixel in a set of coordinates, 
and provides a browser-based client interface for viewing and navigating.

Forked from a browser-only version by David Eck, http://math.hws.edu/eck/index.html

### Getting started
Create the docker container from source, replacing <ver> with the version number:
```
$ docker build --tag harbor.gsk.com/sdi/mandelbrot-server:<ver> .
```
Then push the Docker container to Kubernetes:
```
$ ./kubepush <ver>
```

### Running the application
Once the application is deployed, browse to https://<host>/MB.html, where <host> is the deployment host as defined in mandelbrot.yaml.