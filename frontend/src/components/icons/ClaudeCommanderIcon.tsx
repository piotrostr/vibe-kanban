import type { SVGProps } from "react";

export function ClaudeCommanderIcon(props: SVGProps<SVGSVGElement>) {
	return (
		<svg
			width="24"
			height="24"
			viewBox="0 0 24 24"
			fill="none"
			xmlns="http://www.w3.org/2000/svg"
			{...props}
		>
			{/* Body Outline */}
			<path
				d="M6 5C6 4.44772 6.44772 4 7 4H17C17.5523 4 18 4.44772 18 5V7H20C20.5523 7 21 7.44772 21 8V12C21 12.5523 20.5523 13 20 13H18V17H15V19C15 19.5523 14.5523 20 14 20H10C9.44772 20 9 19.5523 9 19V17H6C5.44772 17 5 16.5523 5 16V13H4C3.44772 13 3 12.5523 3 12V8C3 7.44772 3.44772 7 4 7H6V5Z"
				stroke="currentColor"
				strokeWidth="2"
				strokeLinejoin="round"
			/>
			{/* Left Eye > */}
			<path
				d="M8 9L10 10.5L8 12"
				stroke="currentColor"
				strokeWidth="2"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
			{/* Right Eye < */}
			<path
				d="M16 9L14 10.5L16 12"
				stroke="currentColor"
				strokeWidth="2"
				strokeLinecap="round"
				strokeLinejoin="round"
			/>
		</svg>
	);
}
