import { db, eq } from '@exlo/db'
import { activationCodes } from '@exlo/db/schema/index'
import { createServerFn } from '@tanstack/react-start'
import { authMiddleware } from '@/middleware/auth'

// Server function to authorize a device code
export const authorizeCode = createServerFn({ method: 'POST' })
  .middleware([authMiddleware])
  .inputValidator((data: { code: string }) => data)
  .handler(async ({ context, data }) => {
    const { code } = data

    if (!context.session?.user) {
      throw new Error('Not authenticated')
    }

    // Find the code
    const existing = await db.query.activationCodes.findFirst({
      where: eq(activationCodes.code, code)
    })

    if (!existing) {
      throw new Error('Code not found')
    }

    if (existing.status !== 'pending') {
      throw new Error('Code already used or expired')
    }

    if (new Date() > existing.expiresAt) {
      // Update to expired
      await db.update(activationCodes).set({ status: 'expired' }).where(eq(activationCodes.code, code))
      throw new Error('Code has expired')
    }

    // Authorize the code
    await db
      .update(activationCodes)
      .set({
        status: 'verified',
        userId: context.session.user.id
      })
      .where(eq(activationCodes.code, code))

    return { success: true }
  })

// Get code status for display
export const getCodeStatus = createServerFn({ method: 'GET' })
  .inputValidator((data: { code: string }) => data)
  .handler(async ({ data }) => {
    const { code } = data

    if (!code) {
      return { error: 'No code provided' }
    }

    const existing = await db.query.activationCodes.findFirst({
      where: eq(activationCodes.code, code)
    })

    if (!existing) {
      return { status: 'not_found' }
    }

    if (new Date() > existing.expiresAt) {
      return { status: 'expired' }
    }

    return { status: existing.status }
  })
