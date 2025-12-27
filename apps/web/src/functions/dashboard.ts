import { count, db } from '@exlo/db'
import { user } from '@exlo/db/schema/auth'
import { createServerFn } from '@tanstack/react-start'
import { env } from '@/lib/env'
import { authMiddleware } from '@/middleware/auth'

export type DashboardMetrics = {
  activeTunnels: number
  onlineUsers: number
  totalUsers: number
}

export type TunnelHistoryPoint = {
  time: string
  tunnels: number
}

export const getDashboardMetrics = createServerFn({ method: 'GET' })
  .middleware([authMiddleware])
  .handler(async (): Promise<DashboardMetrics> => {
    try {
      const tunnelResponse = await fetch(`${env.TUNNL_MANAGEMENT_API_URL}/tunnels`, {
        method: 'GET',
        headers: { 'Content-Type': 'application/json' }
      })

      let activeTunnels = 0
      const uniqueUserIds = new Set<string>()

      if (tunnelResponse.ok) {
        const data = await tunnelResponse.json()
        activeTunnels = data.tunnels?.length ?? 0
        for (const tunnel of data.tunnels ?? []) {
          if (tunnel.user_id) {
            uniqueUserIds.add(tunnel.user_id)
          }
        }
      }

      const userCountResult = await db.select({ count: count() }).from(user)
      const totalUsers = userCountResult[0]?.count ?? 0

      return {
        activeTunnels,
        onlineUsers: uniqueUserIds.size,
        totalUsers
      }
    } catch (error) {
      console.error('Error fetching dashboard metrics:', error)
      return {
        activeTunnels: 0,
        onlineUsers: 0,
        totalUsers: 0
      }
    }
  })

export const getRecentTunnels = createServerFn({ method: 'GET' })
  .middleware([authMiddleware])
  .handler(async () => {
    try {
      const response = await fetch(`${env.TUNNL_MANAGEMENT_API_URL}/tunnels`, {
        method: 'GET',
        headers: { 'Content-Type': 'application/json' }
      })

      if (!response.ok) {
        return { tunnels: [] }
      }

      const data = await response.json()
      return { tunnels: data.tunnels?.slice(0, 5) ?? [] }
    } catch (error) {
      console.error('Error fetching recent tunnels:', error)
      return { tunnels: [] }
    }
  })
