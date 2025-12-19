import { authMiddleware } from '@/middleware/auth'
import { createServerFn } from '@tanstack/react-start'

// Type for active tunnel from the Rust API
export interface ActiveTunnel {
  subdomain: string
  user_id: string | null
  client_ip: string
  connected_at: string
}

interface TunnelsListResponse {
  tunnels: ActiveTunnel[]
}

const MANAGEMENT_API_URL = process.env.TUNNL_MANAGEMENT_API_URL || 'http://127.0.0.1:9090'

// Server function to fetch all active tunnels
export const getTunnels = createServerFn({ method: 'GET' })
  .middleware([authMiddleware])
  .handler(async () => {
    try {
      const response = await fetch(`${MANAGEMENT_API_URL}/tunnels`, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      })

      if (!response.ok) {
        throw new Error(`Failed to fetch tunnels: ${response.statusText}`)
      }

      const data = (await response.json()) as TunnelsListResponse
      return { tunnels: data.tunnels }
    } catch (error) {
      console.error('Error fetching tunnels:', error)
      return { tunnels: [], error: error instanceof Error ? error.message : 'Unknown error' }
    }
  })

// Server function to kick a tunnel
export const kickTunnel = createServerFn({ method: 'POST' })
  .middleware([authMiddleware])
  .inputValidator((data: { subdomain: string }) => data)
  .handler(async ({ data }) => {
    const { subdomain } = data

    if (!subdomain) {
      throw new Error('Subdomain is required')
    }

    try {
      const response = await fetch(`${MANAGEMENT_API_URL}/tunnels/${encodeURIComponent(subdomain)}`, {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
        },
      })

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}))
        throw new Error(errorData.error || `Failed to kick tunnel: ${response.statusText}`)
      }

      const result = await response.json()
      return { success: true, message: result.message }
    } catch (error) {
      console.error('Error kicking tunnel:', error)
      throw error
    }
  })
