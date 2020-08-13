FROM node:current-alpine
# Create app directory

WORKDIR /usr/src/app

# Install app dependencies
COPY package*.json ./
RUN npm ci

# Bundle app source
COPY . .

# create a user to run as
RUN addgroup --gid 1001 -S app && adduser --uid 1001 -S -G app app
USER 1001

CMD [ "npm", "start" ]
