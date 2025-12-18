import { relations } from "drizzle-orm";
import { index, pgEnum, pgTable, text, timestamp } from "drizzle-orm/pg-core";
import { user } from "./auth";

// Device Flow activation code status
export const deviceCodeStatus = pgEnum("device_code_status", [
	"pending",
	"verified",
	"expired",
]);

// Stores temporary codes for SSH device flow authentication
export const activationCodes = pgTable(
	"activation_codes",
	{
		// Random code shown to user (e.g. "AF3D-1234")
		code: text("code").primaryKey(),
		// SSH session ID from Rust server
		sessionId: text("session_id").notNull(),
		// Current status of the code
		status: deviceCodeStatus("status").default("pending").notNull(),
		// User who authorized the code (null until verified)
		userId: text("user_id").references(() => user.id, { onDelete: "cascade" }),
		// When this code expires
		expiresAt: timestamp("expires_at").notNull(),
		// When created
		createdAt: timestamp("created_at").defaultNow().notNull(),
	},
	(table) => [
		index("activation_codes_session_idx").on(table.sessionId),
		index("activation_codes_status_idx").on(table.status),
	],
);

// Relations
export const activationCodeRelations = relations(activationCodes, ({ one }) => ({
	user: one(user, {
		fields: [activationCodes.userId],
		references: [user.id],
	}),
}));
