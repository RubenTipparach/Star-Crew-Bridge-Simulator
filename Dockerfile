# The matchmaker on Fly.io (openspec/changes/matchmaker design 5 and 10): only sc-matchmaker is built.
# The crews' game servers run on their creators' machines; this lists them and relays the joins.
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p sc-matchmaker

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /src/target/release/sc-matchmaker /app/sc-matchmaker
COPY data/matchmaker /app/data/matchmaker
ENV PORT=8080
EXPOSE 8080
CMD ["/app/sc-matchmaker", "--root", "/app"]
