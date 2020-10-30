# https://github.com/nodejs/docker-node/issues/1382 so using lts instead of current
FROM node:lts-alpine
# Create app directory

WORKDIR /usr/src/app

# Install app dependencies
COPY package.json ./
RUN npm install

# Bundle app source
COPY . .

# create a user to run as
RUN addgroup --gid 1001 -S app && adduser --uid 1001 -S -G app app
USER 1001

CMD [ "npm", "start" ]
