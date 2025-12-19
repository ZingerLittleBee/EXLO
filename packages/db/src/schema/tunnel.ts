import { relations } from 'drizzle-orm'
import { index, integer, pgTable, text, timestamp } from 'drizzle-orm/pg-core'
import { user } from './auth'

// Stores active tunnel information from the Rust SSH server
export const tunnels = pgTable(
  'tunnels',
  {
    // Unique subdomain assigned to this tunnel (e.g., "tunnel-abc123-1")
    subdomain: text('subdomain').primaryKey(),
    // User who owns this tunnel
    userId: text('user_id')
      .notNull()
      .references(() => user.id, { onDelete: 'cascade' }),
    // SSH session ID from Rust server
    sessionId: text('session_id').notNull(),
    // Client's requested forward address (usually "localhost")
    requestedAddress: text('requested_address').notNull(),
    // Client's local port being forwarded (e.g., 8000)
    requestedPort: integer('requested_port').notNull(),
    // Server's virtual port (usually 80)
    serverPort: integer('server_port').notNull(),
    // Client's IP address
    clientIp: text('client_ip').notNull(),
    // When this tunnel was created
    createdAt: timestamp('created_at').defaultNow().notNull()
  },
  (table) => [index('tunnels_user_idx').on(table.userId), index('tunnels_session_idx').on(table.sessionId)]
)

// Relations
export const tunnelRelations = relations(tunnels, ({ one }) => ({
  user: one(user, {
    fields: [tunnels.userId],
    references: [user.id]
  })
}))
