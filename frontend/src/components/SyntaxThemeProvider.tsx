import React, { createContext, useContext, useEffect, useState } from "react";

export type SyntaxTheme = "gruvbox" | "github" | "monokai" | "dracula";

const SYNTAX_THEME_STORAGE_KEY = "vibe-syntax-theme";

type SyntaxThemeProviderProps = {
	children: React.ReactNode;
	defaultTheme?: SyntaxTheme;
};

type SyntaxThemeProviderState = {
	syntaxTheme: SyntaxTheme;
	setSyntaxTheme: (theme: SyntaxTheme) => void;
};

const initialState: SyntaxThemeProviderState = {
	syntaxTheme: "gruvbox",
	setSyntaxTheme: () => null,
};

const SyntaxThemeProviderContext =
	createContext<SyntaxThemeProviderState>(initialState);

export function SyntaxThemeProvider({
	children,
	defaultTheme = "gruvbox",
}: SyntaxThemeProviderProps) {
	const [syntaxTheme, setSyntaxThemeState] = useState<SyntaxTheme>(() => {
		// Load from localStorage on init
		const stored = localStorage.getItem(SYNTAX_THEME_STORAGE_KEY);
		if (stored && isValidSyntaxTheme(stored)) {
			return stored;
		}
		return defaultTheme;
	});

	useEffect(() => {
		const root = window.document.documentElement;

		// Remove all syntax theme classes
		root.classList.remove(
			"syntax-gruvbox",
			"syntax-github",
			"syntax-monokai",
			"syntax-dracula",
		);

		// Add current theme class
		root.classList.add(`syntax-${syntaxTheme}`);
	}, [syntaxTheme]);

	const setSyntaxTheme = (newTheme: SyntaxTheme) => {
		setSyntaxThemeState(newTheme);
		localStorage.setItem(SYNTAX_THEME_STORAGE_KEY, newTheme);
	};

	const value = {
		syntaxTheme,
		setSyntaxTheme,
	};

	return (
		<SyntaxThemeProviderContext.Provider value={value}>
			{children}
		</SyntaxThemeProviderContext.Provider>
	);
}

export const useSyntaxTheme = () => {
	const context = useContext(SyntaxThemeProviderContext);

	if (context === undefined)
		throw new Error("useSyntaxTheme must be used within a SyntaxThemeProvider");

	return context;
};

function isValidSyntaxTheme(value: string): value is SyntaxTheme {
	return ["gruvbox", "github", "monokai", "dracula"].includes(value);
}

export const SYNTAX_THEMES: { value: SyntaxTheme; label: string }[] = [
	{ value: "gruvbox", label: "Gruvbox" },
	{ value: "github", label: "GitHub" },
	{ value: "monokai", label: "Monokai" },
	{ value: "dracula", label: "Dracula" },
];
