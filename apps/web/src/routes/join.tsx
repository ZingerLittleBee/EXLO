import { useForm } from '@tanstack/react-form'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import z from 'zod'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { acceptInvitation, validateInvitation } from '@/functions/invitation'
import { authClient } from '@/lib/auth-client'

export const Route = createFileRoute('/join')({
  component: JoinPage,
  validateSearch: (search: Record<string, unknown>) => ({
    token: (search.token as string) || ''
  }),
  loaderDeps: ({ search }) => ({ token: search.token }),
  loader: async ({ deps }) => {
    const { token } = deps

    if (!token) {
      return { valid: false, error: 'No invitation token provided', email: '' }
    }

    const result = await validateInvitation({ data: { token } })
    return {
      valid: result.valid,
      error: result.error,
      email: result.email || '',
      token
    }
  }
})

function JoinPage() {
  const loaderData = Route.useLoaderData()
  const navigate = useNavigate()

  const form = useForm({
    defaultValues: {
      name: '',
      password: '',
      confirmPassword: ''
    },
    onSubmit: async ({ value }) => {
      try {
        // Accept the invitation and create the account
        const result = await acceptInvitation({
          data: {
            token: loaderData.token || '',
            name: value.name,
            password: value.password
          }
        })

        // Sign in the newly created user
        await authClient.signIn.email(
          {
            email: result.email,
            password: value.password
          },
          {
            onSuccess: () => {
              toast.success('Account created successfully! Welcome aboard.')
              navigate({ to: '/dashboard' })
            },
            onError: (error) => {
              toast.error(`Account created but sign-in failed: ${error.error.message}`)
              navigate({ to: '/login', search: {} })
            }
          }
        )
      } catch (error) {
        toast.error(error instanceof Error ? error.message : 'Failed to create account')
      }
    },
    validators: {
      onSubmit: z
        .object({
          name: z.string().min(2, 'Name must be at least 2 characters'),
          password: z.string().min(8, 'Password must be at least 8 characters'),
          confirmPassword: z.string()
        })
        .refine((data) => data.password === data.confirmPassword, {
          message: 'Passwords do not match',
          path: ['confirmPassword']
        })
    }
  })

  // Invalid or no token
  if (!loaderData.valid) {
    return (
      <div className="flex min-h-screen items-center justify-center p-4">
        <Card className="w-full max-w-md">
          <CardHeader className="text-center">
            <CardTitle className="text-2xl text-destructive">Invalid Invitation</CardTitle>
            <CardDescription>{loaderData.error || 'This invitation link is invalid or has expired.'}</CardDescription>
          </CardHeader>
          <CardContent className="text-center">
            <p className="text-muted-foreground text-sm">Please contact your administrator for a new invitation.</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="flex min-h-screen items-center justify-center p-4">
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl">Accept Invitation</CardTitle>
          <CardDescription>
            You've been invited to join this tunnel server. Complete your account setup below.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="mb-6 rounded-lg bg-muted p-3 text-center">
            <p className="text-muted-foreground text-sm">Creating account for</p>
            <p className="font-medium">{loaderData.email}</p>
          </div>

          <form
            className="space-y-4"
            onSubmit={(e) => {
              e.preventDefault()
              e.stopPropagation()
              form.handleSubmit()
            }}
          >
            <form.Field name="name">
              {(field) => (
                <div className="space-y-2">
                  <Label htmlFor={field.name}>Name</Label>
                  <Input
                    id={field.name}
                    name={field.name}
                    onBlur={field.handleBlur}
                    onChange={(e) => field.handleChange(e.target.value)}
                    placeholder="Your name"
                    value={field.state.value}
                  />
                  {field.state.meta.errors.map((error) => (
                    <p className="text-destructive text-sm" key={error?.message}>
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
                    onBlur={field.handleBlur}
                    onChange={(e) => field.handleChange(e.target.value)}
                    placeholder="Min. 8 characters"
                    type="password"
                    value={field.state.value}
                  />
                  {field.state.meta.errors.map((error) => (
                    <p className="text-destructive text-sm" key={error?.message}>
                      {error?.message}
                    </p>
                  ))}
                </div>
              )}
            </form.Field>

            <form.Field name="confirmPassword">
              {(field) => (
                <div className="space-y-2">
                  <Label htmlFor={field.name}>Confirm Password</Label>
                  <Input
                    id={field.name}
                    name={field.name}
                    onBlur={field.handleBlur}
                    onChange={(e) => field.handleChange(e.target.value)}
                    placeholder="Confirm your password"
                    type="password"
                    value={field.state.value}
                  />
                  {field.state.meta.errors.map((error) => (
                    <p className="text-destructive text-sm" key={error?.message}>
                      {error?.message}
                    </p>
                  ))}
                </div>
              )}
            </form.Field>

            <form.Subscribe>
              {(state) => (
                <Button className="w-full" disabled={!state.canSubmit || state.isSubmitting} type="submit">
                  {state.isSubmitting ? 'Creating Account...' : 'Create Account'}
                </Button>
              )}
            </form.Subscribe>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
