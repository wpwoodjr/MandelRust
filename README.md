# Mandelbrot for Docker and Kubernetes

This repo contains a full-featured web app to explore the Mandelbrot set.  Run it locally, or use Docker or a Kubernetes cluster as the backend for computation.

![Interface](interface.png)

This repo contains David Eck's original client-side-only JavaScript version (with a few modifications) in [math.hws.edu](math.hws.edu), and a Docker or Kubernetes client/server version in [mb-kubernetes](mb-kubernetes).  The server is written in Rust for a big speed improvement over the original.

![Mandala](mandala.png)
