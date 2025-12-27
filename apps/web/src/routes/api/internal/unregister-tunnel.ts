import { db, eq } from '@exlo/db'
import { tunnels } from '@exlo/db/schema/index'
import { createFileRoute } from '@tanstack/react-router'
import { env } from '@/lib/env'

export const Route = createFileRoute('/api/internal/unregister-tunnel')({
  server: {
    handlers: {
      GET: async () =>
        new Response(
          JSON.stringify({
            endpoint: '/api/internal/unregister-tunnel',
            method: 'POST required',
            headers: { 'X-Internal-Secret': 'required' }
          }),
          {
            headers: { 'Content-Type': 'application/json' }
          }
        ),
      POST: async ({ request }) => {
        const secret = request.headers.get('X-Internal-Secret')
        if (secret !== env.INTERNAL_API_SECRET) {
          return new Response(JSON.stringify({ error: 'Unauthorized' }), {
            status: 401,
            headers: { 'Content-Type': 'application/json' }
          })
        }

        try {
          const body = await request.json()
          const { subdomain } = body as { subdomain: string }

          if (!subdomain) {
            return new Response(JSON.stringify({ error: 'Missing subdomain' }), {
              status: 400,
              headers: { 'Content-Type': 'application/json' }
            })
          }

          const result = await db.delete(tunnels).where(eq(tunnels.subdomain, subdomain)).returning()

          return new Response(
            JSON.stringify({
              success: true,
              deleted: result.length > 0
            }),
            {
              headers: { 'Content-Type': 'application/json' }
            }
          )
        } catch (error) {
          console.error('Failed to unregister tunnel:', error)
          return new Response(
            JSON.stringify({
              error: 'Failed to unregister tunnel',
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
