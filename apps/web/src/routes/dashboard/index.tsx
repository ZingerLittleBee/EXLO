import { createFileRoute } from '@tanstack/react-router'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

export const Route = createFileRoute('/dashboard/')({
  // This is the index route for /dashboard
  component: DashboardOverview
})

function DashboardOverview() {
  const { session } = Route.useRouteContext()

  return (
    <div className="space-y-8">
      <div>
        <h1 className="font-bold text-3xl">Dashboard</h1>
        <p className="text-muted-foreground">Welcome, {session?.user.name}</p>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Active Tunnels</CardTitle>
            <CardDescription>Your currently active SSH tunnels</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground text-sm">No active tunnels</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Recent Activity</CardTitle>
            <CardDescription>Your recent tunnel activity</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground text-sm">No recent activity</p>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
