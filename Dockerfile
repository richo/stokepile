FROM rustlang/rust:nightly
RUN apt-get update
RUN apt-get -y install libusb-1.0-0-dev
ADD . /app
WORKDIR /app
RUN cargo +nightly build --features=web
CMD ["cargo", "+nightly", "run", "--bin=server", "--features=web"]
