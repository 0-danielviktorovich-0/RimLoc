# AI modes, privacy and money

- Local provider (Ollama at http://localhost:11434/v1): free, private,
  recommended default. Once the user has asked for a translation in
  THIS conversation, local jobs may run without a separate
  confirmation; name the model if the user expressed a preference.
- Paid providers (OpenAI/Anthropic/Z.AI/...): keys live ONLY in the
  OS keychain via RimLoc's provider screen. Never accept a key in
  chat; never start a paid job without an explicit go-ahead in THIS
  conversation. Costs scale with rows — state the row count first.
- No-API chat-batch flow (export prompt → external chat → strict
  import) exists in the GUI for users without any API access.
