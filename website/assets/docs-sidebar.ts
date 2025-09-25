document.addEventListener("DOMContentLoaded", function () {
	const leftSidebarToggleElement = document.getElementById(
		"left-sidebar-toggle"
	);
	const rightSidebarToggleElement = document.getElementById(
		"right-sidebar-toggle"
	);
	const leftSidebarElement = document.getElementById("mobile-left-sidebar");
	const rightSidebarElement = document.getElementById("mobile-right-sidebar");

	// Early return if required elements are not found
	if (
		!leftSidebarToggleElement ||
		!rightSidebarToggleElement ||
		!leftSidebarElement ||
		!rightSidebarElement
	) {
		throw new Error("Sidebar elements not found in the DOM");
	}

	// Now we can safely assign to non-nullable variables
	const leftSidebarToggle = leftSidebarToggleElement;
	const rightSidebarToggle = rightSidebarToggleElement;
	const leftSidebar = leftSidebarElement;
	const rightSidebar = rightSidebarElement;

	let leftOpen = false;
	let rightOpen = false;

	function toggleLeftSidebar() {
		leftOpen = !leftOpen;

		leftSidebar.classList.toggle("-translate-x-full", !leftOpen);
		leftSidebar.classList.toggle("translate-x-0", leftOpen);
		leftSidebar.classList.toggle("opacity-0", !leftOpen);
		leftSidebar.classList.toggle("opacity-100", leftOpen);
		leftSidebar.classList.toggle("pointer-events-none", !leftOpen);

		if (leftOpen) {
			document.body.style.overflow = "hidden";
		} else if (!rightOpen) {
			document.body.style.overflow = "";
		}
	}

	function toggleRightSidebar() {
		rightOpen = !rightOpen;

		rightSidebar.classList.toggle("translate-x-full", !rightOpen);
		rightSidebar.classList.toggle("translate-x-0", rightOpen);
		rightSidebar.classList.toggle("opacity-0", !rightOpen);
		rightSidebar.classList.toggle("opacity-100", rightOpen);
		rightSidebar.classList.toggle("pointer-events-none", !rightOpen);

		if (rightOpen) {
			document.body.style.overflow = "hidden";
		} else if (!leftOpen) {
			document.body.style.overflow = "";
		}
	}

	// Close sidebars when clicking outside
	function closeSidebars(event: MouseEvent) {
		const target = event.target;
		if (
			leftOpen &&
			target &&
			!leftSidebar.contains(target as Node) &&
			!leftSidebarToggle.contains(target as Node)
		) {
			toggleLeftSidebar();
		}
		if (
			rightOpen &&
			target &&
			!rightSidebar.contains(target as Node) &&
			!rightSidebarToggle.contains(target as Node)
		) {
			toggleRightSidebar();
		}
	}

	leftSidebarToggle.addEventListener("click", toggleLeftSidebar);
	rightSidebarToggle.addEventListener("click", toggleRightSidebar);
	document.addEventListener("click", closeSidebars);

	// Close right sidebar when clicking on table of contents links
	rightSidebar.addEventListener("click", function (event) {
		const target = event.target as HTMLElement;
		if (
			target &&
			target.tagName === "A" &&
			target.getAttribute("href")?.startsWith("#")
		) {
			if (rightOpen) {
				toggleRightSidebar();
			}
		}
	});
});
