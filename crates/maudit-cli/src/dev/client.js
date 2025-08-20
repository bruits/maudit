/**
 * TODO: This is a quite naive implementation, without necessarily thinking about complex HMR and stuff
 * It might be better to use a more sophisticated approach, using some sort of diffing, handling reconnecting, etc.
 */
const debounceReload = (time) => {
	let timer;
	return () => {
		if (timer) {
			clearTimeout(timer);
			timer = null;
		}
		timer = setTimeout(() => {
			location.reload();
		}, time);
	};
};
const pageReload = debounceReload(50);

const socket = new WebSocket("ws://{SERVER_ADDRESS}/ws");

socket.addEventListener("open", (event) => {
	console.log("Connected to server");
	socket.send("Hello Server!");
});

socket.addEventListener("message", (event) => {
	if (event.data === "done") {
		pageReload();
	}
});
