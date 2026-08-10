// Safety block action — blocks tool calls classified as dangerous.
//
// Compile: javy build safety-block.js -o safety-block.wasm
//
// The host writes the evaluation context as JSON to stdin.
// This action reads it and writes the action result to stdout.

const input = Javy.IO.readSync();
const ctx = JSON.parse(new TextDecoder().decode(input));

const result = {
  action: "block",
  reason: "Tool call blocked by safety classification: " + ctx.llmResult
};

const output = new TextEncoder().encode(JSON.stringify(result));
Javy.IO.writeSync(output);
