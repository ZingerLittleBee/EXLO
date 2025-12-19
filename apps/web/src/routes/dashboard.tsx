import { createFileRoute, Link, Outlet, redirect } from '@tanstack/react-router'
import { getUser } from '@/functions/get-user'

export const Route = createFileRoute('/dashboard')({
  component: DashboardLayout,
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
  }
})

function DashboardLayout() {
  return (
    <div className="flex h-full">
      {/* Sidebar */}
      <aside className="w-64 border-r bg-card p-4">
        <nav className="space-y-2">
          <Link
            activeOptions={{ exact: true }}
            className="block rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent"
            to="/dashboard"
          >
            Overview
          </Link>
          <Link
            className="block rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent"
            to="/dashboard/users"
          >
            User Management
          </Link>
          <Link
            className="block rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent"
            to="/dashboard/tunnels"
          >
            Active Tunnels
          </Link>
        </nav>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-auto p-8">
        <Outlet />
      </main>
    </div>
  )
}
