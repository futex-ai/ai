# Mixed version 1 messages share one part order

- **Severity:** Medium. **Likelihood:** Rare. **Effort:** Large. **Confidence:** Medium. **Harder later:** Yes.
- **Kind:** Needs decision.
- **Added:** 2026-11-02, from branch `calummoore/redesign-4-stream-parts`, item 55: the blocked wider fix of review finding 28.
- **Location:** `crates/ai-interface/src/messages/legacy_conversion.rs`, the version 1 conversions of each provider crate, `docs/protocol/conversation-parts.md` (section "Converter", the rule that the provider of the first item sets the order)
- **Reason:** The fix changes the order rule of `docs/protocol/conversation-parts.md`, so the user decides.

- **Terms:**
  - **Version 1 message:** a stored message of the old format, whose `provider_context` list holds replay items of one or more providers.
  - **Mixed message:** a version 1 message with items of more than one provider.
  - **Full content item:** an Anthropic or Google item that holds the whole ordered assistant content.
- **Background:**
  - The converter turns a version 1 message into ordered parts. In a mixed message, the provider of the first item sets the order of the parts, and the items of the other providers attach to those parts as their provider state.
  - On 2026-11-02, the fix of review finding 28 kept every full content item of a mixed message whole on one reasoning part, so that Anthropic and Google get their old request. Other item shapes of a mixed message still share one part order.
  - Only a host that edits or merges messages creates a mixed message. The tool loop and the fallback router of the old code never did.
- **Problem:** One shared part order cannot keep every provider's native order in a mixed message. Each new item shape that needs its own order needs its own exception. A current case: when a DeepSeek, Kimi, Qwen, MiniMax, or xAI item comes first and OpenAI items follow, the OpenAI request changes from `message, function_call, reasoning` (version 1) to `reasoning, message, function_call`.
- **Impact if no action:** A mixed message whose first item belongs to a Chat Completions dialect already changes its OpenAI request after conversion, and a future provider item can change its provider's request too, until someone adds another exception.
- **Root question:** How can every provider of a mixed version 1 message get its own original request after conversion?
- **Options:**
  - **A.** Keep the current rule, with the exception for full content items. Effort: Small.
  - **B.** Keep the replay of each provider apart for any item shape: each provider's items convert into their own replay carrier, and the portable parts stay separate. The OpenAI and Chat Completions adapters then need a whole-message replay mode. Change the order rule of `conversation-parts.md`. Effort: Large.
- **Left out:** Reject mixed messages: it adds a rejection and breaks the rule that the converter keeps every stored item.
- **Wider fix:** B.
- **Previous examples:** Review findings 28 and 41 (mixed-provider conversions that reordered and duplicated signed content), 52 (repeated MiniMax reasoning items that were joined), 56 (OpenAI items reordered after a full content item), and 67 (an OpenAI call that version 1 did not send), fixed on 2026-11-02.
- **Related:** `plans/follow-ups/user-text-array-form.md`, the same class of conversion defects.
- **Recommendation:** B, before more provider item shapes depend on the shared order.
- **Action:** ask the user, because B changes the order rule of `conversation-parts.md`.
