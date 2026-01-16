export function maskText(text: string | null | undefined): string {
	if (!text) return "";
	return text.replace(/\S/g, "*");
}
