#!/bin/bash
# ProDnB Web Server Startup Script

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== ProDnB Web Server Startup ===${NC}"
echo ""

# Check if .env file exists
if [ ! -f .env ]; then
    echo -e "${YELLOW}Creating .env file from template...${NC}"
    cp prodnb-web/.env.example .env
    echo -e "${RED}Please edit .env and add your GROQ_API_KEY${NC}"
    echo ""
    echo "Get your API key from: https://console.groq.com/keys"
    echo ""
    exit 1
fi

# Check for GROQ_API_KEY
if ! grep -q "GROQ_API_KEY=your_groq_api_key_here" .env; then
    echo -e "${GREEN}API key configured!${NC}"
else
    echo -e "${RED}GROQ_API_KEY not set in .env file${NC}"
    echo "Please edit .env and add your Groq API key"
    echo "Get your key from: https://console.groq.com/keys"
    exit 1
fi

# Build if necessary
if [ ! -f target/release/prodnb-web ]; then
    echo -e "${YELLOW}Building ProDnB Web Server...${NC}"
    cargo build --release --package prodnb-web
fi

# Start the server
echo -e "${GREEN}Starting ProDnB Web Server...${NC}"
echo ""
echo "Access the web UI at: http://127.0.0.1:8080"
echo ""
echo "Press Ctrl+C to stop the server"
echo ""

./target/release/prodnb-web
