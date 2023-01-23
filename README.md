# Mandelbrot for Docker and Kubernetes
Web app to explore the Mandelbrot set.  Uses Docker or a Kubernetes cluster as the backend for computation.

This repo contains David Eck's original client-side-only JavaScript version (with a few modifications) in [math.hws.edu](math.hws.edu), and a Docker or Kubernetes client/server version in [mb-kubernetes](mb-kubernetes).  The server is written in Rust for a big speed improvement over the original.

![Mandala](mandala.png)
