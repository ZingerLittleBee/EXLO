import { db } from '@fwd.rs/db'
import { activationCodes } from '@fwd.rs/db/schema/index'
import { createFileRoute } from '@tanstack/react-router'

// Internal API secret for Rust server communication
const INTERNAL_SECRET = process.env.INTERNAL_API_SECRET || 'dev-secret'

export const Route = createFileRoute('/api/internal/generate-code')({
  server: {
    handlers: {
      GET: async () =>
        new Response(
          JSON.stringify({
            endpoint: '/api/internal/generate-code',
            method: 'POST required',
            headers: { 'X-Internal-Secret': 'required' }
          }),
          {
            headers: { 'Content-Type': 'application/json' }
          }
        ),
      POST: async ({ request }) => {
        const secret = request.headers.get('X-Internal-Secret')
        if (secret !== INTERNAL_SECRET) {
          return new Response(JSON.stringify({ error: 'Unauthorized' }), {
            status: 401,
            headers: { 'Content-Type': 'application/json' }
          })
        }

        try {
          const body = await request.json()
          const { code, sessionId, expiresAt } = body as {
            code: string
            sessionId: string
            expiresAt: string
          }

          if (!(code && sessionId && expiresAt)) {
            return new Response(JSON.stringify({ error: 'Missing required fields' }), {
              status: 400,
              headers: { 'Content-Type': 'application/json' }
            })
          }

          await db.insert(activationCodes).values({
            code,
            sessionId,
            status: 'pending',
            expiresAt: new Date(expiresAt)
          })

          return new Response(JSON.stringify({ success: true, code }), {
            headers: { 'Content-Type': 'application/json' }
          })
        } catch (error) {
          console.error('Failed to generate code:', error)
          return new Response(
            JSON.stringify({
              error: 'Failed to generate code',
              details: error instanceof Error ? error.message : String(error)
            }),
            {
              status: 500,
              headers: { 'Content-Type': 'application/json' }
            }
          )
        }
      }
    }
  }
})
