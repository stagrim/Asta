FROM node:lts-slim

WORKDIR /app
COPY . .

RUN npm i -g pnpm
RUN pnpm i
RUN pnpm run build

EXPOSE 3000
CMD ["node", "build"]
