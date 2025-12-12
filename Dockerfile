# Edition 2024 needs nightly Rust for now.
FROM rustlang/rust:nightly as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

# 默认按中国时区运行（可在 docker-compose 里通过 TZ 覆盖）。
# 注意：若不安装 tzdata，容器通常会以 UTC 作为本地时区，导致基于 Local 的定时任务偏移。
ENV TZ=Asia/Shanghai
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends tzdata \
    && ln -snf /usr/share/zoneinfo/$TZ /etc/localtime \
    && echo $TZ > /etc/timezone \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/tianyi-auto /usr/local/bin/tianyi-auto
ENTRYPOINT ["/usr/local/bin/tianyi-auto"]
