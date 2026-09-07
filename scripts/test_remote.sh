#!/bin/bash

PORT="11434"
TEST_GENERATE=false

# Parse arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --port|-p)
            PORT="$2"
            shift 2
            ;;
        --generate|-g)
            TEST_GENERATE=true
            shift 1
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  -p, --port <port>    Local port the remote server is tunneled to (default: 11434)"
            echo "  -g, --generate       Test a quick prompt generation against the first available model"
            exit 0
            ;;
        *)
            echo "Unknown parameter: $1"
            exit 1
            ;;
    esac
done

URL="http://127.0.0.1:${PORT}"

echo "📡 Testing connection to Ollama on port ${PORT}..."
echo "--------------------------------------------------"

# 1. Test basic connectivity and get models
RESPONSE=$(curl -s -m 5 "${URL}/api/tags")

if [ $? -ne 0 ] || [ -z "$RESPONSE" ]; then
    echo "❌ Connection FAILED."
    echo "Please check:"
    echo "  1. Is the SSH tunnel active? (Run: ./toggle_ai.sh status -lp $PORT)"
    echo "  2. Is the Ollama service running on the remote machine?"
    exit 1
fi

echo "✅ Connection SUCCESSFUL!"
echo ""
echo "📦 Models available on this server:"

# Extract model names using grep/sed (avoids needing 'jq' installed)
MODELS=$(echo "$RESPONSE" | grep -o '"name":"[^"]*"' | sed 's/"name":"//' | sed 's/"//')

if [ -z "$MODELS" ]; then
    echo "   (No models found. You may need to run 'ollama run <model>' on the remote server.)"
    exit 0
else
    echo "$MODELS" | sed 's/^/   - /'
fi

# 2. Optional: Test Text Generation
if [ "$TEST_GENERATE" = true ]; then
    echo ""
    echo "🧠 Testing text generation..."
    
    # Grab the first model from the list
    FIRST_MODEL=$(echo "$MODELS" | head -n 1)
    
    if [ -n "$FIRST_MODEL" ]; then
        echo "   Sending test prompt to '$FIRST_MODEL'..."
        
        # Send a quick generation request (stream: false makes it wait for the full response)
        GEN_RESPONSE=$(curl -s -m 30 "${URL}/api/generate" \
            -H "Content-Type: application/json" \
            -d "{\"model\": \"$FIRST_MODEL\", \"prompt\": \"Say 'Hello from the remote server!'\", \"stream\": false}")
            
        if [ $? -eq 0 ]; then
            # Extract just the response text
            OUTPUT=$(echo "$GEN_RESPONSE" | grep -o '"response":"[^"]*"' | sed 's/"response":"//' | sed 's/"//' | sed 's/\\n/\n/g')
            echo "   Response: \"$OUTPUT\""
            echo "✅ Generation successful!"
        else
            echo "❌ Generation failed. (Timeout or model loading error)"
        fi
    fi
fi
