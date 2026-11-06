# Juno adopts the redesigned API and format

- **Severity:** Medium. **Effort:** Large. **Confidence:** Medium. **Harder later:** No.
- **Kind:** Post-merge task.
- **Added:** 2026-11-05, from branch `calummoore/redesign-8-conformance`: the unchecked post-merge task "Juno adopts the new API and converts its stored conversations" of the AI library redesign plan, which this branch deletes.
- **Location:** None in this repository. The work is in the Juno repository. It calls `Registry::converter` in `crates/ai-models-registry/src/registry.rs` and `LegacyMessageConverter::convert` in `crates/ai-interface/src/messages/legacy_message_converter.rs`. `docs/protocol/conversation-parts.md` (sections "Stored format" and "Converter") gives the rules.
- **Reason:** Juno is a separate repository that pins Git revisions of these crates, so it can adopt the redesigned API and convert its stored conversations only after the redesign merges to `main`.

- **Terms:**
  - **Juno:** a downstream host application, in its own repository, that depends on the crates of this workspace and stores model conversations as JSON.
  - **Version 1 and version 2:** the old stored message format and the new one. A version 1 message has no `format_version` and has the fields `content`, `content_parts`, `name`, `tool_call_id`, `tool_calls`, and `provider_context`. A version 2 message has `format_version: 2`, `role`, ordered `parts`, and an optional `name`.
  - **Converter:** the one-way function that reads a version 1 message and returns a version 2 message. No function writes version 1.
- **Background:**
  - The AI library redesign, which the user approved on 2026-11-01, changed the public API of this workspace and its stored conversation format. This workspace does not change Juno.
  - `plans/follow-ups/firna-adopts-redesigned-api.md` lists every removed or replaced name and behaviour of the redesign. The same list applies to Juno.
  - Reading accepts versions 1 and 2. Reading a version 1 message applies the converter that the registry exposes: `Registry::converter()` returns a `LegacyMessageConverter`, and its `convert` method turns a `LegacyConversationMessage` into a `ConversationMessage`. Writing produces version 2 only. Reading another version fails with a typed error.
  - `ModelRouter`, `activity_verb`, and `group_for_tool` stay, because Juno and Firna use them.
  - Two open decisions concern facts that the converter does not keep. After Juno writes a converted conversation as version 2, those facts are gone:
    - `plans/follow-ups/user-text-array-form.md`: the converter gives the same version 2 message for a user message with one text in `content_parts` and for one with only `content`. Before the redesign, Kimi, Qwen Plus and Flash, MiniMax M3, Grok 4.5, and OpenAI sent the first as an array of content parts and the second as one string. Now both go out as one string.
    - `plans/follow-ups/legacy-replay-per-provider.md`: in a version 1 message with replay items of more than one provider, the provider of the first item sets the part order. Some providers then get a different request than before. Only a host that edits or merges messages creates such a message.
  - This task does not cover the Juno migration of the tool output windows. `docs/protocol/tool-output-management.md` tracks that migration.
- **Task:** Pin Juno to a revision of `main` that holds the redesign. Move each use of a removed name to its replacement. Read every stored conversation through the converter. Write version 2 only. Before Juno overwrites a version 1 conversation with its version 2 form, get the user's answers to the two open decisions above, or keep the version 1 original, because the answers can need facts that version 2 does not hold.
- **Impact if no action:** Juno stays on a revision before the redesign, and it gets no later fix or feature of this workspace.
- **Needs:** The redesign merged to `main`, access to the Juno repository, and the answers to `plans/follow-ups/user-text-array-form.md` and `plans/follow-ups/legacy-replay-per-provider.md` before Juno drops any version 1 original.
- **Done when:** Juno pins a revision of `main` that holds the redesign, reads every stored conversation through the converter, writes version 2 only, and its tests pass.
- **Related:** `plans/follow-ups/firna-adopts-redesigned-api.md` is the same task for Firna. `plans/follow-ups/user-text-array-form.md` and `plans/follow-ups/legacy-replay-per-provider.md` decide what the conversion must keep.
