# build the Mandelbrot server
FROM alpine as builder

# install Rust
RUN apk add curl build-base \
&& sh <(curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs) -y

# build
WORKDIR mb-rust-server
COPY mb-rust-server .
RUN source "$HOME/.cargo/env" \
&& cargo build --release \
&& target/release/mb-rust-server --help


# install web app and server
FROM alpine
WORKDIR /usr/src/app

# copy app files and server
COPY client .
COPY --from=builder /mb-rust-server/target/release/mb-rust-server .

# create a user to run as
RUN addgroup --gid 1001 -S app \
&& adduser --uid 1001 -S -G app app \
&& chown -R 1001 ../app
USER 1001

# start server
ENTRYPOINT ["./mb-rust-server"]
CMD ["0.0.0.0:8081"]
