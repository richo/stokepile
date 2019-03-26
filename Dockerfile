FROM rustlang/rust:nightly
RUN apt-get update
RUN apt-get -y install \
  libusb-1.0-0-dev \
  postgresql-client
ADD . /app
WORKDIR /app
# RUN cargo install diesel_cli
RUN cargo +nightly build --features=web --release

FROM alpine:latest
RUN apk --no-cache add ca-certificates
WORKDIR /
COPY web web
COPY --from=0 /app/target .
CMD ["./target/release/server"]
