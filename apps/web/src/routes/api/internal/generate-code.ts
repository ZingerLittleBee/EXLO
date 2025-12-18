import { db } from "@fwd.rs/db";
import { activationCodes } from "@fwd.rs/db/schema";
import { createAPIFileRoute, json } from "@tanstack/react-start/api";

// Internal API secret for Rust server communication
const INTERNAL_SECRET = process.env.INTERNAL_API_SECRET || "dev-secret";

export const APIRoute = createAPIFileRoute("/api/internal/generate-code")({
	POST: async ({ request }) => {
		// Validate internal secret
		const secret = request.headers.get("X-Internal-Secret");
		if (secret !== INTERNAL_SECRET) {
			return json({ error: "Unauthorized" }, { status: 401 });
		}

		try {
			const body = await request.json();
			const { code, sessionId, expiresAt } = body as {
				code: string;
				sessionId: string;
				expiresAt: string;
			};

			if (!code || !sessionId || !expiresAt) {
				return json({ error: "Missing required fields" }, { status: 400 });
			}

			// Insert the activation code
			await db.insert(activationCodes).values({
				code,
				sessionId,
				status: "pending",
				expiresAt: new Date(expiresAt),
			});

			return json({ success: true, code });
		} catch (error) {
			console.error("Failed to generate code:", error);
			return json({ error: "Failed to generate code" }, { status: 500 });
		}
	},
});
