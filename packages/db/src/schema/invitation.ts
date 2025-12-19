import { relations } from 'drizzle-orm'
import { index, pgTable, text, timestamp, uuid } from 'drizzle-orm/pg-core'
import { user } from './auth'

// Stores invitation tokens for invite-only user registration
export const invitations = pgTable(
  'invitations',
  {
    id: uuid('id').primaryKey().defaultRandom(),
    // Email address the invitation was sent to
    email: text('email').notNull(),
    // Unique token for invitation link
    token: text('token').notNull().unique(),
    // When this invitation expires
    expiresAt: timestamp('expires_at').notNull(),
    // Admin who created this invitation
    createdBy: text('created_by')
      .notNull()
      .references(() => user.id, { onDelete: 'cascade' }),
    // When created
    createdAt: timestamp('created_at').defaultNow().notNull()
  },
  (table) => [index('invitations_token_idx').on(table.token), index('invitations_email_idx').on(table.email)]
)

// Relations
export const invitationRelations = relations(invitations, ({ one }) => ({
  createdByUser: one(user, {
    fields: [invitations.createdBy],
    references: [user.id]
  })
}))
