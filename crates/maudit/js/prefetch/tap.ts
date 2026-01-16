import { prefetch } from "../prefetch.ts";

const listenedAnchors = new WeakSet<HTMLAnchorElement>();

// TODO: Make this configurable, needs rolldown_plugin_replace and stuff
const observeMutations = true;

function init() {
	// Attach click listeners to all anchors
	const attachListeners = () => {
		const anchors = document.getElementsByTagName("a");
		for (const anchor of anchors) {
			if (listenedAnchors.has(anchor)) continue;

			listenedAnchors.add(anchor);
			anchor.addEventListener("click", handleClick, { passive: true });
		}
	};

	document.addEventListener("DOMContentLoaded", attachListeners);

	function handleClick(e: Event) {
		const target = e.target as HTMLAnchorElement;

		if (!target.href) {
			return;
		}

		// Prefetch on click
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
