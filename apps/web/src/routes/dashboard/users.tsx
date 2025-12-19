import { createFileRoute, redirect, useRouter } from '@tanstack/react-router'
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

export const Route = createFileRoute('/dashboard/users')({
  component: UsersPage,
  beforeLoad: async () => {
    const session = await getUser()
    if (!session) {
      throw redirect({ to: '/login', search: {} })
    }
    return { session }
  },
  loader: async () => {
    const invitations = await listInvitations()
    return { invitations }
  }
})

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
    <div className="container mx-auto max-w-4xl py-8">
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="font-bold text-3xl">User Management</h1>
          <p className="text-muted-foreground">Manage users and send invitations</p>
        </div>

        <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
          <DialogTrigger asChild>
            <Button>Invite User</Button>
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
                    <Input readOnly value={generatedLink} className="font-mono text-xs" />
                    <Button variant="outline" size="sm" onClick={handleCopyLink}>
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
                      type="email"
                      placeholder="user@example.com"
                      value={email}
                      onChange={(e) => setEmail(e.target.value)}
                    />
                  </div>
                </div>
                <DialogFooter>
                  <Button variant="outline" onClick={handleCloseDialog}>
                    Cancel
                  </Button>
                  <Button onClick={handleCreateInvitation} disabled={isCreating}>
                    {isCreating ? 'Creating...' : 'Create Invitation'}
                  </Button>
                </DialogFooter>
              </>
            )}
          </DialogContent>
        </Dialog>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Pending Invitations</CardTitle>
          <CardDescription>Users who have been invited but haven't yet created their account.</CardDescription>
        </CardHeader>
        <CardContent>
          {loaderData.invitations.length === 0 ? (
            <p className="py-8 text-center text-muted-foreground">No pending invitations</p>
          ) : (
            <div className="divide-y">
              {loaderData.invitations.map((invitation) => (
                <div key={invitation.id} className="flex items-center justify-between py-4">
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
                  <Button variant="outline" size="sm" onClick={() => handleDeleteInvitation(invitation.id)}>
                    Delete
                  </Button>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
