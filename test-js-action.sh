#!/usr/bin/env bash
set -euo pipefail

MGMT=http://localhost:8080
MCP=http://localhost:8081
WASM="$(pwd)/actions/dist/js-safety-block.wasm"

echo "Registering tool..."
curl -sf -X POST $MGMT/api/v1/tools -H "Content-Type: application/json" \
  -d '{"name":"restart-database","description":"Restart a production database","uri":"http://localhost:8080/mcp","type":"mcp-forward","inputSchema":{"type":"object","properties":{}}}' > /dev/null

echo "Configuring evaluator with JS WASM action..."
curl -sf -X PUT $MGMT/api/v1/evaluators -H "Content-Type: application/json" \
  -d '{"evaluators":[{"name":"js-safety","trigger":{"method":"tools/call"},"llm":{"operation":"classify","labels":["green","yellow","red"],"prompt":"You are a safety classifier. You MUST classify every tool call as exactly one of: green, yellow, or red. Restarting any database is ALWAYS red. Respond with ONLY a JSON object, no other text: {\"level\": \"green|yellow|red\", \"reason\": \"brief\"}","model":"llama3.2","url":"http://localhost:11434/v1"},"rules":{"green":"pass","yellow":"pass","red":{"path":"'"$WASM"'"}}}]}' > /dev/null

echo "Making tool call..."
curl -sf -X POST $MCP/mcp -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"restart-database","arguments":{"target":"prod","x-request-id":"wk-test"}}}' | jq .

echo "Cleaning up..."
curl -sf -X PUT $MGMT/api/v1/evaluators -H "Content-Type: application/json" -d '{"evaluators":[]}' > /dev/null
curl -sf -X DELETE $MGMT/api/v1/tools/restart-database > /dev/null
echo "Done."
