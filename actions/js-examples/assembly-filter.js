// Assembly filter action — filters tools based on LLM result.
//
// Compile: javy build assembly-filter.js -o assembly-filter.wasm
//
// The host writes the evaluation context as JSON to stdin.
// The LLM result should be a JSON array of approved tool names.

const input = Javy.IO.readSync();
const ctx = JSON.parse(new TextDecoder().decode(input));

let approved;
try {
  approved = JSON.parse(ctx.llmResult);
} catch (e) {
  // If LLM result isn't a valid JSON array, pass through (fail-open)
  const output = new TextEncoder().encode(JSON.stringify({ action: "pass" }));
  Javy.IO.writeSync(output);
  // Exit early — Javy doesn't support return in top-level scope
}

if (approved && Array.isArray(approved) && approved.length > 0) {
  const result = {
    action: "filter_tools",
    tools: approved
  };
  const output = new TextEncoder().encode(JSON.stringify(result));
  Javy.IO.writeSync(output);
} else {
  const output = new TextEncoder().encode(JSON.stringify({ action: "pass" }));
  Javy.IO.writeSync(output);
}
