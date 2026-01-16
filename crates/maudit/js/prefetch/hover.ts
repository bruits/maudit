import { prefetch } from "../prefetch.ts";

const listenedAnchors = new WeakSet<HTMLAnchorElement>();

// TODO: Make this configurable, needs rolldown_plugin_replace and stuff
const observeMutations = true;

function init() {
	let timeout: ReturnType<typeof setTimeout> | null = null;

	// Handle focus listeners for keyboard navigation accessibility
	document.body.addEventListener(
		"focusin",
		(e) => {
			if (e.target instanceof HTMLAnchorElement) {
				handleHoverIn(e);
			}
		},
		{ passive: true },
	);
	document.body.addEventListener("focusout", handleHoverOut, { passive: true });

	// Attach hover listeners to all anchors
	const attachListeners = () => {
		const anchors = document.getElementsByTagName("a");
		for (const anchor of anchors) {
			if (listenedAnchors.has(anchor)) continue;

			listenedAnchors.add(anchor);
			anchor.addEventListener("mouseenter", handleHoverIn, { passive: true });
			anchor.addEventListener("mouseleave", handleHoverOut, { passive: true });
		}
	};

	function handleHoverIn(e: Event) {
		const target = e.target as HTMLAnchorElement;

		if (!target.href) {
			return;
		}

		if (timeout !== null) {
			clearTimeout(timeout);
		}
		timeout = setTimeout(() => {
			prefetch(target.href);
			timeout = null;
		}, 80);
	}

	function handleHoverOut() {
		if (timeout !== null) {
			clearTimeout(timeout);
			timeout = null;
		}
	}

	document.addEventListener("DOMContentLoaded", attachListeners);

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
