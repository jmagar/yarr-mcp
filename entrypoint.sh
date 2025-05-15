#!/bin/bash

set -e # Exit immediately if a command exits with a non-zero status.

echo "MCP Entrypoint: Initializing... (Services are ENABLED BY DEFAULT)"

SERVICES_STARTED_COUNT=0
APP_SERVICES_DIR="/app/services"

if [ ! -d "$APP_SERVICES_DIR" ]; then
  echo "MCP Entrypoint: Error - Service directory $APP_SERVICES_DIR not found!" 
  exit 1
fi

for service_dir in $APP_SERVICES_DIR/*-mcp; do
  if [ -d "$service_dir" ]; then
    service_name_full=$(basename "$service_dir") # e.g., gotify-mcp
    service_name_lower=$(echo "$service_name_full" | sed 's/-mcp//') # e.g., gotify
    service_name_upper=$(echo "$service_name_lower" | tr '[:lower:]' '[:upper:]') # e.g., GOTIFY

    disable_var_name="${service_name_upper}_MCP_DISABLE"
    disable_status=${!disable_var_name}
    disable_status_lower=$(echo "$disable_status" | tr '[:upper:]' '[:lower:]')

    server_script="${service_name_lower}-mcp-server.py"

    if [ "$disable_status_lower" = "true" ]; then
      echo "MCP Entrypoint: $service_name_full is explicitly DISABLED (variable $disable_var_name is 'true')."
    else
      echo "MCP Entrypoint: $service_name_full is ENABLED (variable $disable_var_name is '${disable_status:-not set/false}')."
      if [ -f "$service_dir/$server_script" ]; then
        # Check for required port variable
        port_var_name="${service_name_upper}_MCP_PORT"
        port_value=${!port_var_name}
        if [ -z "$port_value" ]; then
          echo "MCP Entrypoint: Warning - ${port_var_name} is not set for $service_name_full. Service might not start correctly or on the expected port."
        fi
        
        # Check for required host variable
        host_var_name="${service_name_upper}_MCP_HOST"
        host_value=${!host_var_name}
        if [ -z "$host_value" ]; then
          echo "MCP Entrypoint: Warning - ${host_var_name} is not set for $service_name_full. Service might default to localhost or an unexpected host."
        fi

        echo "MCP Entrypoint: Starting $server_script in $service_dir..."
        cd "$service_dir"
        python3 "./$server_script" & # MODIFIED: python to python3
        cd .. # Go back to APP_SERVICES_DIR to avoid issues with next iteration if service script changes dir
        SERVICES_STARTED_COUNT=$((SERVICES_STARTED_COUNT + 1))
      else
        echo "MCP Entrypoint: Error - Server script $server_script not found in $service_dir for $service_name_full."
      fi
    fi
  fi
done

echo "-----------------------------------------------------"
if [ "$SERVICES_STARTED_COUNT" -eq 0 ]; then
  echo "MCP Entrypoint: No services were started (either all disabled or none found)."
else
  echo "MCP Entrypoint: $SERVICES_STARTED_COUNT service(s) attempted to start in the background."
fi
echo "MCP Entrypoint: Container will remain active. Monitor individual service logs for details."
echo "-----------------------------------------------------"

# Keep the container alive
tail -f /dev/null 