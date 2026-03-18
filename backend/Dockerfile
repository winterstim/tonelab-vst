FROM golang:1.22 AS build

WORKDIR /app
COPY go.mod ./
COPY . .

RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o server .

FROM gcr.io/distroless/static-debian12

WORKDIR /app
COPY --from=build /app/server /app/server
COPY --from=build /app/assets /app/assets

ENV EVERGREEN_ASSETS_DIR=/app/assets
ENV EVERGREEN_SIGNATURE_FILE=engine.wasm.sig.b64
ENV EVERGREEN_LISTEN_ADDR=0.0.0.0:8080

EXPOSE 8080

USER nonroot:nonroot

ENTRYPOINT ["/app/server"]
