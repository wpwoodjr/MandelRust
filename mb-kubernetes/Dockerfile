FROM node:12

# Create app directory
WORKDIR /usr/src/app

# Install app dependencies
COPY package*.json ./

#ENV https_proxy=http://xzx61153:Kub3rn3t3s@nodecrypt.gtm.corpnet2.com:800
RUN npm ci

EXPOSE 8081
CMD [ "npm", "start" ]

# Bundle app source
# Put this last to maximize Docker caching
COPY . .
