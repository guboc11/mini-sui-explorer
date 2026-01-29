# syntax=docker/dockerfile:1

FROM golang:1.23-jammy AS builder

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o /app/backend-server ./...

FROM ubuntu:22.04

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/backend-server /app/backend-server

ENV PORT=8080

CMD ["/app/backend-server"]
