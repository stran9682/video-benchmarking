FROM rust:1.93

COPY ./ ./

RUN cargo build --release

CMD ["./target/release/video-server"]

EXPOSE 8084/tcp
EXPOSE 8083/udp
EXPOSE 8082/udp
EXPOSE 8081/udp
EXPOSE 8080/udp