import { randomBytes } from 'node:crypto'
import { auth } from '@fwd.rs/auth'
import { db, eq } from '@fwd.rs/db'
import { invitations } from '@fwd.rs/db/schema/invitation'
import { createServerFn } from '@tanstack/react-start'
import { authMiddleware } from '@/middleware/auth'

// Generate a secure random token
function generateToken(): string {
  return randomBytes(32).toString('hex')
}

// Create an invitation (admin only)
export const createInvitation = createServerFn({ method: 'POST' })
  .middleware([authMiddleware])
  .inputValidator((data: { email: string; expiresInDays?: number }) => data)
  .handler(async ({ context, data }) => {
    const { email, expiresInDays = 7 } = data

    if (!context.session?.user) {
      throw new Error('Not authenticated')
    }

    // Check if there's already a pending invitation for this email
    const existing = await db.query.invitations.findFirst({
      where: eq(invitations.email, email.toLowerCase())
    })

    if (existing && new Date() < existing.expiresAt) {
      throw new Error('An active invitation already exists for this email')
    }

    // Delete expired invitations for this email
    if (existing) {
      await db.delete(invitations).where(eq(invitations.id, existing.id))
    }

    const token = generateToken()
    const expiresAt = new Date()
    expiresAt.setDate(expiresAt.getDate() + expiresInDays)

    await db.insert(invitations).values({
      email: email.toLowerCase(),
      token,
      expiresAt,
      createdBy: context.session.user.id
    })

    return {
      success: true,
      token,
      expiresAt: expiresAt.toISOString()
    }
  })

// Validate an invitation token
export const validateInvitation = createServerFn({ method: 'GET' })
  .inputValidator((data: { token: string }) => data)
  .handler(async ({ data }) => {
    const { token } = data

    if (!token) {
      return { valid: false, error: 'No token provided' }
    }

    const invitation = await db.query.invitations.findFirst({
      where: eq(invitations.token, token)
    })

    if (!invitation) {
      return { valid: false, error: 'Invalid invitation token' }
    }

    if (new Date() > invitation.expiresAt) {
      return { valid: false, error: 'Invitation has expired' }
    }

    return {
      valid: true,
      email: invitation.email
    }
  })

// Accept an invitation and create user account
export const acceptInvitation = createServerFn({ method: 'POST' })
  .inputValidator((data: { token: string; name: string; password: string }) => data)
  .handler(async ({ data }) => {
    const { token, name, password } = data

    // Validate the invitation
    const invitation = await db.query.invitations.findFirst({
      where: eq(invitations.token, token)
    })

    if (!invitation) {
      throw new Error('Invalid invitation token')
    }

    if (new Date() > invitation.expiresAt) {
      throw new Error('Invitation has expired')
    }

    // Create the user account using Better Auth API
    const response = await auth.api.signUpEmail({
      body: {
        email: invitation.email,
        password,
        name
      }
    })

    if (!response) {
      throw new Error('Failed to create user account')
    }

    // Delete the used invitation
    await db.delete(invitations).where(eq(invitations.id, invitation.id))

    return { success: true, email: invitation.email }
  })

// List all invitations (admin only)
export const listInvitations = createServerFn({ method: 'GET' })
  .middleware([authMiddleware])
  .handler(async ({ context }) => {
    if (!context.session?.user) {
      throw new Error('Not authenticated')
    }

    const allInvitations = await db.query.invitations.findMany({
      orderBy: (invitations, { desc }) => [desc(invitations.createdAt)]
    })

    return allInvitations.map((inv) => ({
      id: inv.id,
      email: inv.email,
      expiresAt: inv.expiresAt.toISOString(),
      createdAt: inv.createdAt.toISOString(),
      isExpired: new Date() > inv.expiresAt
    }))
  })

// Delete an invitation (admin only)
export const deleteInvitation = createServerFn({ method: 'POST' })
  .middleware([authMiddleware])
  .inputValidator((data: { id: string }) => data)
  .handler(async ({ context, data }) => {
    const { id } = data

    if (!context.session?.user) {
      throw new Error('Not authenticated')
    }

    await db.delete(invitations).where(eq(invitations.id, id))

    return { success: true }
  })
