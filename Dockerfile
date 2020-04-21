FROM alpine:latest

ADD ./ferrogallic/target/x86_64-unknown-linux-musl/release/ferrogallic /opt/ferrogallic
WORKDIR /opt
ENTRYPOINT exec ./ferrogallic 0.0.0.0:$PORT -v
