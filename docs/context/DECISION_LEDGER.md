# Decision Ledger

VTCode maintains a compact, structured record of key decisions during a session and injects it into the system prompt each turn. This improves reliability by keeping implicit choices explicit and visible to the model.

- What’s tracked: current goal, major tool calls (name + args preview), compression/recovery events, and brief outcomes.
- Where it appears: a `[Decision Ledger]` section is appended to the system prompt for each model turn.
- Preservation: the context compressor explicitly preserves ledger messages so they are not pruned.

Configuration (preview): defaults are conservative. Future releases will expose knobs under `[context.ledger]`.

Design goals:
- Keep entries brief and bounded (recent N only).
- Favor high-signal events over verbose transcripts.
- Complement, not replace, trajectory logs.

