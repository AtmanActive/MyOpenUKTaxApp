// Tracks whether the window is in portrait orientation (width < height). The
// spec requires the sidebar to move to the bottom as a horizontal taskbar when
// the window is taller than it is wide.

import { useEffect, useState } from "react";

const PORTRAIT_QUERY = "(max-aspect-ratio: 1/1)";

export function use_is_portrait(): boolean
{
	const [is_portrait, set_is_portrait] = useState<boolean>(
		() => window.matchMedia(PORTRAIT_QUERY).matches,
	);

	useEffect(() =>
	{
		const media = window.matchMedia(PORTRAIT_QUERY);
		const handler = () => set_is_portrait(media.matches);
		media.addEventListener("change", handler);
		return () => media.removeEventListener("change", handler);
	}, []);

	return is_portrait;
}
