FROM rust:1.77.2-slim-bullseye AS builder

WORKDIR /app

RUN apt update && apt install lld clang -y

COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release

# ---------------------------------

FROM debian:bullseye-slim AS runtime

WORKDIR /app

# install open-ssl, it is linked to one of our dependencies
# install ca certificates, it is needed to establish https connections
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/actix_server actix_server
COPY configuration configuration

ENV APP_ENVIRONMENT production

ENTRYPOINT ["./actix_server"]