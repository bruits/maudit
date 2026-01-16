import { prefetch } from "../prefetch.ts";

const prefetchedAnchors = new WeakSet<HTMLAnchorElement>();
const observedAnchors = new WeakSet<HTMLAnchorElement>();

// TODO: Make this configurable, needs rolldown_plugin_replace and stuff
const observeMutations = true;

function init() {
	let intersectionObserver: IntersectionObserver | null = null;

	function createIntersectionObserver(): IntersectionObserver {
		const timeouts = new WeakMap<HTMLAnchorElement, ReturnType<typeof setTimeout>>();

		return new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					const anchor = entry.target as HTMLAnchorElement;
					const existingTimeout = timeouts.get(anchor);

					// Clear any pending timeout
					if (existingTimeout) {
						clearTimeout(existingTimeout);
						timeouts.delete(anchor);
					}

					if (entry.isIntersecting) {
						// Skip if already prefetched
						if (prefetchedAnchors.has(anchor)) {
							intersectionObserver?.unobserve(anchor);
							continue;
						}

						// Debounce by 300ms to avoid prefetching during rapid scrolling
						const timeout = setTimeout(() => {
							timeouts.delete(anchor);
							if (!prefetchedAnchors.has(anchor)) {
								prefetchedAnchors.add(anchor);
								prefetch(anchor.href);
							}
							intersectionObserver?.unobserve(anchor);
						}, 300);

						timeouts.set(anchor, timeout);
					}
					// If exited viewport, timeout already cleared above
				}
			},
			{
				// Prefetch slightly before element enters viewport for smoother UX
				rootMargin: "50px",
				// Only trigger when at least 10% of the link is visible
				threshold: 0.1,
			},
		);
	}

	function observeAnchors() {
		intersectionObserver ??= createIntersectionObserver();

		const anchors = document.getElementsByTagName("a");
		for (const anchor of anchors) {
			// Skip if already observing or has no href
			if (observedAnchors.has(anchor) || !anchor.href) continue;

			observedAnchors.add(anchor);
			intersectionObserver.observe(anchor);
		}
	}

	// This is always in a type="module" script, so, it'll always run after the DOM is ready
	observeAnchors();

	if (observeMutations) {
		// Watch for dynamically added anchors
		const mutationObserver = new MutationObserver((mutations) => {
			let hasNewAnchors = false;
			for (const mutation of mutations) {
				if (mutation.type === "childList" && mutation.addedNodes.length > 0) {
					// Check if any added nodes are or contain anchors
					for (const node of mutation.addedNodes) {
						if (node.nodeType === Node.ELEMENT_NODE) {
							const element = node as Element;
							if (element.tagName === "A" || element.getElementsByTagName("a").length > 0) {
								hasNewAnchors = true;
								break;
							}
						}
					}
				}
				if (hasNewAnchors) break;
			}

			if (hasNewAnchors) {
				observeAnchors();
			}
		});

		mutationObserver.observe(document.body, {
			childList: true,
			subtree: true,
		});
	}
}

init();
