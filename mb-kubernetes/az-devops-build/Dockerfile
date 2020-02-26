# Azure agent Dockerfile

# The az-agent-base image is running Ubuntu 18.04
#
# The base image comes pre-loaded with:
#  * GSK self-signed certificates
#  * docker with buildkit backend
#  * kubectl
#  * utilities including git, curl, ca-certificates, ping, and jq
#
# It will automatically run apt update && apt upgrade when it is pulled
FROM harbor.gsk.com/sdi/az-agent-base:1.0.0

# Your agent's capabilities are cataloged and advertised in the agent pool
# so that only the builds and releases it can handle are assigned to it. 

# Generally you should install on your agent whatever software and tools you use
# on your development machine.

# For example, if your build includes the npm task, then the build won't run
# unless your agent has npm installed.

# For more information, see:
# https://docs.microsoft.com/en-us/azure/devops/pipelines/agents/v2-linux?view=azure-devops#capabilities

# Below, install whatever software and tools are needed to run Azure tasks to build your application

### These are examples only ###
# install zip and unzip in order to run the ArchiveFiles and ExtractFiles tasks:
#RUN apt-get install --no-install-recommends \
#  zip \
#  unzip

# install node 13:
#RUN curl -sL https://deb.nodesource.com/setup_13.x | bash - \
#&& apt-get install --no-install-recommends nodejs

# For other ideas see https://github.com/microsoft/vsts-agent-docker/blob/master/ubuntu/16.04/standard/Dockerfile

### After customizing this Dockerfile ###
#
# Customize and run build.sh to build your agent and push it to Kubernetes:
#
#   $ ./build.sh username:password
#
# Proxy environment variables will be included in the agent image by build.sh using username:password;
# don't explicitly set them here as they would be visible in your image.
#
### Be sure to customize build.sh for your situation, see build.sh for details ###
