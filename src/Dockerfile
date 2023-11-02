FROM --platform=linux/arm64 rust:1.73.0-alpine3.17 as builder
WORKDIR /app
COPY /hass-tether-agent /app

RUN apk update && \
  apk upgrade && \
  apk add --no-cache musl-dev cmake make && \
  cargo install --path .

FROM homeassistant/aarch64-homeassistant-base
COPY --from=builder /usr/local/cargo/bin/hass-tether-agent /usr/local/bin/hass-tether-agent
CMD ["hass-tether-agent"]