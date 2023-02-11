# build the Mandelbrot server
FROM alpine as builder

# install Rust
RUN apk add curl build-base \
&& sh <(curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs) -y

# build
COPY mb-arith mb-arith

COPY mb-rust-server mb-rust-server
RUN source "$HOME/.cargo/env" \
&& cd /mb-rust-server \
&& cargo build --release \
&& target/release/mb-rust-server --help

COPY mb-wasm mb-wasm
RUN source "$HOME/.cargo/env" \
&& cd /mb-wasm \
&& rustup target add wasm32-unknown-unknown \
&& cargo build --target wasm32-unknown-unknown --release

# install web app and server
FROM alpine
WORKDIR /usr/src/app

# copy app files and server
COPY client .
COPY --from=builder /mb-rust-server/target/release/mb-rust-server .
COPY --from=builder /mb-wasm/target/wasm32-unknown-unknown/release/mb_wasm.wasm mb-wasm.wasm

# create a user to run as
RUN addgroup --gid 1001 -S app \
&& adduser --uid 1001 -S -G app app \
&& chmod -R a-w,a+r ../app
USER 1001

# start server
ENTRYPOINT ["./mb-rust-server"]
CMD ["0.0.0.0:8081"]
