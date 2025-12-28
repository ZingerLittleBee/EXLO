import { db } from '@exlo/db'
import * as schema from '@exlo/db/schema/auth'
import { betterAuth } from 'better-auth'
import { drizzleAdapter } from 'better-auth/adapters/drizzle'
import { admin } from 'better-auth/plugins'
import { tanstackStartCookies } from 'better-auth/tanstack-start'

export const auth = betterAuth({
  secret: process.env.AUTH_SECRET,
  baseURL: process.env.HOMEPAGE_URL,
  database: drizzleAdapter(db, {
    provider: 'pg',
    schema
  }),
  trustedOrigins: [process.env.HOMEPAGE_URL || ''],
  emailAndPassword: {
    enabled: true
  },
  // Disable public sign-up endpoint - users can only be created via:
  // 1. First-run setup (/setup page)
  // 2. Admin invitations (/join page)
  disabledPaths: ['/sign-up/email'],
  plugins: [
    tanstackStartCookies(),
    // Admin plugin for user management capabilities
    admin()
  ]
})
