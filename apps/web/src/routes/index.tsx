import { createFileRoute, Link, redirect } from '@tanstack/react-router'
import { Activity, Cable, Copy, Server, Users } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from 'recharts'
import { toast } from 'sonner'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { type ChartConfig, ChartContainer, ChartTooltip, ChartTooltipContent } from '@/components/ui/chart'
import { EmptyState } from '@/components/ui/empty-state'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { type DashboardMetrics, getDashboardMetrics, getRecentTunnels } from '@/functions/dashboard'
import { getUser } from '@/functions/get-user'
import { type ActiveTunnel, getPublicConfig, kickTunnel } from '@/functions/tunnels'

export const Route = createFileRoute('/')({
  component: HomeLayout,
  beforeLoad: async () => {
    const session = await getUser()
    return { session }
  },
  loader: async ({ context }) => {
    if (!context.session) {
      throw redirect({
        to: '/login',
        search: {}
      })
    }
    const [metrics, recentTunnels, config] = await Promise.all([
      getDashboardMetrics(),
      getRecentTunnels(),
      getPublicConfig()
    ])
    return {
      initialMetrics: metrics,
      initialTunnels: recentTunnels.tunnels,
      proxyUrl: config.proxyUrl,
      sshHost: config.sshHost,
      sshPort: config.sshPort
    }
  }
})

const chartConfig = {
  tunnels: {
    label: 'Tunnels',
    color: 'var(--color-emerald-500)'
  }
} satisfies ChartConfig

type TunnelHistoryPoint = {
  time: string
  tunnels: number
}

