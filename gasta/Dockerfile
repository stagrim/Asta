FROM docker.io/denoland/deno:latest

WORKDIR /gasta

COPY package.json deno.json deno.lock /gasta/
RUN deno install --allow-scripts

COPY . /gasta

RUN deno task build

EXPOSE 3000
CMD ["deno", "run", "--allow-env", "--allow-read", "--allow-net", "--allow-write", "build/index.js"]
