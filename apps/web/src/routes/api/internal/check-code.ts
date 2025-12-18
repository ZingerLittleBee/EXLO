import { db } from "@fwd.rs/db";
import { activationCodes } from "@fwd.rs/db/schema";
import { createAPIFileRoute, json } from "@tanstack/react-start/api";
import { eq } from "drizzle-orm";

// Internal API secret for Rust server communication
const INTERNAL_SECRET = process.env.INTERNAL_API_SECRET || "dev-secret";

export const APIRoute = createAPIFileRoute("/api/internal/check-code")({
	GET: async ({ request }) => {
		// Validate internal secret
		const secret = request.headers.get("X-Internal-Secret");
		if (secret !== INTERNAL_SECRET) {
			return json({ error: "Unauthorized" }, { status: 401 });
		}

		const url = new URL(request.url);
		const code = url.searchParams.get("code");

		if (!code) {
			return json({ error: "Missing code parameter" }, { status: 400 });
		}

		try {
			const result = await db.query.activationCodes.findFirst({
				where: eq(activationCodes.code, code),
			});

			if (!result) {
				return json({ status: "not_found" }, { status: 404 });
			}

			// Check if expired
			if (new Date() > result.expiresAt) {
				return json({ status: "expired" });
			}

			return json({
				status: result.status,
				userId: result.userId,
				sessionId: result.sessionId,
			});
		} catch (error) {
			console.error("Failed to check code:", error);
			return json({ error: "Failed to check code" }, { status: 500 });
		}
	},
});
