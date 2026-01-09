import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { cloneDeep, isEqual } from "lodash";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Loader2, ExternalLink, Eye, EyeOff } from "lucide-react";
import { useUserSystem } from "@/components/ConfigProvider";

export function IntegrationsSettings() {
	const { t } = useTranslation(["settings", "common"]);

	const { config, loading, updateAndSaveConfig } = useUserSystem();

	const [draft, setDraft] = useState(() => (config ? cloneDeep(config) : null));
	const [dirty, setDirty] = useState(false);
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [success, setSuccess] = useState(false);
	const [showLinearKey, setShowLinearKey] = useState(false);
	const [showSentryToken, setShowSentryToken] = useState(false);

	useEffect(() => {
		if (config && !draft) {
			setDraft(cloneDeep(config));
		}
	}, [config, draft]);

	useEffect(() => {
		if (config && draft) {
			// Compare integrations including API keys (which are not in TS types)
			const configIntegrations = (config as Record<string, unknown>)
				.integrations as Record<string, unknown>;
			const draftIntegrations = (draft as Record<string, unknown>)
				.integrations as Record<string, unknown>;
			setDirty(!isEqual(configIntegrations, draftIntegrations));
		}
	}, [config, draft]);

	useEffect(() => {
		if (success) {
			const timer = setTimeout(() => setSuccess(false), 3000);
			return () => clearTimeout(timer);
		}
	}, [success]);

	const handleSave = useCallback(async () => {
		if (!draft) return;
		setSaving(true);
		setError(null);
		try {
			await updateAndSaveConfig(draft);
			setDirty(false);
			setSuccess(true);
		} catch (err) {
			setError(
				err instanceof Error ? err.message : t("settings.general.error"),
			);
		} finally {
			setSaving(false);
		}
	}, [draft, updateAndSaveConfig, t]);

	if (loading || !draft) {
		return (
			<div className="flex items-center justify-center p-8">
				<Loader2 className="h-6 w-6 animate-spin" />
			</div>
		);
	}

	return (
		<div className="space-y-6">
			<div>
				<h2 className="text-2xl font-bold tracking-tight">
					{t("settings.integrations.title", "Integrations")}
				</h2>
				<p className="text-muted-foreground">
					{t(
						"settings.integrations.description",
						"Configure MCP integrations for coding agents",
					)}
				</p>
			</div>

			{error && (
				<Alert variant="destructive">
					<AlertDescription>{error}</AlertDescription>
				</Alert>
			)}

			{success && (
				<Alert>
					<AlertDescription>
						{t("settings.general.success", "Settings saved successfully")}
					</AlertDescription>
				</Alert>
			)}

			<Card>
				<CardHeader>
					<CardTitle>
						{t("settings.integrations.linear.title", "Linear")}
					</CardTitle>
					<CardDescription>
						{t(
							"settings.integrations.linear.description",
							"Manage Linear issues and projects directly from your coding agent",
						)}
					</CardDescription>
				</CardHeader>
				<CardContent className="space-y-4">
					<div className="flex items-center justify-between">
						<div className="space-y-0.5">
							<Label htmlFor="linear-enabled">
								{t(
									"settings.integrations.linear.enableByDefault",
									"Enable Linear MCP by default",
								)}
							</Label>
							<p className="text-sm text-muted-foreground">
								{t(
									"settings.integrations.linear.enableByDefaultDescription",
									"New tasks will have Linear MCP enabled by default",
								)}
							</p>
						</div>
						<Switch
							id="linear-enabled"
							checked={draft.integrations?.linear_mcp_enabled ?? false}
							onCheckedChange={(checked) =>
								setDraft((prev) =>
									prev
										? {
												...prev,
												integrations: {
													...prev.integrations,
													linear_mcp_enabled: checked,
												},
											}
										: prev,
								)
							}
						/>
					</div>
					<div className="space-y-2">
						<Label htmlFor="linear-api-key">
							{t("settings.integrations.linear.apiKey", "API Key")}
						</Label>
						<div className="flex gap-2">
							<Input
								id="linear-api-key"
								type={showLinearKey ? "text" : "password"}
								placeholder="lin_api_..."
								value={
									((
										(draft as Record<string, unknown>).integrations as Record<
											string,
											unknown
										>
									)?.linear_api_key as string) ?? ""
								}
								onChange={(e) =>
									setDraft((prev) =>
										prev
											? {
													...prev,
													integrations: {
														...prev.integrations,
														linear_api_key: e.target.value || undefined,
													},
												}
											: prev,
									)
								}
							/>
							<Button
								variant="outline"
								size="icon"
								onClick={() => setShowLinearKey(!showLinearKey)}
								type="button"
							>
								{showLinearKey ? (
									<EyeOff className="h-4 w-4" />
								) : (
									<Eye className="h-4 w-4" />
								)}
							</Button>
						</div>
						<p className="text-xs text-muted-foreground">
							{t(
								"settings.integrations.linear.apiKeyHelp",
								"Get your API key from Linear Settings > API",
							)}
						</p>
					</div>
					<div className="text-sm text-muted-foreground">
						<a
							href="https://linear.app/settings/api"
							target="_blank"
							rel="noopener noreferrer"
							className="inline-flex items-center gap-1 text-primary hover:underline"
						>
							{t(
								"settings.integrations.linear.getApiKey",
								"Get Linear API key",
							)}
							<ExternalLink className="h-3 w-3" />
						</a>
					</div>
				</CardContent>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>
						{t("settings.integrations.sentry.title", "Sentry")}
					</CardTitle>
					<CardDescription>
						{t(
							"settings.integrations.sentry.description",
							"Debug errors and get fix recommendations from Sentry",
						)}
					</CardDescription>
				</CardHeader>
				<CardContent className="space-y-4">
					<div className="flex items-center justify-between">
						<div className="space-y-0.5">
							<Label htmlFor="sentry-enabled">
								{t(
									"settings.integrations.sentry.enableByDefault",
									"Enable Sentry MCP by default",
								)}
							</Label>
							<p className="text-sm text-muted-foreground">
								{t(
									"settings.integrations.sentry.enableByDefaultDescription",
									"New tasks will have Sentry MCP enabled by default",
								)}
							</p>
						</div>
						<Switch
							id="sentry-enabled"
							checked={draft.integrations?.sentry_mcp_enabled ?? false}
							onCheckedChange={(checked) =>
								setDraft((prev) =>
									prev
										? {
												...prev,
												integrations: {
													...prev.integrations,
													sentry_mcp_enabled: checked,
												},
											}
										: prev,
								)
							}
						/>
					</div>
					<div className="space-y-2">
						<Label htmlFor="sentry-token">
							{t("settings.integrations.sentry.authToken", "Auth Token")}
						</Label>
						<div className="flex gap-2">
							<Input
								id="sentry-token"
								type={showSentryToken ? "text" : "password"}
								placeholder="sntrys_..."
								value={
									((
										(draft as Record<string, unknown>).integrations as Record<
											string,
											unknown
										>
									)?.sentry_auth_token as string) ?? ""
								}
								onChange={(e) =>
									setDraft((prev) =>
										prev
											? {
													...prev,
													integrations: {
														...prev.integrations,
														sentry_auth_token: e.target.value || undefined,
													},
												}
											: prev,
									)
								}
							/>
							<Button
								variant="outline"
								size="icon"
								onClick={() => setShowSentryToken(!showSentryToken)}
								type="button"
							>
								{showSentryToken ? (
									<EyeOff className="h-4 w-4" />
								) : (
									<Eye className="h-4 w-4" />
								)}
							</Button>
						</div>
						<p className="text-xs text-muted-foreground">
							{t(
								"settings.integrations.sentry.authTokenHelp",
								"Get your auth token from Sentry Settings > Auth Tokens",
							)}
						</p>
					</div>
					<div className="text-sm text-muted-foreground">
						<a
							href="https://sentry.io/settings/account/api/auth-tokens/"
							target="_blank"
							rel="noopener noreferrer"
							className="inline-flex items-center gap-1 text-primary hover:underline"
						>
							{t(
								"settings.integrations.sentry.getAuthToken",
								"Get Sentry auth token",
							)}
							<ExternalLink className="h-3 w-3" />
						</a>
					</div>
				</CardContent>
			</Card>

			<Card className="bg-muted/50">
				<CardContent className="pt-6">
					<p className="text-sm text-muted-foreground">
						{t(
							"settings.integrations.apiKeyNote",
							"API keys are stored locally and used to authenticate with Linear and Sentry MCP servers.",
						)}
					</p>
				</CardContent>
			</Card>

			<div className="flex justify-end">
				<Button onClick={handleSave} disabled={!dirty || saving}>
					{saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
					{t("common.save", "Save")}
				</Button>
			</div>
		</div>
	);
}
