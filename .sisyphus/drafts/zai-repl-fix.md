# Draft: z.ai REPL Fix

## Requirements (confirmed)
- REPL deve exibir respostas do LLM
- Tool calls devem funcionar (ex: file_read)
- Usando z.ai API endpoint: `https://api.z.ai/api/anthropic/v1/messages`

## Technical Decisions
- **SSE Parsing**: Skip `event:` lines, parse `data:` lines (z.ai sends them separate)
- **Tool Detection**: Check `content_block.type == "tool_use"` (not nested `.tool_use`)
- **Partial JSON**: Accumulate by index, parse incrementally with fallback to `{}`

## Research Findings
- **pi-mono reference**: Uses `@anthropic-ai/sdk` which handles SSE internally
- **z.ai format**: Same events as Anthropic, but `event:` and `data:` on separate lines
- **Tool args**: Arrive via `input_json_delta.partial_json`, must accumulate

## Open Questions
- None - all root causes identified

## Scope Boundaries
- INCLUDE: SSE parsing, tool detection, partial JSON accumulation
- EXCLUDE: Multi-provider support, UI changes, new features

## Plan Created
`.sisyphus/plans/fix-zai-sse-parsing.md`
