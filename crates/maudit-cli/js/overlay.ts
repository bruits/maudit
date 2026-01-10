import type { Message } from "./client";

export const overlayTagName = "maudit-error-overlay";

const template = /*html*/ `
<style>
	#maudit-error-overlay {
		position: fixed;
		z-index: 99999;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background: rgba(0, 0, 0, 0.90);
		margin: 0;
		color: white;
		font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
			"Oxygen", "Ubuntu", "Cantarell", "Fira Sans", "Droid Sans",
			"Helvetica Neue", sans-serif;
		overflow-y: scroll;
		padding: 20px;
		box-sizing: border-box;
		direction: ltr;
	}

	#maudit-error-overlay pre {
    background-color: #070707;
    padding: 1rem;
    border-radius: 4px;
    overflow-x: scroll;
    word-break: break-word;
    font-size: 16px;
    line-height: 1.4;
	}
</style>
<div id="maudit-error-overlay">
	<h1>Build Error</h1>
	<pre id="maudit-error-message"></pre>
</div>
`;

class MauditErrorOverlay extends HTMLElement {
	root: ShadowRoot;

	constructor(err: Message["message"]) {
		super();
		this.root = this.attachShadow({ mode: "open" });
		this.root.innerHTML = template;

		// Set the error message
		const messageElement = this.root.querySelector("#maudit-error-message");
		if (messageElement) {
			messageElement.innerHTML = err.trim();
		}
	}

	close(): void {
		this.parentNode?.removeChild(this);
	}
}

const { customElements } = globalThis;
if (customElements && !customElements.get(overlayTagName)) {
	customElements.define(overlayTagName, MauditErrorOverlay);
}

export function createErrorOverlay(err: Message["message"]) {
	clearErrorOverlay();
	document.body.appendChild(new MauditErrorOverlay(err));
}

export function clearErrorOverlay() {
	document.querySelectorAll<MauditErrorOverlay>(overlayTagName).forEach((n) => n.close());
}
