document.addEventListener("DOMContentLoaded", function () {
	const menuButton = document.getElementById("mobile-menu-button");
	const panel = document.getElementById("mobile-menu-panel");
	const hamburgerIcon = document.getElementById("hamburger-icon");
	const closeIcon = document.getElementById("close-icon");
	let isOpen = false;

	function toggleMenu() {
		if (!panel) return;

		isOpen = !isOpen;

		panel.classList.toggle("-translate-x-4", !isOpen);
		panel.classList.toggle("opacity-0", !isOpen);
		panel.classList.toggle("pointer-events-none", !isOpen);

		if (hamburgerIcon) {
			hamburgerIcon.classList.toggle("hidden", isOpen);
		}
		if (closeIcon) {
			closeIcon.classList.toggle("hidden", !isOpen);
		}

		document.body.style.overflow = isOpen ? "hidden" : "";
	}

	menuButton?.addEventListener("click", toggleMenu);
});
