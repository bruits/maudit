/**
 * TODO: This is a quite naive implementation, without necessarily thinking about complex HMR and stuff
 * It might be better to use a more sophisticated approach, using some sort of diffing, handling reconnecting, etc.
 */

import { AnsiUp } from "ansi_up";
import { createErrorOverlay } from "./overlay";
import { error, log } from "./utils";

const WS_SERVER_ADDRESS = "{SERVER_ADDRESS}";

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

const socket = new WebSocket(`ws://${WS_SERVER_ADDRESS}/ws`);

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
