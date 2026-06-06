// Tailwind CSS configuration. Uses class-based dark mode (the theme provider
// toggles the `dark` class on <html>) and maps colour utilities onto the CSS
// custom properties declared in src/index.css so themes can be swapped at runtime.
import tailwindcss_animate from "tailwindcss-animate";

/** @type {import('tailwindcss').Config} */
export default {
	darkMode: ["class"],
	content: ["./index.html", "./src/**/*.{ts,tsx}"],
	theme: {
		extend: {
			colors: {
				border: "hsl(var(--border))",
				input: "hsl(var(--input))",
				ring: "hsl(var(--ring))",
				background: "hsl(var(--background))",
				foreground: "hsl(var(--foreground))",
				primary: {
					DEFAULT: "hsl(var(--primary))",
					foreground: "hsl(var(--primary-foreground))",
				},
				secondary: {
					DEFAULT: "hsl(var(--secondary))",
					foreground: "hsl(var(--secondary-foreground))",
				},
				destructive: {
					DEFAULT: "hsl(var(--destructive))",
					foreground: "hsl(var(--destructive-foreground))",
				},
				muted: {
					DEFAULT: "hsl(var(--muted))",
					foreground: "hsl(var(--muted-foreground))",
				},
				accent: {
					DEFAULT: "hsl(var(--accent))",
					foreground: "hsl(var(--accent-foreground))",
				},
				popover: {
					DEFAULT: "hsl(var(--popover))",
					foreground: "hsl(var(--popover-foreground))",
				},
				card: {
					DEFAULT: "hsl(var(--card))",
					foreground: "hsl(var(--card-foreground))",
				},
				// Semantic colours used by the Income / Expenses tab-switch and tables.
				income: {
					DEFAULT: "hsl(var(--income))",
					foreground: "hsl(var(--income-foreground))",
				},
				expense: {
					DEFAULT: "hsl(var(--expense))",
					foreground: "hsl(var(--expense-foreground))",
				},
			},
			borderRadius: {
				lg: "var(--radius)",
				md: "calc(var(--radius) - 2px)",
				sm: "calc(var(--radius) - 4px)",
			},
			keyframes: {
				// Used by the "flashing text" widget required by the overview doc.
				flash: {
					"0%, 100%": { opacity: "1" },
					"50%": { opacity: "0.25" },
				},
			},
			animation: {
				flash: "flash 1.2s ease-in-out infinite",
			},
		},
	},
	plugins: [tailwindcss_animate],
};
