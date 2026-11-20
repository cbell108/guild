# NEEDS

Append under your own lane only.

## Lane A (web + store)

## Lane B (frontend)

## Lane C (recommend, llm, ingest, seed, eval, bot)
- (C9, for whoever owns `.env.example`) `llm::DEFAULT_MODEL` / `LLM_MODEL=qwen` is not a model id
  the llama.cpp router knows, so `/api/parse` would return 503. Set `LLM_MODEL` to a router preset,
  e.g. `qwen3.8-27B-UD-Q6_K_M` (the combined layout). `parse_hangout` itself is done and tested.
