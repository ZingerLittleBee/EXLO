import { createFileRoute, redirect } from '@tanstack/react-router'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { authorizeCode, getCodeStatus } from '@/functions/device-code'
import { getUser } from '@/functions/get-user'

export const Route = createFileRoute('/activate')({
  component: ActivatePage,
  validateSearch: (search: Record<string, unknown>) => ({
    code: (search.code as string) || ''
  }),
  beforeLoad: async () => {
    const session = await getUser()
    return { session }
  },
  loaderDeps: ({ search }) => ({ code: search.code }),
  loader: async ({ context, deps }) => {
    // If not logged in, redirect to login with the activation code preserved
    if (!context.session) {
      const code = deps.code
      const redirectTo = code ? `/activate?code=${encodeURIComponent(code)}` : '/activate'
      throw redirect({
        to: '/login',
        search: { redirectTo }
      })
    }

    // Get code status if code provided
    const code = deps.code
    let codeStatus: { status?: string; error?: string } | null = null

    if (code) {
      try {
        codeStatus = await getCodeStatus({ data: { code } })
      } catch {
        codeStatus = { error: 'Failed to check code' }
      }
    }

    return { code, codeStatus }
  }
})

function ActivatePage() {
  const context = Route.useRouteContext()
  const loaderData = Route.useLoaderData()
  const session = 'session' in context ? context.session : null
  const code = loaderData?.code || ''
  const codeStatus = loaderData?.codeStatus

  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState(false)

  const handleAuthorize = async () => {
    if (!code) return

    setIsLoading(true)
    setError(null)

    try {
      await authorizeCode({ data: { code } })
      setSuccess(true)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to authorize')
    } finally {
      setIsLoading(false)
    }
  }

  // No code provided
  if (!code) {
    return (
      <div className="flex min-h-screen items-center justify-center p-4">
        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle>Device Activation</CardTitle>
            <CardDescription>No activation code provided.</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground">Please use the link provided by your SSH client.</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  // Code not found or expired
  if (codeStatus?.status === 'not_found' || codeStatus?.status === 'expired') {
    return (
      <div className="flex min-h-screen items-center justify-center p-4">
        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle>Invalid Code</CardTitle>
            <CardDescription>
              This activation code {codeStatus.status === 'expired' ? 'has expired' : 'was not found'}.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground">Please reconnect your SSH client to get a new code.</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  // Already verified
  if (codeStatus?.status === 'verified' || success) {
    return (
      <div className="flex min-h-screen items-center justify-center p-4">
        <Card className="w-full max-w-md">
          <CardHeader>
            <CardTitle className="text-green-600">✓ Authorized!</CardTitle>
            <CardDescription>Your SSH session has been authorized.</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground">You can close this window. Your tunnel is now active.</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  // Pending - show authorize button
  return (
    <div className="flex min-h-screen items-center justify-center p-4">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>Authorize SSH Session</CardTitle>
          <CardDescription>
            Logged in as <strong>{session && 'user' in session ? session.user?.email : 'Unknown'}</strong>
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="rounded-lg bg-muted p-4 text-center font-mono text-2xl">{code}</div>

          {error && <p className="text-destructive text-sm">{error}</p>}

          <Button className="w-full" disabled={isLoading} onClick={handleAuthorize} size="lg">
            {isLoading ? 'Authorizing...' : 'Authorize SSH Session'}
          </Button>

          <p className="text-center text-muted-foreground text-xs">
            This will allow the SSH client to create tunnels on your account.
          </p>
        </CardContent>
      </Card>
    </div>
  )
}
