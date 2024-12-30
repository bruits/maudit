import typography from "@tailwindcss/typography";
import plugin from "tailwindcss/plugin";

/** @type {import('tailwindcss').Config} */
export default {
	content: ["./src/**/*.rs", "./assets/**/*.svg"],
	theme: {
		extend: {
			colors: {
				"our-white": "#F1F1EE",
				"our-black": "#12130F",
				"faded-black": "#1d1e1b",
				"brand-red": "#BA1F33",
				"brighter-brand": "#FA3252",
			},
			gridTemplateColumns: {
				docs: "0.15fr 0.70fr 0.15fr",
			},
			maxWidth: {
				"larger-prose": "75ch",
			},
		},
	},
	plugins: [
		typography,
		plugin(({ addBase, theme }) => {
			addBase({
				"html, body": {
					backgroundColor: "#F1F1EE",
					color: "#12130F",
					height: "100%",
				},

				body: {
					fontFamily:
						"'Iowan Old Style', 'Palatino Linotype', 'URW Palladio L', P052, serif",
				},

				a: {
					"&:hover": {
						color: theme("colors.brand-red"),
					},
				},

				".btn": {
					color: theme("colors.brand-red"),
					fontSize: "1.35rem",
					fontWeight: "bold",
					"&:hover": {
						color: theme("colors.brighter-brand"),
					},
				},

				".hero-background": {
					backgroundImage:
						"url('data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHhtbDpzcGFjZT0icHJlc2VydmUiIHZpZXdCb3g9IjAgMCA0NjYgNDY1Ij48ZGVmcz48Y2xpcFBhdGggaWQ9ImEiIGNsaXBQYXRoVW5pdHM9InVzZXJTcGFjZU9uVXNlIj48cGF0aCBkPSJNMjMgMTRoNDc2djQ3NUgyM1ptMjQ3IDU4Yy0yOCAwLTU1IDYtODEgMjBhMTY0IDE2NCAwIDAgMC03MCAyMjRjNDUgODEgMTQ3IDExMSAyMjkgNjggODMtNDMgMTE0LTE0NCA3MC0yMjRsLTEzIDdjNDAgNzMgMTIgMTY0LTY0IDIwNC03NSAzOS0xNjggMTEtMjA4LTYyLTQxLTc0LTEyLTE2NSA2My0yMDRzMTY5LTExIDIwOSA2MmwxMy03Yy0zMC01Ni04OC04Ny0xNDgtODhabTcyIDI0OSAzNCA2di01MmwtMi01Mi00LTIyLTMtMjIgNC0zIDUtM2MxLTIgMi02IDEtMTItMS00LTEtNS00LTdzLTMtMi04LTJoLTZsLTQgNS00IDV2MTJsNCAzIDMgMi0xMSAzOGMtMTAgMzYtMTEgMzktMTQgMzlsLTI2LTc3IDMtMSA0LTNjMi0zIDEtOC0xLTEzLTItNC0zLTQtNy01bC03IDFjLTMgMy00IDMtNSA3bC0xIDQgMyA0IDMgNC04IDMxLTExIDM3LTMgOWMtMSAyLTQtMy0xNS0yNC0xMS0yMi0xMS0yMi0xNS0zNGwtNC0xNCAzLTdjMy04IDMtMTAgMS0xNS0zLTQtNi02LTEyLTQtNCAwLTQgMS04IDVsLTQgNCAxIDYgNSAxMSAyIDVhMTIzMSAxMjMxIDAgMCAwLTExIDQwbC0zIDEwLTM1LTQ1LTgtMTIgMS0yIDItNy0yLTgtNi0yYy00LTEtNC0xLTcgMS00IDItNCAyLTQgNi0xIDUgMSAxMSA1IDEybDEgMSAyIDY5IDMgNjlhOTA4IDkwOCAwIDAgMSA1OS00bDQxLTIgMjAgMmMyMSAwIDIxIDAgNTMgNnoiIGNsYXNzPSJwb3dlcmNsaXAiIHN0eWxlPSJjb2xvcjojMDAwO2ZpbGw6I2ZmZjtzdHJva2Utd2lkdGg6MTstaW5rc2NhcGUtc3Ryb2tlOm5vbmUiLz48L2NsaXBQYXRoPjwvZGVmcz48cGF0aCBkPSJNNDU3IDQxMWMtNyAwLTE1IDItMjEgNy0zNSAyNi0xMCA3NSAzMSA0MSAzMy0yNyAxNC00OS0xMC00OFpNMjkwIDE5Yy0zMSAwLTY3IDE0LTg3IDI1LTE1IDgtNjQgMzctMTA3IDQ2LTU1IDEwLTg3IDYzLTU1IDkwIDU0IDQ0IDU2IDEwMSAxNiAxNDMtNDQgNDYgMSAxMTMgNjQgMTA2IDYzLTggODkgMCAxMjEgMzIgNDEgNDEgMTMyIDI1IDE1NC0zMyAxMC0yNiAyNC01NiA1OC02OCA0MS0xNCA1My01OSAyNi05MS0xOS0yMy0yMS00NyA1LTk1IDIyLTQwIDUtODQtNTYtODEtNDUgMy02NC0yNy05Ny01OWE1OSA1OSAwIDAgMC00Mi0xNXoiIGNsaXAtcGF0aD0idXJsKCNhKSIgdHJhbnNmb3JtPSJ0cmFuc2xhdGUoLTI4IC0xOSkiIGZpbGw9IiNiYTFmMzMiLz48L3N2Zz4K');",
					backgroundRepeat: "no-repeat",
					backgroundPositionX: "calc(100%)",
					"@media (min-width: 1280px)": {
						backgroundPositionX: "calc(100% - 5rem)",
					},
				},
			});
		}),
	],
};
