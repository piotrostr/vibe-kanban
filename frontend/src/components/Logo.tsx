interface LogoProps {
	size?: number;
	className?: string;
}

export function Logo({ size = 32, className = "" }: LogoProps) {
	return (
		<img
			src="/vibe-192.png"
			alt="Vibe"
			width={size}
			height={size}
			className={className}
		/>
	);
}
