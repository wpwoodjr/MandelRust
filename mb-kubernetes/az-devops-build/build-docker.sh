#!/bin/bash
# Azure DevOps agent build script

### update these values for your situation ###
#
# url of Azure devops "organization"
azp_url=https://dev.azure.com/SDI-K8s/
#
# Azure personal access token file name (download from Azure devops)
azp_token=sdi-k8s.pat
#
# Azure agent pool name
azp_pool=gsk-dev
#
# Kubernetes namespace to deploy agent in (typically same as your project's namespace)
namespace=cp-sdi-k8s
#
# harbor project name (if your image is "harbor.gsk.com/sdi/image-name:tag", project would be "sdi")
project=sdi
#
# Agent name; normally you wouldn't need to change this
# If you plan to run multiple agents in the same Docker instance then give each one a different name
agent_name=az-agent
#
# Image name of the agent; normally you wouldn't need to change this
image=harbor.gsk.com/$project/$agent_name:1.0.0
#
# After setting the above values, run this to build your agent and push it to Kubernetes:
#
#   $ ./build.sh username:password
#
# Proxy environment variables will be included in the agent image using username:password
# Don't explicitly set them in Dockerfile as they would be visible in your image.
#
### there shouldn't be any need to edit after this line ###

[ -z "$1" ] && echo "No username:password specified, exiting..." && exit
proxy=http://"$1"@nodecrypt.gtm.corpnet2.com:800
noproxy=localhost,127.0.0.0/8,.gsk.com,.corpnet1.com,.corpnet2.com,.corpnet3d.com

set -e
echo -e "\nBuilding docker container $image..."
docker build --tag $image . \
  --build-arg http_proxy="$proxy" \
  --build-arg HTTP_PROXY="$proxy" \
  --build-arg https_proxy="$proxy" \
  --build-arg HTTPS_PROXY="$proxy" \
  --build-arg ftp_proxy="$proxy" \
  --build-arg FTP_PROXY="$proxy" \
  --build-arg no_proxy="$noproxy" \
  --build-arg NO_PROXY="$noproxy"

echo -e "\nPushing docker image to $image..."
docker login harbor.gsk.com
docker push $image

set +e
echo -e "\nStopping $agent_name..."
docker stop $agent_name && sleep 3

echo -e "Running $image in Docker..."
docker run --rm --detach --name $agent_name --hostname $agent_name \
  --env http_proxy="$proxy" \
  --env HTTP_PROXY="$proxy" \
  --env https_proxy="$proxy" \
  --env HTTPS_PROXY="$proxy" \
  --env ftp_proxy="$proxy" \
  --env FTP_PROXY="$proxy" \
  --env no_proxy="$noproxy" \
  --env NO_PROXY="$noproxy" \
  --env AZP_URL=$azp_url \
  --env AZP_POOL=$azp_pool \
  --env AZP_TOKEN=$(cat $azp_token) \
  $image
