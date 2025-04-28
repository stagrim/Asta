FROM denoland/deno:latest

WORKDIR /
COPY gasta .

RUN deno i --allow-scripts && deno run build

EXPOSE 3000
CMD ["deno", "run", "--allow-env", "--allow-read", "--allow-net", "--allow-write", "build/index.js"]
