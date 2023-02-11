#!/bin/bash
# run mandelbrot server in docker

port=8001
[ -n "$1" ] && port="$1" && shift 1
docker stop mb-server 2>/dev/null
docker run -d --rm --name mb-server -p $port:$port mb-server:latest 0.0.0.0:$port $@ \
&& echo Mandelbrot server running on URL localhost:$port $@
