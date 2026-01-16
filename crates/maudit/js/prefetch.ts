const preloadedUrls = new Set<string>();

interface PreloadConfig {
	skipConnectionCheck?: boolean;
}

export function prefetch(url: string, config?: PreloadConfig) {
	let urlObj: URL;
	try {
		urlObj = new URL(url, window.location.href);
		urlObj.hash = "";
	} catch {
		throw new Error(`Invalid URL provided to prefetch: ${url}`);
	}

	const skipConnectionCheck = config?.skipConnectionCheck ?? false;

	if (!canPrefetchUrl(urlObj, skipConnectionCheck)) {
		return;
	}

	const linkElement = document.createElement("link");
	const supportsPrefetch = linkElement.relList?.supports?.("prefetch");

	if (supportsPrefetch) {
		linkElement.rel = "prefetch";
		linkElement.href = url;
		document.head.appendChild(linkElement);
		preloadedUrls.add(urlObj.href);
	}
}

function canPrefetchUrl(url: URL, skipConnectionCheck: boolean): boolean {
	return (
		navigator.onLine && // 1. Don't prefetch if the browser is offline (duh)
		(skipConnectionCheck || !hasLimitedBandwidth()) && // 2. Don't prefetch if the user has limited bandwidth, unless explicitely asked
		window.location.origin === url.origin && // 3. Don't prefetch cross-origin URLs
		!preloadedUrls.has(url.href) && // 4. Don't prefetch URLs we've already prefetched
		(window.location.pathname !== url.pathname || // 5. Don't prefetch the current page (different path or query string)
			window.location.search !== url.search)
	);
}

function hasLimitedBandwidth(): boolean {
	// Chrome thing
	// https://caniuse.com/?search=navigator.connection
	if ("connection" in navigator) {
		const networkInfo = (navigator as any).connection;
		return networkInfo.saveData || networkInfo.effectiveType.endsWith("2g");
	}

	return false;
}
