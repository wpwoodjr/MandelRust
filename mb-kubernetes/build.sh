#!/bin/bash
# mandelbrot build

[ -z "$1" ] && echo "No Kubernetes context specified, exiting..." && exit 1
[ -z "$2" ] && echo "No image tag specified, exiting..." && exit 1
[ -z "$3" ] && echo "No url specified, exiting..." && exit 1
[ -z "$4" ] && echo "No number of replicas specified, exiting..." && exit 1
[ -z "$5" ] && echo "No uname:pwd specified, exiting..." && exit 1

proxy=http://"$5"@nodecrypt.gtm.corpnet2.com:800
noproxy=localhost,127.0.0.0/8,.gsk.com,.corpnet1.com,.corpnet2.com,.corpnet3d.com

docker login harbor.gsk.com
docker build --tag harbor.gsk.com/sdi/mandelbrot:"$2" . \
    --build-arg http_proxy="$proxy" \
    --build-arg HTTP_PROXY="$proxy" \
    --build-arg https_proxy="$proxy" \
    --build-arg HTTPS_PROXY="$proxy" \
    --build-arg ftp_proxy="$proxy" \
    --build-arg FTP_PROXY="$proxy" \
    --build-arg no_proxy="$noproxy" \
    --build-arg NO_PROXY="$noproxy"
docker push harbor.gsk.com/sdi/mandelbrot:"$2"

sed -e "s/{{image-tag}}/$2/g" \
  -e "s^{{url}}^$3^g" \
  -e "s^{{replicas}}^$4^g" \
  mandelbrot.yaml | kubectl --context="$1" apply -f -
