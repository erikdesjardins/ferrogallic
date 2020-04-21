FROM scratch

ADD ./ferrogallic/target/x86_64-unknown-linux-musl/release/ferrogallic /ferrogallic

ENTRYPOINT ["/ferrogallic"]
CMD ["-v"]
