/**
 * TODO: This is a quite naive implementation, without necessarily thinking about complex HMR and stuff
 * It might be better to use a more sophisticated approach, using some sort of diffing, handling reconnecting, etc.
 */

import { AnsiUp } from "ansi_up";
import { createErrorOverlay } from "./overlay";

const ansiUp = new AnsiUp();

export interface Message {
	type: "success" | "error";
	message: string;
}

const debounceReload = (time: number | undefined) => {
	let timer: number | null | undefined;
	return () => {
		if (timer) {
			clearTimeout(timer);
			timer = null;
		}
		timer = setTimeout(() => {
			location.reload();
		}, time);
	};
};
const pageReload = debounceReload(50);

const socket = new WebSocket("ws://{SERVER_ADDRESS}/ws");

socket.addEventListener("open", (event) => {
	console.log("Connected to server");
	socket.send("Hello Server!");
});

socket.addEventListener("message", (event) => {
	try {
		const message = JSON.parse(event.data) as Message;

		if (message.type === "success") {
			log("Build successful:", message.message);
			pageReload();
		} else if (message.type === "error") {
			error("Build error:", message.message);

			createErrorOverlay(ansiUp.ansi_to_html(message.message));
		}
	} catch (e) {
		error("Failed to parse WebSocket message", event.data, e);
	}
});

function log(...args: any[]) {
	mauditMessage("log", args);
}

function warn(...args: any[]) {
	mauditMessage("warn", args);
}

function error(...args: any[]) {
	mauditMessage("error", args);
}

function mauditMessage(level: "log" | "warn" | "error", message: any[]) {
	console[level](
		`%cMaudit`,
		"background: #ba1f33; color: white; padding-inline: 4px; border-radius: 2px; font-family: serif;",
		message
	);
}
