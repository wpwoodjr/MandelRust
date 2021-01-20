FROM node:lts-alpine
# Create app directory

WORKDIR /usr/src/app

# Bundle app source
COPY . .

# create a user to run as
RUN addgroup --gid 1001 -S app && adduser --uid 1001 -S -G app app \
&& chown -R 1001 ../app
USER 1001

RUN npm install

CMD [ "npm", "start" ]
