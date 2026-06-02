# Deskbrid MCP Server — Docker image for Glama MCP verification
# Build: docker build -t deskbrid-mcp .
# Run:   docker run -i deskbrid-mcp
# (stdio MCP server — stdin/stdout is the transport)

FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release --bin deskbrid

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/deskbrid /usr/local/bin/deskbrid
COPY --from=builder /app/src/mcp /opt/deskbrid/src/mcp

# Expose nothing — MCP runs over stdio
ENTRYPOINT ["deskbrid", "mcp"]
