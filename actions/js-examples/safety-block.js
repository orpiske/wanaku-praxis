import { block } from 'wanaku:evaluator/response';
import { warn } from 'wanaku:evaluator/log';

export function evaluate(ctx) {
  const reason = `Tool call blocked by safety classification: ${ctx.llmResult}`;
  warn(reason);
  block(reason);
}
