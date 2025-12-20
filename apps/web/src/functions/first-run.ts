import { auth } from '@exlo/auth'
import { count, db } from '@exlo/db'
import { user } from '@exlo/db/schema/auth'
import { createServerFn } from '@tanstack/react-start'

// Check if this is a first-run (no users exist in the database)
export const isFirstRun = createServerFn({ method: 'GET' }).handler(async () => {
  const result = await db.select({ count: count() }).from(user)
  const userCount = result[0]?.count ?? 0
  return { isFirstRun: userCount === 0 }
})

// Create the initial admin user (only works when no users exist)
export const createInitialAdmin = createServerFn({ method: 'POST' })
  .inputValidator((data: { email: string; password: string; name: string }) => data)
  .handler(async ({ data }) => {
    const { email, password, name } = data

    // Double-check that no users exist (security measure)
    const result = await db.select({ count: count() }).from(user)
    const userCount = result[0]?.count ?? 0

    if (userCount > 0) {
      throw new Error('Setup has already been completed')
    }

    // Create the first user using Better Auth's internal API
    // This bypasses the disabled sign-up endpoint
    const response = await auth.api.signUpEmail({
      body: {
        email,
        password,
        name
      }
    })

    if (!response) {
      throw new Error('Failed to create admin user')
    }

    return { success: true }
  })
