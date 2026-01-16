import {
	createContext,
	useContext,
	useState,
	useCallback,
	type ReactNode,
} from "react";

interface PrivacyContextValue {
	privacyMode: boolean;
	togglePrivacy: () => void;
}

const PrivacyContext = createContext<PrivacyContextValue | null>(null);

export function PrivacyProvider({ children }: { children: ReactNode }) {
	const [privacyMode, setPrivacyMode] = useState(false);

	const togglePrivacy = useCallback(() => {
		setPrivacyMode((prev) => !prev);
	}, []);

	return (
		<PrivacyContext.Provider value={{ privacyMode, togglePrivacy }}>
			{children}
		</PrivacyContext.Provider>
	);
}

export function usePrivacy(): PrivacyContextValue {
	const context = useContext(PrivacyContext);
	if (!context) {
		throw new Error("usePrivacy must be used within a PrivacyProvider");
	}
	return context;
}
