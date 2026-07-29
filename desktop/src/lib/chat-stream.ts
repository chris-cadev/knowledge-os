import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ChatDelta, ProcessingStatus, Citation } from "./types.js";
import { chatStream as invokeChatStream } from "./api.js";

export interface StreamCallbacks {
  onStatus?: (status: ProcessingStatus) => void;
  onDelta?: (delta: ChatDelta) => void;
  onDone?: (info: {
    userMessageId: string;
    assistantMessageId: string;
    citations: Citation[];
  }) => void;
  onError?: (error: string) => void;
}

export class ChatStreamSession {
  private unlistens: UnlistenFn[] = [];
  private conversationId: string | null = null;

  async start(
    conversationId: string | null,
    message: string,
    entityRefs: string[],
    sourceToggles: { knowledge_graph: boolean; web_search: boolean },
    mode: "fast" | "thinking",
    callbacks: StreamCallbacks
  ): Promise<string> {
    await this.stop();

    this.unlistens.push(
      await listen<ProcessingStatus>("chat:status", (event) => {
        callbacks.onStatus?.(event.payload);
      })
    );

    this.unlistens.push(
      await listen<ChatDelta>("chat:delta", (event) => {
        callbacks.onDelta?.(event.payload);
      })
    );

    this.unlistens.push(
      await listen<{
        user_message_id: string;
        assistant_message_id: string;
        citations: Citation[];
      }>("chat:done", (event) => {
        callbacks.onDone?.({
          userMessageId: event.payload.user_message_id,
          assistantMessageId: event.payload.assistant_message_id,
          citations: event.payload.citations,
        });
        this.stop();
      })
    );

    this.unlistens.push(
      await listen<string>("chat:error", (event) => {
        callbacks.onError?.(event.payload);
        this.stop();
      })
    );

    this.conversationId = await invokeChatStream(
      conversationId,
      message,
      entityRefs,
      sourceToggles,
      mode
    );

    return this.conversationId;
  }

  async stop(): Promise<void> {
    for (const unlisten of this.unlistens) {
      unlisten();
    }
    this.unlistens = [];

    if (this.conversationId) {
      try {
        const { chatStopStream } = await import("./api.js");
        await chatStopStream(this.conversationId);
      } catch {
        // Ignore errors during cleanup
      }
    }
  }

  getConversationId(): string | null {
    return this.conversationId;
  }
}
