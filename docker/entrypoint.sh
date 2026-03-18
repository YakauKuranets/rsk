#!/bin/bash
# docker/entrypoint.sh
set -e


echo "Starting Hyperion Ultimate..."


# Start Ollama in background
ollama serve &
sleep 3


# Pull default model if not present
if ! ollama list | grep -q "llama3"; then
    echo "Pulling llama3 model (first run, may take a few minutes)..."
    ollama pull llama3:8b
fi


# Start Metasploit RPC daemon
msfrpcd -P hyperion-msf-pass -u hyperion -f -a 127.0.0.1 &
sleep 2


# Update nuclei templates
nuclei -update-templates -silent || true


# Start Hyperion REST API
echo "Hyperion REST API starting on :7878"
/hyperion/src-tauri/target/release/hyperion --headless --api-port 7878 --api-key "$HYPERION_API_KEY"

