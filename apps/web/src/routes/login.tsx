import { useForm } from '@tanstack/react-form'
import { createFileRoute, useNavigate, useSearch } from '@tanstack/react-router'
import { toast } from 'sonner'
import z from 'zod'
import Loader from '@/components/loader'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { authClient } from '@/lib/auth-client'

export const Route = createFileRoute('/login')({
  component: LoginPage,
  validateSearch: (search: Record<string, unknown>): { redirectTo?: string } => ({
    redirectTo: (search.redirectTo as string) || undefined
  })
})

function LoginPage() {
  const navigate = useNavigate()
  const { redirectTo } = useSearch({ from: '/login' })
  const { isPending } = authClient.useSession()

  const form = useForm({
    defaultValues: {
      email: '',
      password: ''
    },
    onSubmit: async ({ value }) => {
      await authClient.signIn.email(
        {
          email: value.email,
          password: value.password
        },
        {
          onSuccess: () => {
            toast.success('Sign in successful')
            // Redirect to the original destination or dashboard
            if (redirectTo) {
              navigate({ to: redirectTo })
            } else {
              navigate({ to: '/dashboard' })
            }
          },
          onError: (error) => {
            toast.error(error.error.message || error.error.statusText)
          }
        }
      )
    },
    validators: {
      onSubmit: z.object({
        email: z.string().email('Invalid email address'),
        password: z.string().min(8, 'Password must be at least 8 characters')
      })
    }
  })

  if (isPending) {
    return <Loader />
  }

  return (
    <div className="flex min-h-screen items-center justify-center p-4">
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl">Welcome Back</CardTitle>
          <CardDescription>Sign in to access your tunnel server</CardDescription>
        </CardHeader>
        <CardContent>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              e.stopPropagation()
              form.handleSubmit()
            }}
            className="space-y-4"
          >
            <form.Field name="email">
              {(field) => (
                <div className="space-y-2">
                  <Label htmlFor={field.name}>Email</Label>
                  <Input
                    id={field.name}
                    name={field.name}
                    type="email"
                    placeholder="you@example.com"
                    value={field.state.value}
                    onBlur={field.handleBlur}
                    onChange={(e) => field.handleChange(e.target.value)}
                  />
                  {field.state.meta.errors.map((error) => (
                    <p key={error?.message} className="text-destructive text-sm">
                      {error?.message}
                    </p>
                  ))}
                </div>
              )}
            </form.Field>

            <form.Field name="password">
              {(field) => (
                <div className="space-y-2">
                  <Label htmlFor={field.name}>Password</Label>
                  <Input
                    id={field.name}
                    name={field.name}
                    type="password"
                    placeholder="Your password"
                    value={field.state.value}
                    onBlur={field.handleBlur}
                    onChange={(e) => field.handleChange(e.target.value)}
                  />
                  {field.state.meta.errors.map((error) => (
                    <p key={error?.message} className="text-destructive text-sm">
                      {error?.message}
                    </p>
                  ))}
                </div>
              )}
            </form.Field>

            <form.Subscribe>
              {(state) => (
                <Button type="submit" className="w-full" disabled={!state.canSubmit || state.isSubmitting}>
                  {state.isSubmitting ? 'Signing in...' : 'Sign In'}
                </Button>
              )}
            </form.Subscribe>
          </form>

          <div className="mt-6 rounded-lg bg-muted p-4 text-center">
            <p className="text-muted-foreground text-sm">
              <strong>Private Instance</strong>
            </p>
            <p className="mt-1 text-muted-foreground text-xs">
              This is a private tunnel server. Contact your administrator for access.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
