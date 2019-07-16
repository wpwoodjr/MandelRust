FROM nginx:alpine

# Create app directory
WORKDIR /usr/share/nginx/html/

EXPOSE 8082

# Bundle app source
# Put this last to maximize Docker caching
COPY . .
