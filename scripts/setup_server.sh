#!/bin/bash

# Default values
MODE="status"
REMOTE_HOST="machine01"
LOCAL_PORT="11435"
REMOTE_PORT="11434"

# Parse arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        remote|local|status)
            MODE="$1"
            shift
            ;;
        --remote|-r)
            REMOTE_HOST="$2"
            shift 2
            ;;
        --local-port|-lp)
            LOCAL_PORT="$2"
            shift 2
            ;;
        --remote-port|-rp)
            REMOTE_PORT="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 {remote|local|status} [options]"
            echo "Options:"
            echo "  -r,  --remote <host>        SSH hostname/IP (default: machine01)"
            echo "  -lp, --local-port <port>    Local port for tunnel (default: 11434)"
            echo "  -rp, --remote-port <port>   Remote port for Ollama (default: 11434)"
            exit 0
            ;;
        *)
            echo "Unknown parameter: $1"
            exit 1
            ;;
    esac
done

TUNNEL_CMD="ssh -N -f -L ${LOCAL_PORT}:127.0.0.1:${REMOTE_PORT} ${REMOTE_HOST}"

check_status() {
    if curl -s -f http://127.0.0.1:${LOCAL_PORT}/api/tags > /dev/null; then
        return 0
    else
        return 1
    fi
}

case "$MODE" in
    remote)
        echo "🔄 Routing local port ${LOCAL_PORT} to ${REMOTE_HOST}:${REMOTE_PORT}..."
        
        # 1. Kill any existing tunnel on this specific local port
        pkill -f "ssh -N -f -L ${LOCAL_PORT}:" 2>/dev/null
        
        # 2. Start the SSH tunnel
        echo "   Establishing SSH tunnel..."
        $TUNNEL_CMD
        
        # 3. Verify connection
        sleep 1.5
        if check_status; then
            echo "✅ Successfully connected! Local port ${LOCAL_PORT} is tunneled to ${REMOTE_HOST}."
        else
            echo "❌ Tunnel failed. (Is port ${LOCAL_PORT} already used by your local Ollama?)"
        fi
        ;;
        
    local)
        echo "🔄 Disconnecting tunnel on local port ${LOCAL_PORT}..."
        
        # Tear down the SSH tunnel on this port
        pkill -f "ssh -N -f -L ${LOCAL_PORT}:" 2>/dev/null
        
        echo "✅ Tunnel closed. Port ${LOCAL_PORT} is now freed."
        ;;
        
    status)
        if pgrep -f "ssh -N -f -L ${LOCAL_PORT}:" > /dev/null; then
            echo "🌐 Port ${LOCAL_PORT} is TUNNELED to REMOTE (${REMOTE_HOST}:${REMOTE_PORT})"
        else
            echo "💻 No tunnel found on port ${LOCAL_PORT}."
            if check_status; then
                echo "   Ollama IS responding directly on local port ${LOCAL_PORT} (Local Server)."
            else
                echo "   ⚠️  No service is currently responding on local port ${LOCAL_PORT}."
            fi
        fi
        ;;
esac
