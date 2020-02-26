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
namespace=mandelbrot
#
# harbor project name (if your image is "harbor.gsk.com/sdi/image-name:tag", project would be "sdi")
project=sdi
#
# Agent name; normally you wouldn't need to change this
# If you plan to run multiple agents in the same namespace then give each one a different name
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

echo -e "\nRunning $image in Kubernetes context $(kubectl config current-context) and namespace $namespace..."
kubectl create secret generic $agent_name -n $namespace --dry-run -o yaml \
  --from-file=AZP_TOKEN=$azp_token \
  --from-literal=AZP_URL=$azp_url \
  --from-literal=AZP_POOL=$azp_pool \
  --from-literal=proxy="$proxy" \
  --from-literal=noproxy="$noproxy" \
  | kubectl apply -f -
sed s^{{image}}^$image^g az-agent.yaml | \
  sed s^{{agent-name}}^$agent_name^g | \
  kubectl -n $namespace apply -f -
# force pod to restart with new agent
kubectl delete pod $agent_name-0 -n $namespace
sa_secret_name=$(kubectl get serviceAccounts $agent_name -n $namespace -o=jsonpath={.secrets[*].name})
sa_secret=$(kubectl get secret $sa_secret_name -n $namespace -o json)
echo -e "\nHere is the info you need to configure an Azure Kubernetes service connection:"
echo "The agent's service account is named $agent_name"
echo "The Kubernetes server URL is: $(kubectl config view --minify -o jsonpath={.clusters[0].cluster.server})"
echo -e "The authorization secret is in this file: $agent_name.secret"
echo "$sa_secret" >$agent_name.secret
