// PostCSS pipeline for Tailwind CSS. Tailwind generates the utility classes and
// autoprefixer adds vendor prefixes for the WebView2 / WKWebView targets.
export default {
	plugins: {
		tailwindcss: {},
		autoprefixer: {},
	},
};
