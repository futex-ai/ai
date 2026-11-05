# Converted user text arrays become strings

- **Severity:** Medium. **Likelihood:** Common. **Effort:** Small. **Confidence:** High. **Harder later:** Yes.
- **Kind:** Needs decision.
- **Added:** 2026-11-02, from branch `calummoore/redesign-4-stream-parts`, item 53 (review finding 53).
- **Location:** `crates/ai-interface/src/messages/legacy_conversion.rs` (`portable_message`), `crates/ai-models-compat/src/engine/message_content.rs:19`, `crates/ai-models-compat/src/engine/messages.rs:94`, `crates/ai-models-openai/src/openai/request_content.rs:13`, `docs/protocol/conversation-parts.md` (section "Converter")
- **Reason:** Every option either changes the stored-format contract, relaxes the rule that the redesign keeps every wire request, needs paid calls, or adds a test for a class of defects, so the user decides.

- **Terms:**
  - **Version 1 and version 2:** the old stored message format with `content` and `content_parts`, and the new one with ordered parts.
  - **String form and array form:** a user message sends its text as one string, or as an array of content parts with one text part.
- **Background:**
  - Before the redesign, a user message with a non-empty `content_parts` list went out in array form, also with one text part, on Kimi, Qwen Plus and Flash, MiniMax M3, Grok 4.5, and OpenAI. A user message with only `content` went out in string form.
  - The converter turns both into the same version 2 message with one text part, and the adapters send one text part in string form. After a host saves a converted conversation as version 2, the original form is lost.
  - Chat Completions and the OpenAI Responses API accept both forms for the same text. `docs/protocol/chat-completions-dialects.md` and the Kimi and MiniMax docs give the string form.
  - The same change of wire form also affects empty content: on xAI, user and tool content `null` became `""`, and assistant content `null` became absent; on MiniMax, user content became `""` where it was absent.
- **Problem:** Converted conversations send a different wire form than before, which breaks the redesign rule that every wire request stays the same.
- **Impact if no action:** If a provider treats the two forms differently, a converted conversation behaves differently, and the change is permanent once hosts save version 2.
- **Root question:** Must a version 2 user message keep the string-or-array form of its text on every adapter that had both forms, or may the library send the string form for the same text?
- **Options:**
  - **A.** Add a small stored marker for the original content form, honour it in request mapping, and update the stored-format contract. Also change the OpenAI mapping. Effort: Medium.
  - **B.** Keep the original user-message representation until the provider maps it. Update storage and the adapter boundaries. Effort: Large.
  - **C.** Accept the string form. Record it in the redesign's list of replaced behaviour, add one sentence to the converter rules, and align the Qwen doc with the Kimi and MiniMax docs. If a provider treats the forms differently, the change is permanent. Effort: Small.
  - **D.** Prove first: send the same text as a string and as a one-element array to Kimi K3, Qwen Plus, MiniMax M3, Grok 4.5, and one OpenAI model, about ten paid calls, and compare the prompt token counts. Save the output. Apply C when every pair matches, and A otherwise. Effort: Small, plus C or A.
  - **E.** Add a conformance test that converts a complete version 1 conversation for each provider and asserts the request of the code before the redesign. It also catches the other conversion defects of this class. Effort: Medium.
- **Left out:** An adapter-only mapping, provider state on the user part, and a migration in the hosts. The adapter cannot tell the forms apart after conversion, a part holds one provider state, and the hosts are out of scope.
- **Wider fix:** E. It catches every converted request that differs, not only this form.
- **Previous examples:** Two other conversion defects of the redesign changed provider requests: mixed-provider messages and repeated MiniMax reasoning items. Both were fixed on 2026-11-02.
- **Recommendation:** D. A cheap one-time proof shows whether A's permanent contract field gives any benefit. When the counts match, C closes the item at the lowest cost.
- **Action:** ask the user, because the options change a contract, relax a rule, or need paid calls.
