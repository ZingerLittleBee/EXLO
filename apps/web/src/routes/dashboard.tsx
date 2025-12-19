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
            to="/dashboard"
            className="block rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent"
            activeOptions={{ exact: true }}
          >
            Overview
          </Link>
          <Link
            to="/dashboard/users"
            className="block rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent"
          >
            User Management
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
