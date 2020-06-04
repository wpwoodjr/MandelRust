#set -e
#pass show docker-credential-helpers/docker-pass-initialized-check
#docker login harbor.gsk.com
#docker build --tag harbor.gsk.com/sdi/mandelbrot:"$2" .
#docker push harbor.gsk.com/sdi/mandelbrot:"$2"
sed -e "s/{{image-tag}}/$2/g" \
  -e "s^{{url}}^$3^g" \
  -e "s^{{replicas}}^$4^g" \
  mandelbrot.yaml | kubectl --context="$1" apply -f -
