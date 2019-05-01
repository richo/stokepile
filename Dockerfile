FROM renderco/rocket-rust
RUN apt-get update
RUN apt-get -y install \
  libusb-1.0-0-dev \
  postgresql-client
ADD . /app
WORKDIR /app
# RUN cargo install diesel_cli
RUN cargo +nightly build --features=web --release
CMD ["./target/release/server"]
