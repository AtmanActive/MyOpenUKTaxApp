// A minimal toast/notification store.
//
// Command failures and confirmations are surfaced as transient toasts rendered
// by the Toaster component. Kept deliberately tiny: an array of messages with
// auto-expiry, no external toast dependency.

import { create } from "zustand";

export type NotifyKind = "info" | "success" | "error";

export interface NotifyMessage
{
	id: number;
	kind: NotifyKind;
	text: string;
}

interface NotifyState
{
	messages: NotifyMessage[];
	push: (kind: NotifyKind, text: string) => void;
	dismiss: (id: number) => void;
}

// Monotonic id generator for toast messages.
let next_id = 1;

export const use_notify = create<NotifyState>((set) => ({
	messages: [],
	push: (kind, text) =>
	{
		const id = next_id++;
		set((state) => ({ messages: [...state.messages, { id, kind, text }] }));
		// Auto-dismiss after a few seconds so toasts never pile up forever.
		window.setTimeout(() =>
		{
			set((state) => ({ messages: state.messages.filter((message) => message.id !== id) }));
		}, 4500);
	},
	dismiss: (id) =>
		set((state) => ({ messages: state.messages.filter((message) => message.id !== id) })),
}));

// Convenience helper to turn an unknown error (commands reject with a string)
// into a readable toast message.
export function notify_error(error: unknown): void
{
	const text = typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
	use_notify.getState().push("error", text);
}
