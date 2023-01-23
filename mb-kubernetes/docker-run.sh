#!/bin/bash
# run mandelbrot server in docker
port=8001
[ -n "$1" ] && port="$1"
docker build --tag mb-server . \
&& docker run -d --rm --name mb-server -p $port:$port mb-server 0.0.0.0:$port \
&& echo Mandelbrot server running on URL localhost:$port
