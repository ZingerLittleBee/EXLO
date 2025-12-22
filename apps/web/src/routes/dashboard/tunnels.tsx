import { createFileRoute, useRouter } from '@tanstack/react-router'
import { useCallback, useEffect, useState } from 'react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { type ActiveTunnel, getTunnels, kickTunnel } from '@/functions/tunnels'

export const Route = createFileRoute('/dashboard/tunnels')({
  component: TunnelsDashboard,
  loader: async () => {
    // Auth is handled by parent /dashboard route
    const data = await getTunnels()
    const proxyUrl = process.env.PROXY_URL || 'http://localhost:8080'
    return { initialTunnels: data.tunnels, proxyUrl }
  }
})

function formatDuration(connectedAt: string): string {
  const connected = new Date(connectedAt)
  const now = new Date()
  const diffMs = now.getTime() - connected.getTime()

  const seconds = Math.floor(diffMs / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (days > 0) {
    return `${days}d ${hours % 24}h`
  }
  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`
  }
  return `${seconds}s`
}

function getTunnelUrl(subdomain: string, proxyUrl: string): string {
  try {
    const url = new URL(proxyUrl)
    return `${url.protocol}//${subdomain}.${url.host}`
  } catch {
    return `http://${subdomain}.localhost:8080`
  }
}

function TunnelsDashboard() {
  const { initialTunnels, proxyUrl } = Route.useLoaderData()
  const router = useRouter()
  const [tunnels, setTunnels] = useState<ActiveTunnel[]>(initialTunnels)
  const [isLoading, setIsLoading] = useState(false)
  const [kickingSubdomain, setKickingSubdomain] = useState<string | null>(null)
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
  }, [])

  const fetchTunnels = useCallback(async () => {
    try {
      const data = await getTunnels()
      setTunnels(data.tunnels)
    } catch (error) {
      console.error('Error fetching tunnels:', error)
    }
  }, [])

  // Poll for updates every 3 seconds
  useEffect(() => {
    const interval = setInterval(fetchTunnels, 3000)
    return () => clearInterval(interval)
  }, [fetchTunnels])

  const handleKick = async (subdomain: string) => {
    setKickingSubdomain(subdomain)
    try {
      await kickTunnel({ data: { subdomain } })
      toast.success(`Tunnel "${subdomain}" disconnected`)
      // Optimistically remove from UI
      setTunnels((prev) => prev.filter((t) => t.subdomain !== subdomain))
      // Also invalidate router to refresh data
      router.invalidate()
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to kick tunnel')
      // Refresh to get current state
      await fetchTunnels()
    } finally {
      setKickingSubdomain(null)
    }
  }

  const handleRefresh = async () => {
    setIsLoading(true)
    try {
      await fetchTunnels()
      toast.success('Tunnels refreshed')
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Active Tunnels</CardTitle>
              <CardDescription>
                Manage SSH tunnels connected to your server. Auto-refreshes every 3 seconds.
              </CardDescription>
            </div>
            <Button disabled={isLoading} onClick={handleRefresh} size="sm" variant="outline">
              {isLoading ? (
                <svg
                  className="h-4 w-4 animate-spin"
                  fill="none"
                  viewBox="0 0 24 24"
                  xmlns="http://www.w3.org/2000/svg"
                >
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                  <path
                    className="opacity-75"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    fill="currentColor"
                  />
                </svg>
              ) : (
                'Refresh'
              )}
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {tunnels.length === 0 ? (
            <div className="py-12 text-center text-muted-foreground">
              <svg
                className="mx-auto mb-4 h-12 w-12 text-muted-foreground/50"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                />
              </svg>
              <p className="font-medium text-lg">No active tunnels</p>
              <p className="mt-1 text-sm">
                Connect with:{' '}
                <code className="rounded bg-muted px-2 py-1 text-xs">
                  ssh -N -R 80:localhost:PORT -p 2222 user@server
                </code>
              </p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Subdomain</TableHead>
                  <TableHead>User ID</TableHead>
                  <TableHead>IP Address</TableHead>
                  <TableHead>Duration</TableHead>
                  <TableHead className="text-right">Action</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {tunnels.map((tunnel) => (
                  <TableRow key={tunnel.subdomain}>
                    <TableCell className="font-medium">
                      <a
                        className="text-primary hover:underline"
                        href={getTunnelUrl(tunnel.subdomain, proxyUrl)}
                        rel="noopener noreferrer"
                        target="_blank"
                      >
                        {tunnel.subdomain}
                      </a>
                    </TableCell>
                    <TableCell>
                      {tunnel.user_id ? (
                        <span className="rounded bg-muted px-2 py-1 font-mono text-xs">{tunnel.user_id}</span>
                      ) : (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </TableCell>
                    <TableCell>
                      <span className="font-mono text-xs">{tunnel.client_ip}</span>
                    </TableCell>
                    <TableCell>
                      <span className="tabular-nums">{mounted ? formatDuration(tunnel.connected_at) : '—'}</span>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        disabled={kickingSubdomain === tunnel.subdomain}
                        onClick={() => handleKick(tunnel.subdomain)}
                        size="sm"
                        variant="destructive"
                      >
                        {kickingSubdomain === tunnel.subdomain ? (
                          <svg
                            className="h-4 w-4 animate-spin"
                            fill="none"
                            viewBox="0 0 24 24"
                            xmlns="http://www.w3.org/2000/svg"
                          >
                            <circle
                              className="opacity-25"
                              cx="12"
                              cy="12"
                              r="10"
                              stroke="currentColor"
                              strokeWidth="4"
                            />
                            <path
                              className="opacity-75"
                              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                              fill="currentColor"
                            />
                          </svg>
                        ) : (
                          'Kick'
                        )}
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <div className="mt-4 text-center text-muted-foreground text-sm">
        {tunnels.length} active tunnel{tunnels.length !== 1 ? 's' : ''}
      </div>
    </div>
  )
}
