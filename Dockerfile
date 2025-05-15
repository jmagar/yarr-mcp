FROM python:3.13-slim

WORKDIR /app

# Install bash (for entrypoint.sh)
RUN apt-get update && \
    apt-get install -y bash && \
    rm -rf /var/lib/apt/lists/*

# Copy uv binary from the official Astral image to a standard PATH location
COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

# Copy project dependency files
COPY pyproject.toml uv.lock* ./

# Install Python dependencies for the entire project
# uv should now be found in /usr/local/bin/ which is in the default PATH
RUN uv pip install . --system --no-cache-dir

RUN echo "--- INSTALLED PACKAGES (uv pip list) ---" && \
    uv pip list && \
    echo "--- INSTALLED PACKAGES (python3 -m pip list) ---" && \
    python3 -m pip list && \
    echo "--- END INSTALLED PACKAGES ---"

# Copy entrypoint script and make it executable
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Copy MCP service application code
COPY src/gotify-mcp /app/services/gotify-mcp
COPY src/portainer-mcp /app/services/portainer-mcp
COPY src/sabnzbd-mcp /app/services/sabnzbd-mcp
COPY src/unraid-mcp /app/services/unraid-mcp
COPY src/qbittorrent-mcp /app/services/qbittorrent-mcp
COPY src/tautulli-mcp /app/services/tautulli-mcp
COPY src/plex-mcp /app/services/plex-mcp
COPY src/overseerr-mcp /app/services/overseerr-mcp
COPY src/prowlarr-mcp /app/services/prowlarr-mcp
COPY src/unifi-mcp /app/services/unifi-mcp

# Set the entrypoint
ENTRYPOINT ["/app/entrypoint.sh"]