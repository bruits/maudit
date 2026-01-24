declare global {
	interface Window {
		prefetch: (url: string, options?: { prerender?: boolean; eagerness?: string }) => void;
	}
}

export {};