function formatDuration(connectedAt: string): string {
  const connected = new Date(connectedAt)
  const now = new Date()
  const diffMs = now.getTime() - connected.getTime()
  const seconds = Math.floor(diffMs / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)

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

function HomeLayout() {
  return (
    <div className="flex h-full">
      {/* Sidebar */}
      <aside className="w-64 border-r bg-card p-4">
        <nav className="space-y-2">
          <Link
            activeOptions={{ exact: true }}
            className="block rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent"
            to="/"
          >
            Overview
          </Link>
          <Link className="block rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent" to="/tunnels">
            Active Tunnels
          </Link>
          <Link className="block rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent" to="/users">
            User Management
          </Link>
        </nav>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-auto p-8">
        <DashboardOverview />
      </main>
    </div>
  )
}

function DashboardOverview() {
  const { session } = Route.useRouteContext()
  const { initialMetrics, initialTunnels, proxyUrl, sshHost, sshPort } = Route.useLoaderData()

  const [metrics, setMetrics] = useState<DashboardMetrics>(initialMetrics)
  const [tunnels, setTunnels] = useState<ActiveTunnel[]>(initialTunnels)
  const [tunnelHistory, setTunnelHistory] = useState<TunnelHistoryPoint[]>([])
  const [kickingSubdomain, setKickingSubdomain] = useState<string | null>(null)
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
    setTunnelHistory([
      { time: new Date().toLocaleTimeString('en-US', { hour12: false }), tunnels: initialMetrics.activeTunnels }
    ])
  }, [initialMetrics.activeTunnels])

  const fetchData = useCallback(async () => {
    try {
      const [newMetrics, recentTunnels] = await Promise.all([getDashboardMetrics(), getRecentTunnels()])
      setMetrics(newMetrics)
      setTunnels(recentTunnels.tunnels)
      setTunnelHistory((prev) => {
        const newPoint = {
          time: new Date().toLocaleTimeString('en-US', { hour12: false }),
          tunnels: newMetrics.activeTunnels
        }
        const updated = [...prev, newPoint].slice(-20)
        return updated
      })
    } catch (error) {
      console.error('Error fetching dashboard data:', error)
    }
  }, [])

  useEffect(() => {
    const interval = setInterval(fetchData, 5000)
    return () => clearInterval(interval)
  }, [fetchData])

  const handleKick = async (subdomain: string) => {
    setKickingSubdomain(subdomain)
    try {
      await kickTunnel({ data: { subdomain } })
      toast.success(`Tunnel "${subdomain}" disconnected`)
      setTunnels((prev) => prev.filter((t) => t.subdomain !== subdomain))
      await fetchData()
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to kick tunnel')
    } finally {
      setKickingSubdomain(null)
    }
  }

  const copyCommand = () => {
    const cmd = `ssh -R 8000:localhost:8000 -p ${sshPort} ${sshHost}`
    navigator.clipboard.writeText(cmd)
    toast.success('Command copied to clipboard')
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="font-bold text-3xl">Dashboard</h1>
        <p className="text-muted-foreground">Welcome back, {session?.user.name}</p>
      </div>

      {/* Metrics Cards */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="font-medium text-sm">Active Tunnels</CardTitle>
            <Cable className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="font-bold text-3xl">{metrics.activeTunnels}</div>
            <p className="text-muted-foreground text-xs">Live SSH connections</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="font-medium text-sm">Online Users</CardTitle>
            <Activity className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="font-bold text-3xl">{metrics.onlineUsers}</div>
            <p className="text-muted-foreground text-xs">Users with active tunnels</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="font-medium text-sm">Total Users</CardTitle>
            <Users className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="font-bold text-3xl">{metrics.totalUsers}</div>
            <p className="text-muted-foreground text-xs">Registered accounts</p>
          </CardContent>
        </Card>
      </div>

      {/* Chart and Quick Connect */}
      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Tunnel Activity</CardTitle>
            <CardDescription>Active tunnels over time</CardDescription>
          </CardHeader>
          <CardContent>
            {tunnelHistory.length > 1 ? (
              <ChartContainer className="h-[200px] w-full" config={chartConfig}>
                <AreaChart data={tunnelHistory} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                  <defs>
                    <linearGradient id="fillTunnels" x1="0" x2="0" y1="0" y2="1">
                      <stop offset="5%" stopColor="var(--color-emerald-500)" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="var(--color-emerald-500)" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid className="stroke-muted" strokeDasharray="3 3" />
                  <XAxis axisLine={false} dataKey="time" tick={{ fontSize: 10 }} tickLine={false} />
                  <YAxis allowDecimals={false} axisLine={false} tick={{ fontSize: 10 }} tickLine={false} />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Area
                    dataKey="tunnels"
                    fill="url(#fillTunnels)"
                    stroke="var(--color-emerald-500)"
                    strokeWidth={2}
                    type="monotone"
                  />
                </AreaChart>
              </ChartContainer>
            ) : (
              <div className="flex h-[200px] items-center justify-center text-muted-foreground text-sm">
                Collecting data...
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Quick Connect</CardTitle>
            <CardDescription>Copy the SSH command to create a tunnel</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center gap-2">
              <code className="flex-1 rounded-md bg-muted px-3 py-2 font-mono text-sm">
                ssh -R 8000:localhost:8000 -p {sshPort} {sshHost}
              </code>
              <Button onClick={copyCommand} size="icon" variant="outline">
                <Copy className="h-4 w-4" />
              </Button>
            </div>
            <div className="space-y-2 text-muted-foreground text-sm">
              <div className="flex items-center gap-2">
                <Server className="h-4 w-4" />
                <span>Replace PORT with your local service port</span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Active Connections Table */}
      <Card>
        <CardHeader>
          <CardTitle>Active Connections</CardTitle>
          <CardDescription>Recent SSH tunnels connected to your server</CardDescription>
          <CardAction>
            <Button asChild size="sm" variant="outline">
              <Link to="/tunnels">View All</Link>
            </Button>
          </CardAction>
        </CardHeader>
        <CardContent>
          {tunnels.length === 0 ? (
            <EmptyState icon={Cable} title="No active tunnels" />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Status</TableHead>
                  <TableHead>Subdomain</TableHead>
                  <TableHead>User</TableHead>
                  <TableHead>Origin IP</TableHead>
                  <TableHead>Duration</TableHead>
                  <TableHead className="text-right">Action</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {tunnels.map((tunnel) => (
                  <TableRow key={tunnel.subdomain}>
                    <TableCell>
                      {tunnel.is_connected ? (
                        <Badge className="bg-emerald-500/10 text-emerald-500" variant="outline">
                          <span className="mr-1 inline-block h-2 w-2 rounded-full bg-emerald-500" />
                          Active
                        </Badge>
                      ) : (
                        <Badge className="bg-amber-500/10 text-amber-500" variant="outline">
                          <span className="mr-1 inline-block h-2 w-2 rounded-full bg-amber-500" />
                          Reconnectable
                        </Badge>
                      )}
                    </TableCell>
                    <TableCell>
                      <a
                        className="font-mono text-primary text-sm hover:underline"
                        href={getTunnelUrl(tunnel.subdomain, proxyUrl)}
                        rel="noopener noreferrer"
                        target="_blank"
                      >
                        {tunnel.subdomain}
                      </a>
                    </TableCell>
                    <TableCell>
                      {tunnel.user_id ? (
                        <span className="font-mono text-xs">{tunnel.user_id.slice(0, 8)}...</span>
                      ) : (
                        <span className="text-muted-foreground">-</span>
                      )}
                    </TableCell>
                    <TableCell>
                      <Badge className="font-mono text-xs" variant="secondary">
                        {tunnel.client_ip}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <span className="tabular-nums">{mounted ? formatDuration(tunnel.connected_at) : '-'}</span>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        disabled={kickingSubdomain === tunnel.subdomain}
                        onClick={() => handleKick(tunnel.subdomain)}
                        size="sm"
                        variant="destructive"
                      >
                        {kickingSubdomain === tunnel.subdomain ? 'Killing...' : 'Kill'}
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
