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
# Debian 官方源在部分网络环境下可能较慢，这里提供可覆盖的镜像域名（只替换域名，不改路径）。
# 用法示例：docker build --build-arg DEBIAN_MIRROR=mirrors.tuna.tsinghua.edu.cn -t tianyi-auto .
ARG DEBIAN_MIRROR=mirrors.ustc.edu.cn
RUN set -eux; \
    # bookworm-slim 默认使用 deb822 格式的 /etc/apt/sources.list.d/debian.sources，可能不存在 /etc/apt/sources.list
    if [ -f /etc/apt/sources.list ]; then \
      sed -i "s|deb.debian.org|${DEBIAN_MIRROR}|g; s|security.debian.org|${DEBIAN_MIRROR}|g" /etc/apt/sources.list; \
    fi; \
    if [ -f /etc/apt/sources.list.d/debian.sources ]; then \
      sed -i "s|deb.debian.org|${DEBIAN_MIRROR}|g; s|security.debian.org|${DEBIAN_MIRROR}|g" /etc/apt/sources.list.d/debian.sources; \
    fi; \
    apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends tzdata \
    && ln -snf /usr/share/zoneinfo/$TZ /etc/localtime \
    && echo $TZ > /etc/timezone \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/tianyi-auto /usr/local/bin/tianyi-auto
ENTRYPOINT ["/usr/local/bin/tianyi-auto"]
