const preloadedResources = new Set<string>();

interface PreloadConfig {
	skipConnectionCheck?: boolean;
}

export function prefetch(url: string, config?: PreloadConfig) {
	url = url.replace(/#.*/, "");

	const bypassConnectionCheck = config?.skipConnectionCheck ?? false;

	if (!canPrefetchUrl(url, bypassConnectionCheck)) {
		return;
	}

	const linkElement = document.createElement("link");
	const supportsPrefetch = linkElement.relList?.supports?.("prefetch");

	if (supportsPrefetch) {
		linkElement.rel = "prefetch";
		linkElement.href = url;
		document.head.appendChild(linkElement);
		preloadedResources.add(url);
	}
}

function canPrefetchUrl(url: string, bypassConnectionCheck: boolean): boolean {
	if (!navigator.onLine) return false;
	if (!bypassConnectionCheck && hasLimitedBandwidth()) return false;

	try {
		const destination = new URL(url, window.location.href);

		return (
			(window.location.origin === destination.origin &&
				window.location.pathname !== destination.pathname) ||
			(window.location.search !== destination.search && !preloadedResources.has(url))
		);
	} catch {
		return false;
	}
}

function hasLimitedBandwidth(): boolean {
	// Chrome thing
	// https://caniuse.com/?search=navigator.connection
	if ("connection" in navigator) {
		const networkInfo = (navigator as any).connection;
		return networkInfo.saveData || /2g/.test(networkInfo.effectiveType);
	}

	return false;
}
