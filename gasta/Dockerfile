FROM node:lts-slim

WORKDIR /
COPY gasta .

RUN npm i -g pnpm
RUN pnpm i
RUN pnpm run build

EXPOSE 3000
CMD ["node", "build"]
