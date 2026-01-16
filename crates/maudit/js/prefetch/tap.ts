import { prefetch } from "../prefetch.ts";

const listenedAnchors = new WeakSet<HTMLAnchorElement>();

// TODO: Make this configurable, needs rolldown_plugin_replace and stuff
const observeMutations = true;

function init() {
	// Attach touchstart/mousedown listeners to all anchors
	const attachListeners = () => {
		const anchors = document.getElementsByTagName("a");
		for (const anchor of anchors) {
			if (listenedAnchors.has(anchor)) continue;

			listenedAnchors.add(anchor);
			anchor.addEventListener("touchstart", handleTap, { passive: true });
			anchor.addEventListener("mousedown", handleTap, { passive: true });
		}
	};

	document.addEventListener("DOMContentLoaded", attachListeners);

	function handleTap(e: TouchEvent | MouseEvent) {
		const target = e.currentTarget as HTMLAnchorElement;

		if (!target.href) {
			return;
		}

		// Prefetch on tap/mousedown
		prefetch(target.href);
	}

	if (observeMutations) {
		// Re-attach listeners for dynamically added content
		const observer = new MutationObserver((mutations) => {
			for (const mutation of mutations) {
				if (mutation.type === "childList" && mutation.addedNodes.length > 0) {
					attachListeners();
					break;
				}
			}
		});

		observer.observe(document.body, {
			childList: true,
			subtree: true,
		});
	}
}

init();
