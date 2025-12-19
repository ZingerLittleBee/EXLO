import { db } from '@fwd.rs/db'
import { tunnels } from '@fwd.rs/db/schema/index'
import { createFileRoute } from '@tanstack/react-router'

// Internal API secret for Rust server communication
const INTERNAL_SECRET = process.env.INTERNAL_API_SECRET || 'dev-secret'

export const Route = createFileRoute('/api/internal/register-tunnel')({
  server: {
    handlers: {
      GET: async () =>
        new Response(
          JSON.stringify({
            endpoint: '/api/internal/register-tunnel',
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
          const { subdomain, userId, sessionId, requestedAddress, requestedPort, serverPort, clientIp } = body as {
            subdomain: string
            userId: string
            sessionId: string
            requestedAddress: string
            requestedPort: number
            serverPort: number
            clientIp: string
          }

          if (
            !(
              subdomain &&
              userId &&
              sessionId &&
              requestedAddress &&
              requestedPort !== undefined &&
              serverPort !== undefined &&
              clientIp
            )
          ) {
            return new Response(JSON.stringify({ error: 'Missing required fields' }), {
              status: 400,
              headers: { 'Content-Type': 'application/json' }
            })
          }

          await db
            .insert(tunnels)
            .values({
              subdomain,
              userId,
              sessionId,
              requestedAddress,
              requestedPort,
              serverPort,
              clientIp
            })
            .onConflictDoUpdate({
              target: tunnels.subdomain,
              set: {
                userId,
                sessionId,
                requestedAddress,
                requestedPort,
                serverPort,
                clientIp,
                createdAt: new Date()
              }
            })

          return new Response(JSON.stringify({ success: true, subdomain }), {
            headers: { 'Content-Type': 'application/json' }
          })
        } catch (error) {
          console.error('Failed to register tunnel:', error)
          return new Response(
            JSON.stringify({
              error: 'Failed to register tunnel',
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
