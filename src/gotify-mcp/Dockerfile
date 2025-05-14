# Use an official Python runtime as a parent image
FROM python:3.13-slim

# Set the working directory in the container
WORKDIR /app

# Copy the requirements file into the container at /app
COPY requirements.txt .

# Install any needed packages specified in requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# Copy the server script and the example environment file into the container at /app
COPY gotify_mcp_server.py .
COPY .env.example .env

# Make port 8000 available to the world outside this container
# This is the default port specified in gotify_mcp_server.py and .env.example
EXPOSE 8000

# Define environment variables (optional, can be overridden at runtime)
# These are just examples, users should configure their .env file or pass variables at runtime
ENV GOTIFY_MCP_TRANSPORT=sse
ENV GOTIFY_MCP_HOST=0.0.0.0
ENV GOTIFY_MCP_PORT=8000
# Users need to set GOTIFY_URL and GOTIFY_CLIENT_TOKEN via .env file or runtime variables

# Run gotify_mcp_server.py when the container launches
CMD ["python", "gotify_mcp_server.py"] 