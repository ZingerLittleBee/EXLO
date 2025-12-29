import { db, eq } from '@exlo/db'
import { activationCodes } from '@exlo/db/schema'
import { createFileRoute } from '@tanstack/react-router'
import { env } from '@/lib/env'

export const Route = createFileRoute('/api/internal/check-code')({
  server: {
    handlers: {
      GET: async ({ request }) => {
        const secret = request.headers.get('X-Internal-Secret')
        if (secret !== env.INTERNAL_API_SECRET) {
          return new Response(JSON.stringify({ error: 'Unauthorized' }), {
            status: 401,
            headers: { 'Content-Type': 'application/json' }
          })
        }

        const url = new URL(request.url)
        const code = url.searchParams.get('code')

        if (!code) {
          return new Response(JSON.stringify({ error: 'Missing code parameter' }), {
            status: 400,
            headers: { 'Content-Type': 'application/json' }
          })
        }

        try {
          const result = await db.query.activationCodes.findFirst({
            where: eq(activationCodes.code, code),
            with: {
              user: true
            }
          })

          if (!result) {
            return new Response(JSON.stringify({ status: 'not_found' }), {
              status: 404,
              headers: { 'Content-Type': 'application/json' }
            })
          }

          if (new Date() > result.expiresAt) {
            return new Response(JSON.stringify({ status: 'expired' }), {
              headers: { 'Content-Type': 'application/json' }
            })
          }

          return new Response(
            JSON.stringify({
              status: result.status,
              userId: result.userId,
              userName: result.user?.name,
              sessionId: result.sessionId
            }),
            {
              headers: { 'Content-Type': 'application/json' }
            }
          )
        } catch (error) {
          console.error('Failed to check code:', error)
          return new Response(JSON.stringify({ error: 'Failed to check code' }), {
            status: 500,
            headers: { 'Content-Type': 'application/json' }
          })
        }
      }
    }
  }
})
