import { createFileRoute, Link, redirect, useRouter } from '@tanstack/react-router'
import { useState } from 'react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { getUser } from '@/functions/get-user'
import { createInvitation, deleteInvitation, listInvitations } from '@/functions/invitation'

export const Route = createFileRoute('/users')({
  component: UsersLayout,
  beforeLoad: async () => {
    const session = await getUser()
    return { session }
  },
  loader: async ({ context }) => {
    if (!context.session) {
      throw redirect({ to: '/login', search: {} })
    }
    const invitations = await listInvitations()
    return { invitations }
  }
})

function UsersLayout() {
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
        <UsersPage />
      </main>
    </div>
  )
}

function UsersPage() {
  const loaderData = Route.useLoaderData()
  const router = useRouter()
  const [isDialogOpen, setIsDialogOpen] = useState(false)
  const [email, setEmail] = useState('')
  const [isCreating, setIsCreating] = useState(false)
  const [generatedLink, setGeneratedLink] = useState<string | null>(null)

  const handleCreateInvitation = async () => {
    if (!email) {
      toast.error('Please enter an email address')
      return
    }

    setIsCreating(true)
    try {
      const result = await createInvitation({ data: { email } })
      const inviteUrl = `${window.location.origin}/join?token=${result.token}`
      setGeneratedLink(inviteUrl)
      toast.success('Invitation created successfully!')
      router.invalidate()
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to create invitation')
    } finally {
      setIsCreating(false)
    }
  }

  const handleDeleteInvitation = async (id: string) => {
    try {
      await deleteInvitation({ data: { id } })
      toast.success('Invitation deleted')
      router.invalidate()
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to delete invitation')
    }
  }

  const handleCopyLink = () => {
    if (generatedLink) {
      navigator.clipboard.writeText(generatedLink)
      toast.success('Link copied to clipboard!')
    }
  }

  const handleCloseDialog = () => {
    setIsDialogOpen(false)
    setEmail('')
    setGeneratedLink(null)
  }

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>User Management</CardTitle>
              <CardDescription>Manage users and send invitations</CardDescription>
            </div>
            <Dialog onOpenChange={setIsDialogOpen} open={isDialogOpen}>
              <DialogTrigger asChild>
                <Button size="sm">Invite User</Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Invite a New User</DialogTitle>
                  <DialogDescription>Send an invitation link to allow someone to create an account.</DialogDescription>
                </DialogHeader>

                {generatedLink ? (
                  <div className="space-y-4">
                    <div className="rounded-lg bg-muted p-4">
                      <Label className="font-medium text-sm">Invitation Link</Label>
                      <div className="mt-2 flex gap-2">
                        <Input className="font-mono text-xs" readOnly value={generatedLink} />
                        <Button onClick={handleCopyLink} size="sm" variant="outline">
                          Copy
                        </Button>
                      </div>
                    </div>
                    <p className="text-muted-foreground text-sm">
                      This link will expire in 7 days. Share it securely with the invited user.
                    </p>
                    <DialogFooter>
                      <Button onClick={handleCloseDialog}>Done</Button>
                    </DialogFooter>
                  </div>
                ) : (
                  <>
                    <div className="space-y-4 py-4">
                      <div className="space-y-2">
                        <Label htmlFor="email">Email Address</Label>
                        <Input
                          id="email"
                          onChange={(e) => setEmail(e.target.value)}
                          placeholder="user@example.com"
                          type="email"
                          value={email}
                        />
                      </div>
                    </div>
                    <DialogFooter>
                      <Button onClick={handleCloseDialog} variant="outline">
                        Cancel
                      </Button>
                      <Button disabled={isCreating} onClick={handleCreateInvitation}>
                        {isCreating ? 'Creating...' : 'Create Invitation'}
                      </Button>
                    </DialogFooter>
                  </>
                )}
              </DialogContent>
            </Dialog>
          </div>
        </CardHeader>
        <CardContent>
          {loaderData.invitations.length === 0 ? (
            <div className="py-12 text-center text-muted-foreground">
              <svg
                className="mx-auto mb-4 h-12 w-12 text-muted-foreground/50"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <title>No invitations</title>
                <path
                  d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                />
              </svg>
              <p className="font-medium text-lg">No pending invitations</p>
              <p className="mt-1 text-sm">Click "Invite User" to send an invitation</p>
            </div>
          ) : (
            <div className="divide-y">
              {loaderData.invitations.map((invitation) => (
                <div className="flex items-center justify-between py-4" key={invitation.id}>
                  <div>
                    <p className="font-medium">{invitation.email}</p>
                    <p className="text-muted-foreground text-sm">
                      {invitation.isExpired ? (
                        <span className="text-destructive">Expired</span>
                      ) : (
                        <>Expires {new Date(invitation.expiresAt).toLocaleDateString()}</>
                      )}
                    </p>
                  </div>
                  <Button onClick={() => handleDeleteInvitation(invitation.id)} size="sm" variant="outline">
                    Delete
                  </Button>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <div className="mt-4 text-center text-muted-foreground text-sm">
        {loaderData.invitations.length} pending invitation{loaderData.invitations.length !== 1 ? 's' : ''}
      </div>
    </div>
  )
}
