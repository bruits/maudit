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
	if (!navigator.onLine) return false;
	if (!skipConnectionCheck && hasLimitedBandwidth()) return false;

	return (
		(window.location.origin === url.origin && window.location.pathname !== url.pathname) ||
		(window.location.search !== url.search && !preloadedUrls.has(url.href))
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
