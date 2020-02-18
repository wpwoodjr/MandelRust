FROM node:13

# Create app directory
WORKDIR /usr/src/app

# Install app dependencies
COPY package*.json ./
RUN env
RUN cd ~/
RUN ls -lat
RUN ls -lat .docker/
RUN npm ci

EXPOSE 8081
CMD [ "npm", "start" ]

# Bundle app source
# Put this last to maximize Docker caching
COPY . .
