FROM denoland/deno:alpine

USER deno

WORKDIR /gasta

COPY --chown=deno:deno gasta/package.json gasta/deno.json gasta/deno.lock ./

RUN deno install --allow-scripts

COPY --chown=deno:deno gasta .

RUN deno run build

EXPOSE 3000

CMD ["deno", "run", "--allow-env", "--allow-read", "--allow-net", "--allow-write", "build/index.js"]
