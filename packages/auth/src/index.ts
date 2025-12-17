import { tanstackStartCookies } from 'better-auth/tanstack-start';
import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { db } from "@fwd.rs/db";
import * as schema from "@fwd.rs/db/schema/auth";

export const auth = betterAuth({
	database: drizzleAdapter(db, {
		provider: "pg",

		schema: schema,
	}),
	trustedOrigins: [process.env.CORS_ORIGIN || ""],
	emailAndPassword: {
		enabled: true,
	},
  plugins: [tanstackStartCookies()]
});
